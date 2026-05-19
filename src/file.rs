use crate::encoding::{detect_encoding, TextEncoding};
use crate::error::{FjiffyldgError, Result, UtfMode};
use crate::line_index::LineIndex;
use memmap2::Mmap;
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
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
/// 大文件映射分块大小：1GB
#[allow(dead_code)]
const MMAP_CHUNK_SIZE: u64 = 1024 * MB as u64;

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
    data: RwLock<Option<Vec<u8>>>,
    /// 大文件内存映射
    mmap: RwLock<Option<Mmap>>,
    /// 大文件分块映射状态
    mmap_offset: RwLock<u64>,

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
    /// 后台扫描进行中标志
    scanning: Arc<AtomicBool>,
}

impl FileModel {
    /// 创建新的文件模型实例
    pub fn new() -> Self {
        Self {
            line_index: Arc::new(LineIndex::new()),
            data: RwLock::new(None),
            mmap: RwLock::new(None),
            mmap_offset: RwLock::new(0),
            file: RwLock::new(None),
            file_size: RwLock::new(0),
            utf_mode: RwLock::new(UtfMode::Default),
            error_code: RwLock::new(0),
            is_loaded: RwLock::new(false),
            scanning: Arc::new(AtomicBool::new(false)),
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
        self.scanning.load(Ordering::Acquire)
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
        let path = path.as_ref();
        self.wait_scan_complete();
        self.line_index.clear();
        *self.data.write() = None;
        *self.mmap.write() = None;
        *self.file.write() = None;
        *self.is_loaded.write() = false;

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                *self.error_code.write() = FjiffyldgError::FileInaccessible.to_error_code();
                return Err(FjiffyldgError::FileInaccessible);
            }
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => {
                *self.error_code.write() = FjiffyldgError::StreamError.to_error_code();
                return Err(FjiffyldgError::StreamError);
            }
        };

        let file_size = metadata.len();
        *self.file_size.write() = file_size;
        *self.file.write() = Some(file);

        if file_size == 0 {
            *self.is_loaded.write() = true;
            *self.error_code.write() = 0;
            self.line_index.mark_scanned();
            return Ok(());
        }

        // 加载文件数据
        if file_size <= USUALLY_IO_SIZE_MAX {
            let mut buffer = Vec::new();
            if let Ok(mut f) = File::open(path) {
                if f.read_to_end(&mut buffer).is_ok() {
                    *self.data.write() = Some(buffer);
                }
            }
        } else {
            match unsafe { Mmap::map(&*self.file.read().as_ref().unwrap()) } {
                Ok(mmap) => {
                    *self.mmap.write() = Some(mmap);
                    *self.mmap_offset.write() = 0;
                }
                Err(_) => {
                    *self.error_code.write() = FjiffyldgError::MmapError.to_error_code();
                    return Err(FjiffyldgError::MmapError);
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
        self.scan_lines_background_from(0, None);
    }

    /// 从指定偏移开始后台扫描行结构
    fn scan_lines_background_from(&self, offset: u64, forced_utf_mode: Option<UtfMode>) {
        let scanning = Arc::clone(&self.scanning);
        let line_index = Arc::clone(&self.line_index);
        let data = self.get_raw_data();
        let file_utf_mode = Self::detect_utf_mode(&data);
        let scan_data = data.get(offset as usize..).unwrap_or_default().to_vec();
        let current_utf_mode = self.get_utf_mode();

        scanning.store(true, Ordering::Release);

        rayon::spawn(move || {
            let utf_mode = forced_utf_mode
                .filter(|mode| *mode != UtfMode::Default)
                .or_else(|| {
                    if current_utf_mode != UtfMode::Default {
                        Some(current_utf_mode)
                    } else {
                        Some(file_utf_mode)
                    }
                })
                .unwrap_or(UtfMode::Default);

            line_index.build_from_data_at(&scan_data, offset, utf_mode);
            scanning.store(false, Ordering::Release);
        });
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
        if !self.is_loaded() {
            *self.error_code.write() = FjiffyldgError::FileNotLoaded.to_error_code();
            return Err(FjiffyldgError::FileNotLoaded);
        }

        let file_size = *self.file_size.read();
        if offset > file_size {
            *self.error_code.write() = FjiffyldgError::InvalidOffset.to_error_code();
            return Err(FjiffyldgError::InvalidOffset);
        }

        self.wait_scan_complete();
        self.line_index.clear();
        self.set_utf_mode(utf_mode);
        self.scan_lines_background_from(offset, Some(utf_mode));
        *self.error_code.write() = 0;
        Ok(())
    }

    /// 等待行扫描完成
    ///
    /// 如果后台扫描仍在进行，阻塞至完成。
    pub fn wait_scan_complete(&self) {
        while self.is_scanning() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// 获取原始文件数据
    fn get_raw_data(&self) -> Vec<u8> {
        if let Some(ref data) = *self.data.read() {
            return data.clone();
        }

        if let Some(ref mmap) = *self.mmap.read() {
            return mmap.to_vec();
        }

        Vec::new()
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

        if !self.is_loaded() || pos < 0 || pos >= file_size {
            return None;
        }

        if len == 0 {
            len = BUFFER_SIZE;
        }

        let end_pos = (pos as usize).min(file_size as usize);
        let remaining = file_size as usize - end_pos;
        let actual_len = len.min(remaining);

        if let Some(ref data) = *self.data.read() {
            return Some(data[end_pos..end_pos + actual_len].to_vec());
        }

        if let Some(ref mmap) = *self.mmap.read() {
            return Some(mmap[end_pos..end_pos + actual_len].to_vec());
        }

        None
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

        // 计算行结束位置
        let mut end_pos = *self.file_size.read() as i64;
        if index + 1 < self.line_index.get_line_count() {
            let np = self.line_index.get_line_pos((index + 1) as usize);
            if np >= 0 {
                end_pos = np;
            }
        }

        let full_len = if end_pos > begin {
            (end_pos - begin) as usize
        } else {
            0
        };

        let actual_len = if *len == 0 {
            full_len.min(CRITICAL_LONGLINE_LEN)
        } else {
            (*len).min(full_len)
        };
        *epos = begin + actual_len as i64;
        *len = actual_len;

        if actual_len == 0 {
            None
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
        }

        let actual_len = if next_pos > begin {
            (next_pos - begin) as usize
        } else {
            0
        };

        *epos = next_pos;
        *len = actual_len;

        if actual_len == 0 {
            None
        } else {
            self.read_data(begin, actual_len)
        }
    }

    /// 从指定位置读取到行尾
    ///
    /// # 参数
    /// - `index`：行索引
    /// - `pos`：行内起始位置
    /// - `len`：最大读取长度。若为0，返回剩余内容
    pub fn read_to_end_of_line(&self, index: i64, pos: i64, len: &mut usize) -> Option<Vec<u8>> {
        let file_size = *self.file_size.read() as i64;
        let line_start = self.line_index.get_line_pos(index as usize);

        if line_start < 0 || pos < line_start || pos >= file_size {
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
        self.read_data(pos, actual_len)
    }

    /// 获取整个文件的内存映射（超大文件场景）
    ///
    /// 返回整个文件的只读映射。仅推荐用于需要随机访问全文的场景。
    pub fn get_huge_buffer<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|_| FjiffyldgError::FileInaccessible)?;

        match unsafe { Mmap::map(&file) } {
            Ok(mmap) => Ok(mmap.to_vec()),
            Err(_) => Err(FjiffyldgError::MmapError),
        }
    }

    /// 清空所有数据
    pub fn clear(&self) {
        self.wait_scan_complete();
        *self.data.write() = None;
        *self.mmap.write() = None;
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

/// 获取文件大小（字节）
pub fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|_| FjiffyldgError::FileInaccessible)
}

/// 克隆文件
pub fn clone_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<()> {
    std::fs::copy(src, dst)
        .map(|_| ())
        .map_err(|_| FjiffyldgError::StreamError)
}

/// 保存内容到文件
pub fn save_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;

    file.write_all(data)
        .map_err(|_| FjiffyldgError::StreamError)
}

/// 追加内容到文件
pub fn append_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;

    file.write_all(data)
        .map_err(|_| FjiffyldgError::StreamError)
}

/// 合并多个文件
pub fn concatenate_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(
    files: I,
    output: P,
) -> Result<()> {
    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)
        .map_err(|_| FjiffyldgError::FileInaccessible)?;

    for file_path in files {
        let mut input_file = File::open(file_path).map_err(|_| FjiffyldgError::FileInaccessible)?;
        std::io::copy(&mut input_file, &mut output_file)
            .map_err(|_| FjiffyldgError::StreamError)?;
    }

    Ok(())
}
