#![allow(clippy::items_after_test_module)]

//! 文件模型与文件操作。
//!
//! 本模块包含两个主要部分：
//!
//! # 1. [`FileModel`] — 核心文件处理模型
//!
//! 提供文件加载、内存映射、行扫描和数据读取的底层实现。
//! 通常通过高层 [`Fjiffyldg`](crate::Fjiffyldg) 接口使用，而非直接操作。
//!
//! # 2. 独立文件操作函数
//!
//! | 函数 | 用途 | 大文件优化 |
//! |------|------|-----------|
//! | [`clone_file`] | 克隆文件 | >10MB 走 mmap |
//! | [`save_file`] | 保存数据 | >10MB 走可写 mmap |
//! | [`append_file`] | 追加数据 | >10MB 走缓冲写入 |
//! | [`concatenate_files`] | 合并多个文件 | >10MB 走 mmap |
//! | [`get_file_size`] | 获取文件大小 | — |
//!
//! 所有写入操作均包含写后大小校验，不匹配时返回 [`IncompleteWrite`](crate::FjiffyldgError::IncompleteWrite)。
//!
//! # 示例
//!
//! ```no_run
//! use fjiffyldg::{save_file, append_file, clone_file, get_file_size};
//!
//! save_file("/tmp/demo.txt", b"Hello")?;
//! append_file("/tmp/demo.txt", b" World")?;
//! assert_eq!(get_file_size("/tmp/demo.txt")?, 11);
//!
//! clone_file("/tmp/demo.txt", "/tmp/demo_copy.txt")?;
//! # fjiffyldg::Result::Ok(())
//! ```

use crate::encoding::{detect_encoding, TextEncoding};
use crate::error::{FjiffyldgError, Result, UtfMode};
use crate::line_index::LineIndex;
use memmap2::{Mmap, MmapMut, MmapOptions};
use parking_lot::{Condvar, Mutex, RwLock};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, ErrorKind, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const KB: usize = 1024;
const MB: usize = 1024 * KB;
/// 文件≤10MB时使用内存缓冲，>10MB时使用内存映射
const USUALLY_IO_SIZE_MAX: u64 = 10 * MB as u64;
/// 单次读取缓冲区大小：128KB
const BUFFER_SIZE: usize = 128 * KB;
/// 超长行临界值：4KB
const CRITICAL_LONGLINE_LEN: usize = 4 * KB;
/// 大文件写入时使用的缓冲区大小：8MB
const LARGE_WRITE_BUFFER_SIZE: usize = 8 * MB;
/// 大文件映射分块大小：1GB
///
/// 32 位平台虚拟地址空间有限（~3GB），必须分块映射；
/// 64 位平台地址空间充裕（128TB），此值用于初始窗口大小。
pub(crate) const MMAP_CHUNK_SIZE: u64 = 1024 * MB as u64;

/// 后台扫描状态与完成通知
struct ScanState {
    /// 后台扫描是否仍在进行
    scanning: AtomicBool,
    /// 条件变量配套互斥锁，防止完成通知丢失
    lock: Mutex<()>,
    /// 扫描完成通知
    completed: Condvar,
}

impl ScanState {
    /// 创建新的扫描状态
    fn new() -> Self {
        Self {
            scanning: AtomicBool::new(false),
            lock: Mutex::new(()),
            completed: Condvar::new(),
        }
    }

    /// 标记扫描开始
    fn begin(&self) {
        let _guard = self.lock.lock();
        self.scanning.store(true, Ordering::Release);
    }

    /// 标记扫描完成并唤醒等待方
    fn finish(&self) {
        let _guard = self.lock.lock();
        self.scanning.store(false, Ordering::Release);
        self.completed.notify_all();
    }

    /// 检查扫描是否仍在进行
    fn is_scanning(&self) -> bool {
        self.scanning.load(Ordering::Acquire)
    }

    /// 等待扫描完成
    fn wait_complete(&self) {
        let mut guard = self.lock.lock();
        while self.is_scanning() {
            self.completed.wait(&mut guard);
        }
    }
}

/// 扫描任务完成守卫
///
/// 后台任务退出时自动发送完成通知，避免扫描过程中 panic 导致等待方永久阻塞。
struct ScanCompletionGuard {
    /// 需要通知的扫描状态
    scan_state: Arc<ScanState>,
}

impl ScanCompletionGuard {
    /// 创建新的扫描完成守卫
    fn new(scan_state: Arc<ScanState>) -> Self {
        Self { scan_state }
    }
}

impl Drop for ScanCompletionGuard {
    fn drop(&mut self) {
        self.scan_state.finish();
    }
}

/// 后台扫描共享的原始字节缓冲。
#[allow(dead_code)]
#[derive(Clone)]
enum ScanBuffer {
    /// 小文件内存缓冲
    Owned(Arc<[u8]>),
    /// 大文件内存映射
    Mapped(Arc<Mmap>),
}

impl ScanBuffer {
    /// 返回完整字节切片。
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(data) => data,
            Self::Mapped(mmap) => mmap,
        }
    }

    /// 返回从指定偏移开始的字节切片。
    fn slice_from(&self, offset: u64) -> &[u8] {
        self.as_slice().get(offset as usize..).unwrap_or_default()
    }
}

/// 高性能文件模型
///
/// 特性：
/// - 支持1B~1TB的文件
/// - 小文件直接加载，大文件智能映射
/// - 后台异步扫描行结构
/// - 原生UTF-16/32支持（无转换开销）
/// - 分级索引管理超大行数（>100万行）
///
/// # 示例
/// ```ignore
/// let file_model = FileModel::new();
/// file_model.load_and_scan_file("large_file.txt")?;
///
/// println!("Total lines: {}", file_model.get_line_count());
/// let line_pos = file_model.get_line_pos(0);
/// let data = file_model.read_data(line_pos, 1024)?;
/// ```
pub struct FileModel {
    line_index: Arc<LineIndex>,

    /// 小文件直接内存缓冲
    data: RwLock<Option<Arc<[u8]>>>,
    /// 大文件内存映射（偏移 + mmap 合并到单锁，消除 TOCTOU 竞态）
    mmap_window: RwLock<Option<(u64, Arc<Mmap>)>>,
    /// 大文件分块映射窗口大小
    mmap_chunk_size: RwLock<u64>,

    /// 文件句柄（用于后续操作）
    file: RwLock<Option<File>>,
    /// 文件总大小（字节）
    file_size: RwLock<u64>,
    /// UTF编码模式
    utf_mode: RwLock<UtfMode>,
    /// 错误码
    error_code: RwLock<i32>,
    /// 文件加载完成标志
    is_loaded: RwLock<bool>,
    /// 后台扫描状态
    scan_state: Arc<ScanState>,
    /// 后台扫描取消请求标记
    cancel_requested: Arc<AtomicBool>,
    /// 加载操作互斥锁（防止并发 load_file 导致状态损坏）
    load_mutex: Mutex<()>,
}

impl FileModel {
    /// 创建新的文件模型实例
    pub fn new() -> Self {
        Self {
            line_index: Arc::new(LineIndex::new()),
            data: RwLock::new(None),
            mmap_window: RwLock::new(None),
            mmap_chunk_size: RwLock::new(MMAP_CHUNK_SIZE),
            file: RwLock::new(None),
            file_size: RwLock::new(0),
            utf_mode: RwLock::new(UtfMode::Default),
            error_code: RwLock::new(0),
            is_loaded: RwLock::new(false),
            scan_state: Arc::new(ScanState::new()),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            load_mutex: Mutex::new(()),
        }
    }

    /// 获取最后的错误码
    ///
    /// # 错误码说明
    /// - `0`：无错误
    /// - `-1`：文件不存在
    /// - `1`：文件无法访问
    /// - `2`：流读取错误
    /// - `3`：内存映射失败
    pub fn get_error_code(&self) -> i32 {
        *self.error_code.read()
    }

    /// 检查文件是否已加载
    pub fn is_loaded(&self) -> bool {
        *self.is_loaded.read()
    }

    /// 获取文件加载状态
    ///
    /// 返回 `Ok(true)` 表示已加载，`Ok(false)` 表示尚未加载且没有错误，
    /// 返回 `Err` 表示最近一次加载或文件操作失败。
    pub fn get_load_status(&self) -> Result<bool> {
        if self.is_loaded() {
            return Ok(true);
        }

        let code = self.get_error_code();
        if code == 0 {
            Ok(false)
        } else {
            Err(FjiffyldgError::from_error_code(code).unwrap_or(FjiffyldgError::IoError))
        }
    }

    /// 检查行扫描是否完成
    pub fn is_scanning(&self) -> bool {
        self.scan_state.is_scanning()
    }

    /// 获取文件大小（字节）
    pub fn get_file_size(&self) -> i64 {
        *self.file_size.read() as i64
    }

    /// 获取当前UTF编码模式
    pub fn get_utf_mode(&self) -> UtfMode {
        *self.utf_mode.read()
    }

    /// 设置UTF编码模式
    pub fn set_utf_mode(&self, mode: UtfMode) {
        *self.utf_mode.write() = mode;
        self.line_index.set_utf_mode(mode);
    }

    /// 获取文件总行数
    ///
    /// # 返回值
    /// - 若文件已扫描完成：返回行数（≥1）
    /// - 若仍在后台扫描中：返回 -1
    /// - 若文件未加载：返回 -1
    pub fn get_line_count(&self) -> i64 {
        if !self.is_loaded() {
            return -1;
        }
        // 扫描进行中时返回 -1（对齐 C++ linescanRunning 三态逻辑）
        if self.is_scanning() {
            return -1;
        }
        self.line_index.get_line_count()
    }

    /// 获取指定行的起始字节位置（0-based）
    ///
    /// # 参数
    /// - `index`：行索引（0-based）
    ///
    /// # 返回值
    /// - 成功：字节位置
    /// - 失败：-1
    pub fn get_line_pos(&self, index: i64) -> i64 {
        self.line_index.get_line_pos(index as usize)
    }

    /// 获取指定行的内容长度（字节数，不含行尾符）
    pub fn get_line_length(&self, index: i64) -> i64 {
        self.line_index.get_line_length(index as usize)
    }

    /// 根据字节位置查找所在行的索引
    pub fn get_line_by_pos(&self, pos: i64) -> i64 {
        if !self.is_loaded() {
            return -1;
        }

        if pos > *self.file_size.read() as i64 {
            return -1;
        }

        self.line_index.get_line_by_pos(pos)
    }

    /// 加载文件并异步扫描行结构
    ///
    /// 立即返回，行扫描在后台进行。
    /// 可通过 `is_scanning()` 检查扫描进度，或通过 `wait_scan_complete()` 等待完成。
    pub fn load_and_scan_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.load_file(path, true)
    }

    /// 仅加载文件，不扫描行结构
    ///
    /// 用于只需读取数据不需要行操作的场景。
    pub fn load_file_only<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.load_file(path, false)
    }

    fn load_file<P: AsRef<Path>>(&self, path: P, enable_scan: bool) -> Result<()> {
        self.load_file_with_mmap_chunk_size(path, enable_scan, MMAP_CHUNK_SIZE)
    }

    /// 使用指定 mmap 窗口大小加载文件。
    fn load_file_with_mmap_chunk_size<P: AsRef<Path>>(
        &self,
        path: P,
        enable_scan: bool,
        mmap_chunk_size: u64,
    ) -> Result<()> {
        let _load_guard = self.load_mutex.lock();
        let path = path.as_ref();
        self.request_stop_scan();
        // request_stop_scan 内部已调用 line_index.clear()，无需重复调用
        *self.data.write() = None;
        *self.mmap_window.write() = None;
        *self.file.write() = None;
        *self.file_size.write() = 0;
        *self.is_loaded.write() = false;

        let file = match File::open(path) {
            Ok(f) => f,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                *self.error_code.write() = FjiffyldgError::FileNotLoaded.to_error_code();
                return Err(FjiffyldgError::FileNotLoaded);
            }
            Err(_) => {
                *self.error_code.write() = FjiffyldgError::FileInaccessible.to_error_code();
                return Err(FjiffyldgError::FileInaccessible);
            }
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => {
                *self.error_code.write() = FjiffyldgError::FileInaccessible.to_error_code();
                return Err(FjiffyldgError::FileInaccessible);
            }
        };

        let file_size = metadata.len();

        // 拒绝目录路径（Windows 上 File::open 对目录成功，metadata.len() 返回 0）
        if !metadata.is_file() {
            *self.error_code.write() = FjiffyldgError::FileInaccessible.to_error_code();
            return Err(FjiffyldgError::FileInaccessible);
        }

        *self.file_size.write() = file_size;
        *self.mmap_chunk_size.write() = mmap_chunk_size.max(1);
        *self.file.write() = Some(file);

        if file_size == 0 {
            *self.is_loaded.write() = true;
            *self.error_code.write() = 0;
            if enable_scan {
                self.line_index.mark_empty_file();
            } else {
                self.line_index.mark_scanned();
            }
            return Ok(());
        }

        // 加载文件数据
        if file_size <= USUALLY_IO_SIZE_MAX {
            // 先在独立作用域内 try_clone，确保读锁在错误处理前释放，
            // 避免 ok_or_else 中 write() 与 read() 同锁死锁。
            let mut f = {
                let guard = self.file.read();
                guard.as_ref().and_then(|f| f.try_clone().ok())
            }
            .ok_or_else(|| {
                *self.file_size.write() = 0;
                *self.file.write() = None;
                *self.error_code.write() = FjiffyldgError::StreamError.to_error_code();
                FjiffyldgError::StreamError
            })?;

            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|_| {
                // 回滚已提交的 file_size 和 file handle
                *self.file_size.write() = 0;
                *self.file.write() = None;
                *self.error_code.write() = FjiffyldgError::StreamError.to_error_code();
                FjiffyldgError::StreamError
            })?;
            *self.data.write() = Some(Arc::<[u8]>::from(buffer));
        } else {
            let window_len = file_size.min(*self.mmap_chunk_size.read());
            match self.map_file_window(0, window_len) {
                Ok(mmap) => {
                    *self.mmap_window.write() = Some((0, Arc::new(mmap)));
                }
                Err(err) => {
                    // 回滚已提交的 file_size 和 file handle
                    *self.file_size.write() = 0;
                    *self.file.write() = None;
                    *self.error_code.write() = err.to_error_code();
                    return Err(err);
                }
            }
        }

        *self.is_loaded.write() = true;
        *self.error_code.write() = 0;

        if enable_scan {
            self.scan_lines_background();
        } else {
            self.line_index.mark_scanned();
        }

        Ok(())
    }

    /// 后台扫描行结构
    fn scan_lines_background(&self) {
        self.scan_lines_background_from(0, None, true);
    }

    /// 从指定偏移开始后台扫描行结构
    fn scan_lines_background_from(
        &self,
        offset: u64,
        forced_utf_mode: Option<UtfMode>,
        auto_detect: bool,
    ) {
        let scan_state = Arc::clone(&self.scan_state);
        let cancel_requested = Arc::clone(&self.cancel_requested);
        let line_index = Arc::clone(&self.line_index);
        let current_utf_mode = self.get_utf_mode();

        if let Some(scan_buffer) = self
            .data
            .read()
            .as_ref()
            .map(|data| ScanBuffer::Owned(Arc::clone(data)))
        {
            let file_utf_mode = Self::detect_utf_mode(scan_buffer.as_slice());
            cancel_requested.store(false, Ordering::Release);
            scan_state.begin();

            rayon::spawn(move || {
                let _completion_guard = ScanCompletionGuard::new(scan_state);
                let scan_data = scan_buffer.slice_from(offset);
                let utf_mode = Self::resolve_scan_utf_mode(
                    forced_utf_mode,
                    auto_detect,
                    file_utf_mode,
                    current_utf_mode,
                );

                let _ = line_index.build_from_data_at_cancelable(
                    scan_data,
                    offset,
                    utf_mode,
                    &cancel_requested,
                );
            });
            return;
        }

        let Some((_, first_mmap)) = self
            .mmap_window
            .read()
            .as_ref()
            .map(|(o, m)| (*o, Arc::clone(m)))
        else {
            self.line_index.mark_scanned();
            return;
        };
        let Some(file) = self
            .file
            .read()
            .as_ref()
            .and_then(|file| file.try_clone().ok())
        else {
            self.line_index.mark_scanned();
            return;
        };
        let file_size = *self.file_size.read();
        let mmap_chunk_size = (*self.mmap_chunk_size.read()).max(1);
        let file_utf_mode = Self::detect_utf_mode(&first_mmap);

        cancel_requested.store(false, Ordering::Release);
        scan_state.begin();

        rayon::spawn(move || {
            let _completion_guard = ScanCompletionGuard::new(scan_state);
            let utf_mode = Self::resolve_scan_utf_mode(
                forced_utf_mode,
                auto_detect,
                file_utf_mode,
                current_utf_mode,
            );

            let _ = line_index.build_from_windows_at_cancelable(
                offset,
                file_size,
                mmap_chunk_size,
                utf_mode,
                &cancel_requested,
                |window_offset, window_len| {
                    let aligned_offset = (window_offset / mmap_chunk_size) * mmap_chunk_size;
                    let delta = window_offset - aligned_offset;
                    let requested_end = window_offset.saturating_add(window_len).min(file_size);
                    let map_end = aligned_offset
                        .saturating_add(mmap_chunk_size)
                        .max(requested_end)
                        .min(file_size);
                    let map_len = usize::try_from(map_end.saturating_sub(aligned_offset)).ok()?;
                    let begin = usize::try_from(delta).ok()?;
                    let end = begin.checked_add(usize::try_from(window_len).ok()?)?;
                    let mmap = unsafe {
                        MmapOptions::new()
                            .offset(aligned_offset)
                            .len(map_len)
                            .map(&file)
                            .ok()?
                    };
                    Some(mmap.get(begin..end)?.to_vec())
                },
            );
        });
    }

    /// 解析本次行扫描使用的 UTF 模式。
    fn resolve_scan_utf_mode(
        forced_utf_mode: Option<UtfMode>,
        auto_detect: bool,
        file_utf_mode: UtfMode,
        current_utf_mode: UtfMode,
    ) -> UtfMode {
        match forced_utf_mode {
            Some(UtfMode::Default) if auto_detect => file_utf_mode,
            Some(UtfMode::AutoDetect) => file_utf_mode,
            Some(mode) => mode,
            None if current_utf_mode != UtfMode::Default => current_utf_mode,
            None if auto_detect => file_utf_mode,
            None => UtfMode::Default,
        }
    }

    /// 根据 BOM 检测行扫描使用的 UTF 模式
    fn detect_utf_mode(data: &[u8]) -> UtfMode {
        match detect_encoding(data) {
            TextEncoding::Utf16Le => UtfMode::Utf16Le,
            TextEncoding::Utf16Be => UtfMode::Utf16Be,
            TextEncoding::Utf32Le => UtfMode::Utf32Le,
            TextEncoding::Utf32Be => UtfMode::Utf32Be,
            TextEncoding::Ascii | TextEncoding::Utf8 | TextEncoding::Unknown => UtfMode::Default,
        }
    }

    /// 重新扫描已加载文件的行结构
    pub fn restart_scan(&self, offset: u64, utf_mode: UtfMode) -> Result<()> {
        self.restart_scan_with_auto_detect(offset, utf_mode, true)
    }

    /// 重新扫描已加载文件的行结构，并指定默认模式是否按 BOM 自动检测
    pub(crate) fn restart_scan_with_auto_detect(
        &self,
        offset: u64,
        utf_mode: UtfMode,
        auto_detect: bool,
    ) -> Result<()> {
        let _guard = self.load_mutex.lock();
        if !self.is_loaded() {
            *self.error_code.write() = FjiffyldgError::FileNotLoaded.to_error_code();
            return Err(FjiffyldgError::FileNotLoaded);
        }

        let file_size = *self.file_size.read();
        if offset > file_size {
            *self.error_code.write() = FjiffyldgError::InvalidOffset.to_error_code();
            return Err(FjiffyldgError::InvalidOffset);
        }

        self.request_stop_scan();
        self.line_index.clear();
        // 解析 AutoDetect 为实际编码后再存储，使 get_utf_mode() 返回生效的编码
        let resolved_mode = if utf_mode == UtfMode::AutoDetect || auto_detect {
            let file_mode = self
                .data
                .read()
                .as_ref()
                .map(|d| Self::detect_utf_mode(d))
                .or_else(|| {
                    self.mmap_window
                        .read()
                        .as_ref()
                        .map(|(_, m)| Self::detect_utf_mode(m))
                })
                .unwrap_or(UtfMode::Default);
            Self::resolve_scan_utf_mode(Some(utf_mode), auto_detect, file_mode, self.get_utf_mode())
        } else {
            utf_mode
        };
        self.set_utf_mode(resolved_mode);
        self.scan_lines_background_from(offset, Some(resolved_mode), false);
        *self.error_code.write() = 0;
        Ok(())
    }

    /// 请求停止后台扫描并等待其退出
    pub fn request_stop_scan(&self) {
        self.cancel_requested.store(true, Ordering::Release);
        self.wait_scan_complete();
        self.cancel_requested.store(false, Ordering::Release);
        self.line_index.clear();
        self.line_index.mark_scanned();
    }

    /// 等待行扫描完成
    ///
    /// 如果后台扫描仍在进行，阻塞至完成。
    pub fn wait_scan_complete(&self) {
        self.scan_state.wait_complete();
    }

    /// 获取原始文件数据
    #[allow(dead_code)]
    fn get_scan_buffer(&self) -> Option<ScanBuffer> {
        if let Some(data) = self.data.read().as_ref() {
            return Some(ScanBuffer::Owned(Arc::clone(data)));
        }

        if let Some((_, mmap)) = self.mmap_window.read().as_ref() {
            return Some(ScanBuffer::Mapped(Arc::clone(mmap)));
        }

        None
    }

    /// 映射指定文件窗口。
    fn map_file_window(&self, offset: u64, len: u64) -> Result<Mmap> {
        let file_guard = self.file.read();
        let file = file_guard.as_ref().ok_or(FjiffyldgError::FileNotLoaded)?;
        let len = usize::try_from(len).map_err(|_| FjiffyldgError::MmapError)?;

        unsafe {
            MmapOptions::new()
                .offset(offset)
                .len(len)
                .map(file)
                .map_err(|_| FjiffyldgError::MmapError)
        }
    }

    /// 确保当前 mmap 窗口覆盖指定读取范围。
    ///
    /// 返回 `(窗口偏移, Arc<Mmap>)` 元组，确保调用方获得一致的窗口视图。
    fn ensure_mmap_window(&self, pos: u64, len: u64) -> Result<(u64, Arc<Mmap>)> {
        {
            let window = self.mmap_window.read();
            if let Some((offset, mmap)) = window.as_ref() {
                let mapped_end = offset + mmap.len() as u64;
                if pos >= *offset && pos.saturating_add(len) <= mapped_end {
                    return Ok((*offset, Arc::clone(mmap)));
                }
            }
        }

        let file_size = *self.file_size.read();
        let chunk_size = (*self.mmap_chunk_size.read()).max(1);
        let aligned_offset = (pos / chunk_size) * chunk_size;
        let requested_end = pos.saturating_add(len).min(file_size);
        let window_end = aligned_offset
            .saturating_add(chunk_size)
            .max(requested_end)
            .min(file_size);
        let window_len = window_end.saturating_sub(aligned_offset);
        let mmap = Arc::new(self.map_file_window(aligned_offset, window_len)?);

        *self.mmap_window.write() = Some((aligned_offset, Arc::clone(&mmap)));
        Ok((aligned_offset, mmap))
    }

    /// 标记扫描完成（内部调用）
    pub fn mark_scanned(&self) {
        self.line_index.mark_scanned();
    }

    /// 从指定位置读取数据
    ///
    /// # 参数
    /// - `pos`：起始字节位置
    /// - `len`：读取长度。若为0，使用默认缓冲区大小（128KB）
    ///
    /// # 返回值
    /// 读取的字节数据，可能少于请求长度（接近文件末尾时）
    pub fn read_data(&self, pos: i64, mut len: usize) -> Option<Vec<u8>> {
        let file_size = *self.file_size.read() as i64;

        if !self.is_loaded() {
            return None;
        }

        let pos = pos.max(0);

        if len == 0 {
            len = BUFFER_SIZE;
        }

        let end_pos = (pos as usize).min(file_size as usize);
        let remaining = file_size as usize - end_pos;
        let actual_len = len.min(remaining);

        if actual_len == 0 {
            return Some(Vec::new());
        }

        if let Some(ref data) = *self.data.read() {
            return Some(data[end_pos..end_pos + actual_len].to_vec());
        }

        let (mmap_offset, mmap) = self
            .ensure_mmap_window(end_pos as u64, actual_len as u64)
            .ok()?;
        let window_pos = end_pos.checked_sub(mmap_offset as usize)?;
        Some(mmap[window_pos..window_pos + actual_len].to_vec())
    }

    /// 读取指定行的数据
    ///
    /// # 参数
    /// - `index`：行索引（0-based）
    /// - `bpos`：输出参数，返回行起始位置
    /// - `epos`：输出参数，返回行结束位置
    /// - `len`：输入最大读取长度，输出实际读取长度；若为 0，最多读取 4KB
    ///
    /// # 返回值
    /// 行数据，若失败返回None
    pub fn read_line(
        &self,
        index: i64,
        bpos: &mut i64,
        epos: &mut i64,
        len: &mut usize,
    ) -> Option<Vec<u8>> {
        let begin = self.line_index.get_line_pos(index as usize);
        if begin < 0 {
            *len = 0;
            return None;
        }

        *bpos = begin;

        // 使用行长度（不含行尾符）确定实际内容范围
        let line_len = self.line_index.get_line_length(index as usize);
        if line_len < 0 {
            // get_line_length 失败（未扫描/越界），与 get_line_pos 返回 -1 同等处理
            *len = 0;
            return None;
        }
        let full_len = line_len as usize;

        let actual_len = if *len == 0 {
            full_len.min(CRITICAL_LONGLINE_LEN)
        } else {
            (*len).min(full_len)
        };
        *epos = begin + actual_len as i64;
        *len = actual_len;

        if actual_len == 0 {
            Some(Vec::new())
        } else {
            self.read_data(*bpos, actual_len)
        }
    }

    /// 按 C++ `ReadFileDataLLineCut` 语义读取一段行数据
    ///
    /// 从 `index` 指向的行起始位置开始读取。短行会按行边界批量读取，`len`
    /// 用作批量预算而不是硬上限；如果遇到超过 4KB 的长行，则在 4KB 处截断。
    /// 返回时 `index` 会推进到最后一个完整纳入读取范围的行。
    pub fn read_line_cut(
        &self,
        index: &mut i64,
        bpos: &mut i64,
        epos: &mut i64,
        len: &mut usize,
    ) -> Option<Vec<u8>> {
        if *index < 0 {
            *len = 0;
            return None;
        }

        let begin = self.line_index.get_line_pos(*index as usize);
        if begin < 0 {
            *len = 0;
            return None;
        }

        *bpos = begin;

        let mut length = if *len == 0 { BUFFER_SIZE } else { *len };
        length = length.min(usize::MAX - 1 - CRITICAL_LONGLINE_LEN);

        let mut cur_pos = begin;
        let mut next_pos = self.line_index.get_line_pos((*index + 1) as usize);

        while next_pos > 0
            && (next_pos - begin) as usize <= length
            && (next_pos - cur_pos) as usize <= CRITICAL_LONGLINE_LEN
        {
            *index += 1;
            cur_pos = next_pos;
            next_pos = self.line_index.get_line_pos((*index + 1) as usize);
        }

        if next_pos < 0 {
            let file_size = *self.file_size.read() as i64;
            if file_size - cur_pos <= CRITICAL_LONGLINE_LEN as i64 {
                next_pos = file_size;
            }
        }

        if next_pos < 0 || next_pos - cur_pos > CRITICAL_LONGLINE_LEN as i64 {
            next_pos = cur_pos + CRITICAL_LONGLINE_LEN as i64;
            // 无条件推进 index：长行截断后跳过该行，避免下次调用重复读取。
            // 无论长行是首行、中间行还是末行，index 都推进到下一行。
            // 若是末行，index 超出总行数，下次 get_line_pos 返回 -1 终止迭代。
            *index += 1;
        }

        let actual_len = if next_pos > begin {
            (next_pos - begin) as usize
        } else {
            0
        };

        *epos = next_pos;
        *len = actual_len;

        if actual_len == 0 {
            Some(Vec::new())
        } else {
            self.read_data(begin, actual_len)
        }
    }

    /// 从指定位置读取到行尾
    ///
    /// # 参数
    /// - `index`：行索引
    /// - `pos`：行内起始位置
    /// - `len`：最大读取长度。若为0，默认最多读取 4KB
    pub fn read_to_end_of_line(&self, index: i64, pos: i64, len: &mut usize) -> Option<Vec<u8>> {
        let file_size = *self.file_size.read() as i64;
        let line_start = self.line_index.get_line_pos(index as usize);

        if line_start < 0 || pos < line_start || pos > file_size {
            *len = 0;
            return None;
        }

        // 计算行尾位置
        let mut line_end = file_size;
        if index + 1 != self.line_index.get_line_count() {
            let end_pos = self.line_index.get_line_pos((index + 1) as usize);
            if end_pos >= 0 && pos <= end_pos {
                line_end = end_pos;
            } else {
                *len = 0;
                return None;
            }
        }

        let mut actual_len = *len;
        if actual_len == 0 {
            actual_len = CRITICAL_LONGLINE_LEN;
        }

        if actual_len > (line_end - pos) as usize {
            actual_len = (line_end - pos) as usize;
        }

        *len = actual_len;
        if actual_len == 0 {
            Some(Vec::new())
        } else {
            self.read_data(pos, actual_len)
        }
    }

    /// 获取整个文件的内存映射（超大文件场景）
    ///
    /// 返回整个文件的只读映射。仅推荐用于需要随机访问全文的场景。
    pub fn get_huge_buffer<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|_| FjiffyldgError::FileInaccessible)?;

        // 拒绝目录路径
        if !file.metadata().map(|m| m.is_file()).unwrap_or(false) {
            return Err(FjiffyldgError::FileInaccessible);
        }

        match unsafe { Mmap::map(&file) } {
            Ok(mmap) => Ok(mmap.to_vec()),
            Err(_) => Err(FjiffyldgError::MmapError),
        }
    }

    /// 清空所有数据
    pub fn clear(&self) {
        let _guard = self.load_mutex.lock();
        self.request_stop_scan();
        *self.data.write() = None;
        *self.mmap_window.write() = None;
        *self.file.write() = None;
        self.line_index.clear();
        *self.file_size.write() = 0;
        *self.error_code.write() = 0;
        *self.is_loaded.write() = false;
    }
}

impl Default for FileModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::mpsc;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    #[test]
    fn test_scan_state_notifies_waiters_on_finish() {
        let state = Arc::new(ScanState::new());
        state.begin();

        let (sender, receiver) = mpsc::channel();
        for _ in 0..2 {
            let waiter = Arc::clone(&state);
            let sender = sender.clone();
            std::thread::spawn(move || {
                waiter.wait_complete();
                sender.send(()).unwrap();
            });
        }
        drop(sender);

        state.finish();

        receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(!state.is_scanning());
    }

    #[test]
    fn test_scan_buffer_reuses_small_file_storage() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"abcdef").unwrap();

        let model = FileModel::new();
        model.load_file_only(temp.path()).unwrap();

        let expected_ptr = model.data.read().as_ref().unwrap().as_ptr();
        let scan_buffer = model.get_scan_buffer().unwrap();
        let slice = scan_buffer.slice_from(2);

        assert_eq!(slice, b"cdef");
        assert_eq!(slice.as_ptr(), unsafe { expected_ptr.add(2) });
    }

    #[test]
    fn test_scan_buffer_reuses_mmap_storage() {
        let mut temp = NamedTempFile::new().unwrap();
        let data = vec![b'x'; (USUALLY_IO_SIZE_MAX as usize) + 1];
        temp.write_all(&data).unwrap();

        let model = FileModel::new();
        model.load_file_only(temp.path()).unwrap();

        let expected_ptr = model.mmap_window.read().as_ref().unwrap().1.as_ptr();
        let scan_buffer = model.get_scan_buffer().unwrap();
        let slice = scan_buffer.slice_from(3);

        assert_eq!(slice.len(), data.len() - 3);
        assert_eq!(slice.as_ptr(), unsafe { expected_ptr.add(3) });
    }

    #[test]
    fn test_read_data_remaps_window_for_far_mmap_offset() {
        let mut temp = NamedTempFile::new().unwrap();
        let mut data = vec![b'a'; (USUALLY_IO_SIZE_MAX as usize) + 64];
        let window_size = 4096u64;
        let target_pos = window_size as usize + 17;
        data[target_pos..target_pos + 5].copy_from_slice(b"hello");
        temp.write_all(&data).unwrap();

        let model = FileModel::new();
        model
            .load_file_with_mmap_chunk_size(temp.path(), false, window_size)
            .unwrap();

        assert_eq!(model.mmap_window.read().as_ref().unwrap().0, 0);
        assert_eq!(model.read_data(target_pos as i64, 5).unwrap(), b"hello");
        assert_eq!(model.mmap_window.read().as_ref().unwrap().0, window_size);
    }

    #[test]
    fn test_scan_uses_all_mmap_windows() {
        let mut temp = NamedTempFile::new().unwrap();
        let window_size = 4096u64;
        let mut data = vec![b'a'; (USUALLY_IO_SIZE_MAX as usize) + 64];
        data[8] = b'\n';
        data[window_size as usize + 17] = b'\n';
        temp.write_all(&data).unwrap();

        let model = FileModel::new();
        model
            .load_file_with_mmap_chunk_size(temp.path(), true, window_size)
            .unwrap();
        model.wait_scan_complete();

        assert_eq!(model.get_line_count(), 3);
        assert_eq!(model.get_line_pos(1), 9);
        assert_eq!(model.get_line_pos(2), window_size as i64 + 18);
    }

    #[test]
    fn test_windowed_scan_supports_unaligned_restart_offset() {
        let mut temp = NamedTempFile::new().unwrap();
        let window_size = 4096u64;
        let mut data = vec![b'a'; (USUALLY_IO_SIZE_MAX as usize) + 64];
        data[8] = b'\n';
        data[window_size as usize + 17] = b'\n';
        temp.write_all(&data).unwrap();

        let model = FileModel::new();
        model
            .load_file_with_mmap_chunk_size(temp.path(), false, window_size)
            .unwrap();
        model
            .restart_scan_with_auto_detect(3, UtfMode::Default, false)
            .unwrap();
        model.wait_scan_complete();

        assert_eq!(model.get_line_count(), 3);
        assert_eq!(model.get_line_pos(0), 3);
        assert_eq!(model.get_line_pos(1), 9);
        assert_eq!(model.get_line_pos(2), window_size as i64 + 18);
    }

    #[test]
    fn test_append_file_creates_missing_target() {
        let path = std::env::temp_dir().join("fjiffyldg-rs-append-create.txt");
        let _ = std::fs::remove_file(&path);

        append_file(&path, b"hello").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_file_large_buffer_round_trips() {
        let path = std::env::temp_dir().join("fjiffyldg-rs-save-large.bin");
        let _ = std::fs::remove_file(&path);
        let data = vec![b'a'; (USUALLY_IO_SIZE_MAX as usize) + 1];

        save_file(&path, &data).unwrap();

        assert_eq!(std::fs::metadata(&path).unwrap().len(), data.len() as u64);
        let stored = std::fs::read(&path).unwrap();
        assert_eq!(stored.len(), data.len());
        assert_eq!(stored[0], b'a');
        assert_eq!(stored[stored.len() - 1], b'a');
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_clone_file_large_input_round_trips() {
        let src = std::env::temp_dir().join("fjiffyldg-rs-clone-large-src.bin");
        let dst = std::env::temp_dir().join("fjiffyldg-rs-clone-large-dst.bin");
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
        let data = vec![b'b'; (USUALLY_IO_SIZE_MAX as usize) + 1];

        save_file(&src, &data).unwrap();
        clone_file(&src, &dst).unwrap();

        assert_eq!(std::fs::metadata(&dst).unwrap().len(), data.len() as u64);
        let stored = std::fs::read(&dst).unwrap();
        assert_eq!(stored.len(), data.len());
        assert_eq!(stored[0], b'b');
        assert_eq!(stored[stored.len() - 1], b'b');
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn test_concatenate_files_large_input_appends_to_output() {
        let output = std::env::temp_dir().join("fjiffyldg-rs-concat-out.bin");
        let append = std::env::temp_dir().join("fjiffyldg-rs-concat-append.bin");
        let _ = std::fs::remove_file(&output);
        let _ = std::fs::remove_file(&append);
        let prefix = b"prefix";
        let suffix = vec![b'c'; (USUALLY_IO_SIZE_MAX as usize) + 1];

        save_file(&output, prefix).unwrap();
        save_file(&append, &suffix).unwrap();
        concatenate_files([append.as_path()], output.as_path()).unwrap();

        let meta = std::fs::metadata(&output).unwrap();
        assert_eq!(meta.len(), (prefix.len() + suffix.len()) as u64);
        let stored = std::fs::read(&output).unwrap();
        assert_eq!(&stored[..prefix.len()], prefix);
        assert_eq!(stored[prefix.len()], b'c');
        assert_eq!(stored[stored.len() - 1], b'c');
        let _ = std::fs::remove_file(&output);
        let _ = std::fs::remove_file(&append);
    }
}

/// 获取文件大小（字节）
pub fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let meta = std::fs::metadata(path).map_err(|_| FjiffyldgError::FileInaccessible)?;
    if !meta.is_file() {
        return Err(FjiffyldgError::FileInaccessible);
    }
    Ok(meta.len())
}

/// 写后校验：验证文件实际大小与预期一致。
///
/// 如果 `get_file_size` 失败，保留原始错误（`FileInaccessible`）；
/// 仅在大小不匹配时返回 [`IncompleteWrite`](FjiffyldgError::IncompleteWrite)。
fn verify_file_size(path: &Path, expected: u64) -> Result<()> {
    let actual = get_file_size(path)?;
    if actual != expected {
        return Err(FjiffyldgError::IncompleteWrite);
    }
    Ok(())
}

/// 使用可写内存映射将大块数据保存到文件。
fn save_large_file(path: &Path, data: &[u8]) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;

    file.set_len(data.len() as u64)
        .map_err(|_| FjiffyldgError::StreamError)?;

    let mut mmap = unsafe { MmapMut::map_mut(&file) }.map_err(|_| FjiffyldgError::MmapError)?;
    mmap.copy_from_slice(data);
    mmap.flush().map_err(|_| FjiffyldgError::StreamError)
}

/// 以较大的顺序写缓冲追加数据，适合大文件追加与拼接。
fn append_large_data(path: &Path, data: &[u8]) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;
    let mut writer = BufWriter::with_capacity(LARGE_WRITE_BUFFER_SIZE, file);
    writer
        .write_all(data)
        .and_then(|_| writer.flush())
        .map_err(|_| FjiffyldgError::StreamError)
}

/// 克隆文件
pub fn clone_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let file_size = get_file_size(src)?;

    if file_size > USUALLY_IO_SIZE_MAX {
        clone_large_file(src, dst)?;
    } else {
        std::fs::copy(src, dst)
            .map(|_| ())
            .map_err(|_| FjiffyldgError::StreamError)?;
    }

    // 写后校验：目标文件大小应与源文件一致
    verify_file_size(dst, file_size)?;
    Ok(())
}

/// 大文件克隆 —— 平台特化实现。
///
/// 64 位平台使用 mmap 映射整个文件以获得最佳性能；
/// 32 位平台使用缓冲 I/O 以避免虚拟地址空间不足。
#[cfg(not(target_pointer_width = "32"))]
fn clone_large_file(src: &Path, dst: &Path) -> Result<()> {
    let file = File::open(src).map_err(|_| FjiffyldgError::FileInaccessible)?;
    let mmap = unsafe { Mmap::map(&file) }.map_err(|_| FjiffyldgError::MmapError)?;
    save_file(dst, &mmap)
}

/// 大文件克隆 —— 32 位平台使用缓冲 I/O。
#[cfg(target_pointer_width = "32")]
fn clone_large_file(src: &Path, dst: &Path) -> Result<()> {
    copy_file_buffered(src, dst)
}

/// 使用缓冲 I/O 复制文件（适用于 32 位平台虚拟地址空间受限场景）。
#[cfg(target_pointer_width = "32")]
fn copy_file_buffered(src: &Path, dst: &Path) -> Result<()> {
    let src_file = File::open(src).map_err(|_| FjiffyldgError::FileInaccessible)?;
    let dst_file = File::create(dst).map_err(|_| FjiffyldgError::FileInaccessible)?;
    let mut reader = std::io::BufReader::with_capacity(BUFFER_SIZE, src_file);
    let mut writer = std::io::BufWriter::with_capacity(BUFFER_SIZE, dst_file);
    std::io::copy(&mut reader, &mut writer).map_err(|_| FjiffyldgError::StreamError)?;
    writer.flush().map_err(|_| FjiffyldgError::StreamError)?;
    Ok(())
}

/// 保存内容到文件
pub fn save_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let expected_len = data.len() as u64;

    if expected_len > USUALLY_IO_SIZE_MAX {
        save_large_file(path, data)?;
    } else {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|_| FjiffyldgError::FileInaccessible)?;

        file.write_all(data)
            .map_err(|_| FjiffyldgError::StreamError)?;
    }

    // 写后校验：文件大小应与预期一致
    verify_file_size(path, expected_len)?;
    Ok(())
}

/// 追加内容到文件
pub fn append_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let append_len = data.len() as u64;

    // 记录追加前的文件大小（文件可能不存在）
    let size_before = get_file_size(path).unwrap_or(0);

    if append_len > USUALLY_IO_SIZE_MAX {
        append_large_data(path, data)?;
    } else {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|_| FjiffyldgError::FileInaccessible)?;

        file.write_all(data)
            .map_err(|_| FjiffyldgError::StreamError)?;
    }

    // 写后校验：文件大小应增长了 append_len
    verify_file_size(path, size_before + append_len)?;
    Ok(())
}

/// 合并多个文件
pub fn concatenate_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(
    files: I,
    output: P,
) -> Result<()> {
    let output = output.as_ref();
    let size_before = get_file_size(output).unwrap_or(0);
    let mut expected_total_size = size_before;

    let mut output_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;
    for file_path in files {
        let file_path = file_path.as_ref();
        let file_size = get_file_size(file_path)?;
        expected_total_size += file_size;

        if file_size > USUALLY_IO_SIZE_MAX {
            drop(output_file);
            concatenate_large_file(file_path, output)?;
            output_file = OpenOptions::new()
                .append(true)
                .open(output)
                .map_err(|_| FjiffyldgError::FileInaccessible)?;
        } else {
            let mut input_file =
                File::open(file_path).map_err(|_| FjiffyldgError::FileInaccessible)?;
            std::io::copy(&mut input_file, &mut output_file)
                .map_err(|_| FjiffyldgError::StreamError)?;
        }
    }

    // 写后校验：文件大小应与追加前大小 + 所有源文件大小之和一致
    verify_file_size(output, expected_total_size)?;
    Ok(())
}

/// 向输出文件追加大文件内容 —— 平台特化实现。
///
/// 64 位平台使用 mmap；32 位平台使用缓冲 I/O 以避免地址空间不足。
#[cfg(not(target_pointer_width = "32"))]
fn concatenate_large_file(src: &Path, dst: &Path) -> Result<()> {
    let input_file = File::open(src).map_err(|_| FjiffyldgError::FileInaccessible)?;
    let mmap = unsafe { Mmap::map(&input_file) }.map_err(|_| FjiffyldgError::MmapError)?;
    append_file(dst, &mmap)
}

/// 向输出文件追加大文件内容 —— 32 位平台使用缓冲 I/O。
#[cfg(target_pointer_width = "32")]
fn concatenate_large_file(src: &Path, dst: &Path) -> Result<()> {
    let src_file = File::open(src).map_err(|_| FjiffyldgError::FileInaccessible)?;
    let dst_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dst)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;
    let mut reader = std::io::BufReader::with_capacity(BUFFER_SIZE, src_file);
    let mut writer = std::io::BufWriter::with_capacity(BUFFER_SIZE, dst_file);
    std::io::copy(&mut reader, &mut writer).map_err(|_| FjiffyldgError::StreamError)?;
    writer.flush().map_err(|_| FjiffyldgError::StreamError)?;
    Ok(())
}
