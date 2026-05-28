//! 文件行索引管理。
//!
//! 本模块实现分级行索引结构，支持高效的大文件行级随机访问。
//!
//! # 三层索引架构
//!
//! | 层级 | 数据结构 | 容量 | 用途 |
//! |------|----------|------|------|
//! | 第一层 | `direct_offsets: Vec<u32>` | 前 100 万行 | O(1) 精确偏移读取 |
//! | 第二层 | `extended_offsets: Vec<u64>` | >4GB 文件 | 超大偏移支持 |
//! | 第三层 | `chunks: Vec<ChunkIndex>` | 8M × 128KB = 1TB | 范围裁剪加速查询 |
//!
//! 查询时先通过 chunk 索引裁剪二分范围，再在 direct/extended 数组中精确定位。
//! 超过 chunk 限制的行由 `overstep_pos` 记录首个溢出位置。

use crate::error::UtfMode;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// 分块索引结构
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ChunkIndex {
    /// 该分块最大行索引
    max_line_index: u64,
    /// 分块在文件中的起始字节位置
    start_pos: u64,
}

/// 常数定义
const DIRECT_LINES_MAX: usize = 1_000_000;
const CHUNK_BEGIN: usize = DIRECT_LINES_MAX;
#[allow(dead_code)]
const CHUNK_SIZE: u64 = 128 * 1024;
#[allow(dead_code)]
const CHUNK_COUNT_MAX: usize = 8_388_608;
#[allow(dead_code)]
const KB: u64 = 1024;
#[allow(dead_code)]
const MB: u64 = 1024 * KB;

/// 高性能行索引
///
/// 支持1T级文件的O(1)行位置查询。采用三层索引结构：
/// - 直接索引：≤1M行，每行4字节（u32偏移）
/// - 扩展索引：>4GB偏移，每行8字节（u64偏移）
/// - 分块索引：>1M行，O(log n)快速定位
pub struct LineIndex {
    /// 直接索引（行起始位置，u32）
    direct_offsets: RwLock<Vec<u32>>,
    /// 扩展索引（行起始位置，u64，用于>4GB偏移）
    extended_offsets: RwLock<Vec<u64>>,
    /// 每行内容长度（不含行尾符，单位为原始字节）
    line_lengths: RwLock<Vec<u64>>,
    /// 分块索引（用于>1M行快速查询）
    chunks: RwLock<Vec<ChunkIndex>>,
    /// 文件是否已扫描完成
    is_scanned: AtomicBool,
    /// 文件总行数
    total_lines: RwLock<u64>,
    /// 超过分块限制时的起始位置
    overstep_pos: RwLock<u64>,
    /// 缓存的行索引和字节位置（合并为单锁避免 TOCTOU 竞态）
    cached: RwLock<(u64, u64)>,
}

impl LineIndex {
    /// 创建新的行索引
    pub fn new() -> Self {
        Self {
            direct_offsets: RwLock::new(Vec::new()),
            extended_offsets: RwLock::new(Vec::new()),
            line_lengths: RwLock::new(Vec::new()),
            chunks: RwLock::new(Vec::new()),
            is_scanned: AtomicBool::new(false),
            total_lines: RwLock::new(0),
            overstep_pos: RwLock::new(0),
            cached: RwLock::new((u64::MAX, 0)),
        }
    }

    /// 获取扫描状态
    pub fn is_scanned(&self) -> bool {
        self.is_scanned.load(Ordering::Acquire)
    }

    /// 标记扫描完成
    pub fn mark_scanned(&self) {
        self.is_scanned.store(true, Ordering::Release);
    }

    /// 将空文件初始化为与 C++ 参考实现一致的单空行状态。
    pub fn mark_empty_file(&self) {
        self.clear();
        self.direct_offsets.write().push(0);
        self.line_lengths.write().push(0);
        *self.total_lines.write() = 1;
        self.mark_scanned();
    }

    /// 获取总行数
    pub fn get_line_count(&self) -> i64 {
        if self.is_scanned() {
            *self.total_lines.read() as i64
        } else {
            -1
        }
    }

    /// 从数据建立行索引
    pub fn build_from_data(&self, data: &[u8], _utf_mode: UtfMode) {
        self.build_from_data_at(data, 0, _utf_mode);
    }

    /// 从指定文件偏移处的数据建立行索引
    pub fn build_from_data_at(&self, data: &[u8], base_offset: u64, utf_mode: UtfMode) {
        let cancel_requested = AtomicBool::new(false);
        let _ = self.build_from_data_at_cancelable(data, base_offset, utf_mode, &cancel_requested);
    }

    /// 从指定文件偏移处的数据建立可取消的行索引
    ///
    /// 返回 `true` 表示完整构建完成，返回 `false` 表示在扫描过程中收到取消请求，
    /// 此时索引会被清空并标记为已结束。
    pub fn build_from_data_at_cancelable(
        &self,
        data: &[u8],
        base_offset: u64,
        utf_mode: UtfMode,
        cancel_requested: &AtomicBool,
    ) -> bool {
        self.clear();

        if cancel_requested.load(Ordering::Acquire) {
            self.mark_scanned();
            return false;
        }

        if data.is_empty() {
            self.mark_scanned();
            return true;
        }

        self.add_line(base_offset);

        let mut line_start = 0usize;
        let mut scan_pos = 0usize;

        while scan_pos < data.len() {
            if cancel_requested.load(Ordering::Acquire) {
                self.clear();
                self.mark_scanned();
                return false;
            }

            if let Some(newline_len) = Self::newline_len_at(data, scan_pos, utf_mode) {
                self.add_line_length((scan_pos - line_start) as u64);
                scan_pos += newline_len;
                line_start = scan_pos;
                self.add_line(base_offset + line_start as u64);
            } else {
                scan_pos += Self::scan_step(data, scan_pos, utf_mode);
            }
        }

        if cancel_requested.load(Ordering::Acquire) {
            self.clear();
            self.mark_scanned();
            return false;
        }

        self.add_line_length((data.len() - line_start) as u64);
        *self.total_lines.write() = self.line_lengths.read().len() as u64;
        self.mark_scanned();
        true
    }

    /// 从连续窗口读取器建立可取消的行索引。
    ///
    /// 调用方按需返回指定文件窗口的数据，本函数会保留足够的尾部字节以识别跨窗口
    /// 的 CRLF 或定宽编码换行符，从而避免在窗口边界产生错误行。返回 `false`
    /// 表示读取窗口失败或扫描被取消，此时索引会被清空并标记扫描结束。
    pub fn build_from_windows_at_cancelable<F>(
        &self,
        start_offset: u64,
        file_size: u64,
        chunk_size: u64,
        utf_mode: UtfMode,
        cancel_requested: &AtomicBool,
        mut read_window: F,
    ) -> bool
    where
        F: FnMut(u64, u64) -> Option<Vec<u8>>,
    {
        self.clear();

        if cancel_requested.load(Ordering::Acquire) {
            self.mark_scanned();
            return false;
        }

        if start_offset >= file_size {
            self.mark_scanned();
            return true;
        }

        self.add_line(start_offset);

        let chunk_size = chunk_size.max(1);
        let retain_len = Self::boundary_retain_len(utf_mode);
        let mut line_start_abs = start_offset;
        let mut scan_abs = start_offset;
        let mut cursor = start_offset;
        let mut pending_base = start_offset;
        let mut pending = Vec::new();

        while cursor < file_size {
            if cancel_requested.load(Ordering::Acquire) {
                self.clear();
                self.mark_scanned();
                return false;
            }

            let window_len = chunk_size.min(file_size - cursor);
            let Some(mut window) = read_window(cursor, window_len) else {
                self.clear();
                self.mark_scanned();
                return false;
            };

            if window.is_empty() {
                self.clear();
                self.mark_scanned();
                return false;
            }

            if pending.is_empty() {
                pending_base = cursor;
                scan_abs = cursor;
            }

            pending.append(&mut window);
            cursor += window_len;

            let process_len = if cursor < file_size {
                pending.len().saturating_sub(retain_len)
            } else {
                pending.len()
            };

            if !self.scan_pending_until(
                &pending,
                pending_base,
                &mut scan_abs,
                process_len,
                &mut line_start_abs,
                utf_mode,
                cancel_requested,
            ) {
                self.clear();
                self.mark_scanned();
                return false;
            }

            let scanned_len = (scan_abs - pending_base) as usize;
            if scanned_len > 0 {
                pending.drain(..scanned_len);
                pending_base = scan_abs;
            }
        }

        if cancel_requested.load(Ordering::Acquire) {
            self.clear();
            self.mark_scanned();
            return false;
        }

        self.add_line_length(file_size.saturating_sub(line_start_abs));
        *self.total_lines.write() = self.line_lengths.read().len() as u64;
        self.mark_scanned();
        true
    }

    /// 扫描暂存窗口中可安全处理的前缀。
    #[allow(clippy::too_many_arguments)]
    fn scan_pending_until(
        &self,
        pending: &[u8],
        pending_base: u64,
        scan_abs: &mut u64,
        process_len: usize,
        line_start_abs: &mut u64,
        utf_mode: UtfMode,
        cancel_requested: &AtomicBool,
    ) -> bool {
        let mut scan_pos = (*scan_abs - pending_base) as usize;

        while scan_pos < process_len {
            if cancel_requested.load(Ordering::Acquire) {
                return false;
            }

            if let Some(newline_len) = Self::newline_len_at(pending, scan_pos, utf_mode) {
                let newline_abs = pending_base + scan_pos as u64;
                self.add_line_length(newline_abs.saturating_sub(*line_start_abs));
                scan_pos += newline_len;
                *scan_abs = pending_base + scan_pos as u64;
                *line_start_abs = *scan_abs;
                self.add_line(*scan_abs);
            } else {
                scan_pos += Self::scan_step(pending, scan_pos, utf_mode);
                *scan_abs = pending_base + scan_pos as u64;
            }
        }

        true
    }

    /// 返回跨窗口扫描时需要保留的尾部字节数。
    fn boundary_retain_len(utf_mode: UtfMode) -> usize {
        match utf_mode {
            UtfMode::Default | UtfMode::AutoDetect => 1,
            UtfMode::Utf16Le | UtfMode::Utf16Be => 3,
            UtfMode::Utf32Le | UtfMode::Utf32Be => 7,
        }
    }

    /// 获取指定行的长度
    pub fn get_line_length(&self, index: usize) -> i64 {
        if !self.is_scanned() {
            return -1;
        }

        let total_lines = *self.total_lines.read() as usize;

        if index >= total_lines {
            return -1;
        }

        self.line_lengths.read()[index] as i64
    }

    /// 获取指定行的起始字节位置
    pub fn get_line_pos(&self, index: usize) -> i64 {
        let total_lines = if self.is_scanned() {
            *self.total_lines.read() as usize
        } else {
            self.direct_offsets.read().len() + self.extended_offsets.read().len()
        };

        if index >= total_lines {
            return -1;
        }

        // 检查缓存（原子读取，避免 TOCTOU 竞态）
        {
            let cached = self.cached.read();
            if cached.0 == index as u64 {
                return cached.1 as i64;
            }
        }

        let (left, right) = self.search_bounds_by_line(index);
        if index < left || index >= right {
            return -1;
        }

        let dir_offs = self.direct_offsets.read();
        let ext_offs = self.extended_offsets.read();
        let Some(pos) = Self::offset_at(index, &dir_offs, &ext_offs) else {
            return -1;
        };

        // 写入前 double-check：防止其他线程在读锁释放后已写入相同条目
        let mut cached = self.cached.write();
        if cached.0 == index as u64 {
            return cached.1 as i64;
        }
        *cached = (index as u64, pos);
        pos as i64
    }

    /// 根据字节位置查找所在行索引
    ///
    /// 当索引为空（未扫描或扫描中无数据）时返回 -1，
    /// 当有已索引数据时返回对应行号（即使扫描仍在进行中）。
    pub fn get_line_by_pos(&self, pos: i64) -> i64 {
        if pos < 0 {
            return -1;
        }

        let pos = pos as u64;
        let (mut left, mut right) = self.search_bounds_by_pos(pos);
        let dir_offs = self.direct_offsets.read();
        let ext_offs = self.extended_offsets.read();
        let total_offsets = dir_offs.len() + ext_offs.len();

        if total_offsets == 0 {
            return -1;
        }

        left = left.min(total_offsets);
        right = right.min(total_offsets);

        while left < right {
            let mid = (left + right) / 2;
            let Some(offset) = Self::offset_at(mid, &dir_offs, &ext_offs) else {
                right = mid;
                continue;
            };

            if offset <= pos {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        if left > 0 {
            (left - 1) as i64
        } else {
            0
        }
    }

    /// 读取指定全局行号对应的偏移。
    fn offset_at(index: usize, dir_offs: &[u32], ext_offs: &[u64]) -> Option<u64> {
        if index < dir_offs.len() {
            Some(dir_offs[index] as u64)
        } else {
            ext_offs.get(index - dir_offs.len()).copied()
        }
    }

    /// 使用分块索引为行号查询裁剪全局行偏移范围。
    fn search_bounds_by_line(&self, line_index: usize) -> (usize, usize) {
        let dir_len = self.direct_offsets.read().len();
        let ext_len = self.extended_offsets.read().len();
        let total_offsets = dir_len + ext_len;
        let chunks = self.chunks.read();

        if chunks.is_empty() || total_offsets == 0 || line_index < CHUNK_BEGIN {
            return (0, total_offsets);
        }

        let chunk_partition =
            chunks.partition_point(|chunk| chunk.max_line_index < line_index as u64);
        if chunk_partition >= chunks.len() {
            let left = chunks
                .last()
                .map(|chunk| chunk.max_line_index as usize + 1)
                .unwrap_or(CHUNK_BEGIN);
            return (left.min(total_offsets), total_offsets);
        }

        let left = if chunk_partition == 0 {
            CHUNK_BEGIN
        } else {
            chunks[chunk_partition - 1].max_line_index as usize + 1
        };
        let right = chunks[chunk_partition].max_line_index as usize + 1;

        (left.min(total_offsets), right.min(total_offsets))
    }

    /// 使用分块索引为字节位置查询裁剪全局行偏移二分范围。
    fn search_bounds_by_pos(&self, pos: u64) -> (usize, usize) {
        let dir_len = self.direct_offsets.read().len();
        let ext_len = self.extended_offsets.read().len();
        let total_offsets = dir_len + ext_len;
        let chunks = self.chunks.read();

        if chunks.is_empty() || total_offsets == 0 {
            return (0, total_offsets);
        }

        let overstep_pos = *self.overstep_pos.read();
        if overstep_pos != 0 && pos >= overstep_pos {
            let left = chunks
                .last()
                .map(|chunk| chunk.max_line_index as usize + 1)
                .unwrap_or(CHUNK_BEGIN);
            return (left.min(total_offsets), total_offsets);
        }

        let chunk_partition = chunks.partition_point(|chunk| chunk.start_pos <= pos);
        if chunk_partition == 0 {
            return (
                0,
                (chunks[0].max_line_index as usize + 1).min(total_offsets),
            );
        }

        let chunk_index = chunk_partition - 1;
        let left = if chunk_index == 0 {
            CHUNK_BEGIN
        } else {
            chunks[chunk_index - 1].max_line_index as usize + 1
        };
        let right = chunks[chunk_index].max_line_index as usize + 1;

        (left.min(total_offsets), right.min(total_offsets))
    }

    /// 等待扫描完成
    pub fn wait_scan(&self) {
        while !self.is_scanned() {
            std::thread::yield_now();
        }
    }

    /// 设置UTF模式
    pub fn set_utf_mode(&self, _mode: UtfMode) {
        // 目前不需要处理，但保持接口以向后兼容
    }

    /// 清空索引
    pub fn clear(&self) {
        // 缓存重置放在最前面，避免并发读者命中已失效的缓存条目
        *self.cached.write() = (u64::MAX, 0);
        self.is_scanned.store(false, Ordering::Release);
        self.direct_offsets.write().clear();
        self.extended_offsets.write().clear();
        self.line_lengths.write().clear();
        self.chunks.write().clear();
        *self.total_lines.write() = 0;
        *self.overstep_pos.write() = 0;
    }

    /// 添加一行的起始位置
    fn add_line(&self, pos: u64) {
        let mut dir_offs = self.direct_offsets.write();

        if dir_offs.len() < DIRECT_LINES_MAX {
            // 直接索引阶段
            if pos <= u32::MAX as u64 {
                dir_offs.push(pos as u32);
            } else {
                // 偏移超过 u32 范围，存入扩展索引
                drop(dir_offs);
                let mut ext_offs = self.extended_offsets.write();
                ext_offs.push(pos);
            }
        } else {
            // 使用两个数组的总长度作为行索引
            let line_index = dir_offs.len() as u64 + self.extended_offsets.read().len() as u64;
            drop(dir_offs);
            let mut ext_offs = self.extended_offsets.write();
            ext_offs.push(pos);
            drop(ext_offs);
            self.update_chunk_index(line_index, pos, CHUNK_COUNT_MAX);
        }
    }

    /// 根据新增的行起始位置更新第三层 chunk 索引。
    fn update_chunk_index(&self, line_index: u64, pos: u64, max_chunks: usize) {
        if line_index < CHUNK_BEGIN as u64 {
            return;
        }

        let mut chunks = self.chunks.write();
        if chunks.is_empty() {
            chunks.push(ChunkIndex {
                max_line_index: line_index,
                start_pos: pos,
            });
            return;
        }

        if chunks.len() < max_chunks {
            let last_chunk = chunks.last_mut().unwrap();
            if pos.saturating_sub(last_chunk.start_pos) < CHUNK_SIZE {
                last_chunk.max_line_index = line_index;
            } else {
                chunks.push(ChunkIndex {
                    max_line_index: line_index,
                    start_pos: pos,
                });
            }
            return;
        }

        drop(chunks);
        let mut overstep_pos = self.overstep_pos.write();
        if *overstep_pos == 0 {
            *overstep_pos = pos;
        }
    }

    /// 添加一行的内容长度
    fn add_line_length(&self, len: u64) {
        self.line_lengths.write().push(len);
    }

    /// 获取当前编码模式下的扫描步长
    fn scan_step(data: &[u8], pos: usize, utf_mode: UtfMode) -> usize {
        let width = match utf_mode {
            UtfMode::Utf16Le | UtfMode::Utf16Be => 2,
            UtfMode::Utf32Le | UtfMode::Utf32Be => 4,
            UtfMode::Default | UtfMode::AutoDetect => 1,
        };

        width.min(data.len() - pos)
    }

    /// 判断当前位置是否为换行符并返回行尾字节数
    fn newline_len_at(data: &[u8], pos: usize, utf_mode: UtfMode) -> Option<usize> {
        match utf_mode {
            UtfMode::Default | UtfMode::AutoDetect => Self::newline_len_u8(data, pos),
            UtfMode::Utf16Le => Self::newline_len_fixed(data, pos, 2, &[b'\r', 0], &[b'\n', 0]),
            UtfMode::Utf16Be => Self::newline_len_fixed(data, pos, 2, &[0, b'\r'], &[0, b'\n']),
            UtfMode::Utf32Le => {
                Self::newline_len_fixed(data, pos, 4, &[b'\r', 0, 0, 0], &[b'\n', 0, 0, 0])
            }
            UtfMode::Utf32Be => {
                Self::newline_len_fixed(data, pos, 4, &[0, 0, 0, b'\r'], &[0, 0, 0, b'\n'])
            }
        }
    }

    /// 判断当前位置是否为单字节换行符
    fn newline_len_u8(data: &[u8], pos: usize) -> Option<usize> {
        match data[pos] {
            b'\n' => Some(1),
            b'\r' => {
                if data.get(pos + 1) == Some(&b'\n') {
                    Some(2)
                } else {
                    Some(1)
                }
            }
            _ => None,
        }
    }

    /// 判断当前位置是否为定宽编码的换行符
    fn newline_len_fixed(
        data: &[u8],
        pos: usize,
        width: usize,
        cr: &[u8],
        lf: &[u8],
    ) -> Option<usize> {
        let unit = data.get(pos..pos + width)?;

        if unit == lf {
            return Some(width);
        }

        if unit == cr {
            let next = data.get(pos + width..pos + width * 2);
            if next == Some(lf) {
                Some(width * 2)
            } else {
                Some(width)
            }
        } else {
            None
        }
    }
}

impl Default for LineIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl LineIndex {
        fn chunk_count_for_tests(&self) -> usize {
            self.chunks.read().len()
        }

        fn overstep_pos_for_tests(&self) -> u64 {
            *self.overstep_pos.read()
        }

        fn add_chunk_for_tests(&self, line_index: u64, pos: u64, max_chunks: usize) {
            self.update_chunk_index(line_index, pos, max_chunks);
        }

        fn search_bounds_by_pos_for_tests(&self, pos: u64) -> (usize, usize) {
            self.search_bounds_by_pos(pos)
        }

        fn search_bounds_by_line_for_tests(&self, line_index: usize) -> (usize, usize) {
            self.search_bounds_by_line(line_index)
        }
    }

    #[test]
    fn test_line_index_basic() {
        let index = LineIndex::new();
        let data = b"line1\nline2\nline3\n";

        index.build_from_data(data, UtfMode::Default);

        assert_eq!(index.get_line_count(), 4);
        assert_eq!(index.get_line_length(0), 5);
        assert_eq!(index.get_line_length(1), 5);
        assert_eq!(index.get_line_length(2), 5);
        assert_eq!(index.get_line_length(3), 0);
    }

    #[test]
    fn test_get_line_by_pos() {
        let index = LineIndex::new();
        let data = b"line1\nline2\nline3\n";

        index.build_from_data(data, UtfMode::Default);

        assert_eq!(index.get_line_by_pos(0), 0);
        assert_eq!(index.get_line_by_pos(5), 0);
        assert_eq!(index.get_line_by_pos(6), 1);
        assert_eq!(index.get_line_by_pos(12), 2);
    }

    #[test]
    fn test_utf8_lines() {
        let index = LineIndex::new();
        let data = "你好\n世界\n".as_bytes();

        index.build_from_data(data, UtfMode::Default);

        assert_eq!(index.get_line_count(), 3);
        assert_eq!(index.get_line_length(0), 6);
        assert_eq!(index.get_line_length(1), 6);
        assert_eq!(index.get_line_length(2), 0);
    }

    #[test]
    fn test_crlf_line_positions_and_lengths() {
        let index = LineIndex::new();
        let data = b"line1\r\nline2\r\n";

        index.build_from_data(data, UtfMode::Default);

        assert_eq!(index.get_line_count(), 3);
        assert_eq!(index.get_line_pos(0), 0);
        assert_eq!(index.get_line_pos(1), 7);
        assert_eq!(index.get_line_pos(2), 14);
        assert_eq!(index.get_line_length(0), 5);
        assert_eq!(index.get_line_length(1), 5);
        assert_eq!(index.get_line_length(2), 0);
    }

    #[test]
    fn test_windowed_scan_preserves_crlf_across_boundary() {
        let index = LineIndex::new();
        let data = b"abc\r\ndef";
        let cancel_requested = AtomicBool::new(false);

        assert!(index.build_from_windows_at_cancelable(
            0,
            data.len() as u64,
            4,
            UtfMode::Default,
            &cancel_requested,
            |offset, len| Some(data[offset as usize..(offset + len) as usize].to_vec()),
        ));

        assert_eq!(index.get_line_count(), 2);
        assert_eq!(index.get_line_pos(0), 0);
        assert_eq!(index.get_line_pos(1), 5);
        assert_eq!(index.get_line_length(0), 3);
        assert_eq!(index.get_line_length(1), 3);
    }

    #[test]
    fn test_unterminated_last_line_length() {
        let index = LineIndex::new();
        let data = b"first\nsecond";

        index.build_from_data(data, UtfMode::Default);

        assert_eq!(index.get_line_count(), 2);
        assert_eq!(index.get_line_pos(0), 0);
        assert_eq!(index.get_line_pos(1), 6);
        assert_eq!(index.get_line_length(0), 5);
        assert_eq!(index.get_line_length(1), 6);
    }

    #[test]
    fn test_utf16le_line_positions_use_original_byte_offsets() {
        let index = LineIndex::new();
        let data = [b'a', 0, b'\r', 0, b'\n', 0, b'b', 0];

        index.build_from_data(&data, UtfMode::Utf16Le);

        assert_eq!(index.get_line_count(), 2);
        assert_eq!(index.get_line_pos(0), 0);
        assert_eq!(index.get_line_pos(1), 6);
        assert_eq!(index.get_line_length(0), 2);
        assert_eq!(index.get_line_length(1), 2);
    }

    #[test]
    fn test_utf16be_crlf_positions_use_original_byte_offsets() {
        let index = LineIndex::new();
        let data = [0, b'a', 0, b'\r', 0, b'\n', 0, b'b'];

        index.build_from_data(&data, UtfMode::Utf16Be);

        assert_eq!(index.get_line_count(), 2);
        assert_eq!(index.get_line_pos(0), 0);
        assert_eq!(index.get_line_pos(1), 6);
        assert_eq!(index.get_line_length(0), 2);
        assert_eq!(index.get_line_length(1), 2);
    }

    #[test]
    fn test_utf32le_lf_positions_use_original_byte_offsets() {
        let index = LineIndex::new();
        let data = [b'a', 0, 0, 0, b'\n', 0, 0, 0, b'b', 0, 0, 0];

        index.build_from_data_at(&data, 101, UtfMode::Utf32Le);

        assert_eq!(index.get_line_count(), 2);
        assert_eq!(index.get_line_pos(0), 101);
        assert_eq!(index.get_line_pos(1), 109);
        assert_eq!(index.get_line_length(0), 4);
        assert_eq!(index.get_line_length(1), 4);
    }

    #[test]
    fn test_utf32be_crlf_positions_use_original_byte_offsets() {
        let index = LineIndex::new();
        let data = [0, 0, 0, b'a', 0, 0, 0, b'\r', 0, 0, 0, b'\n', 0, 0, 0, b'b'];

        index.build_from_data(&data, UtfMode::Utf32Be);

        assert_eq!(index.get_line_count(), 2);
        assert_eq!(index.get_line_pos(0), 0);
        assert_eq!(index.get_line_pos(1), 12);
        assert_eq!(index.get_line_length(0), 4);
        assert_eq!(index.get_line_length(1), 4);
    }

    #[test]
    fn test_positions_after_direct_index_limit_are_preserved() {
        let index = LineIndex::new();
        let line_count = DIRECT_LINES_MAX + 2;
        let mut data = Vec::with_capacity(line_count * 2);

        for _ in 0..line_count {
            data.extend_from_slice(b"x\n");
        }

        index.build_from_data(&data, UtfMode::Default);

        assert_eq!(index.get_line_count(), (line_count + 1) as i64);
        assert_eq!(
            index.get_line_pos(DIRECT_LINES_MAX),
            (DIRECT_LINES_MAX * 2) as i64
        );
        assert_eq!(
            index.get_line_pos(DIRECT_LINES_MAX + 1),
            ((DIRECT_LINES_MAX + 1) * 2) as i64
        );
        assert_eq!(
            index.get_line_by_pos(((DIRECT_LINES_MAX + 1) * 2) as i64),
            (DIRECT_LINES_MAX + 1) as i64
        );
    }

    #[test]
    fn test_chunk_index_is_populated_after_direct_limit() {
        let index = LineIndex::new();
        let line_count = DIRECT_LINES_MAX + 70_000;
        let mut data = Vec::with_capacity(line_count * 2);

        for _ in 0..line_count {
            data.extend_from_slice(b"x\n");
        }

        index.build_from_data(&data, UtfMode::Default);

        assert!(index.chunk_count_for_tests() > 0);
        assert_eq!(
            index.get_line_pos(DIRECT_LINES_MAX),
            (DIRECT_LINES_MAX * 2) as i64
        );
        assert_eq!(
            index.get_line_by_pos(((DIRECT_LINES_MAX + 65_536) * 2) as i64),
            (DIRECT_LINES_MAX + 65_536) as i64
        );
    }

    #[test]
    fn test_chunk_index_narrows_position_search_bounds() {
        let index = LineIndex::new();
        let line_count = DIRECT_LINES_MAX + 140_000;
        let mut data = Vec::with_capacity(line_count * 2);

        for _ in 0..line_count {
            data.extend_from_slice(b"x\n");
        }

        index.build_from_data(&data, UtfMode::Default);

        let (left, right) =
            index.search_bounds_by_pos_for_tests(((DIRECT_LINES_MAX + 65_536) * 2) as u64);

        assert!(left >= DIRECT_LINES_MAX);
        assert!(right < index.direct_offsets.read().len() + index.extended_offsets.read().len());
        assert!(right - left < 140_000);
    }

    #[test]
    fn test_chunk_index_narrows_line_search_bounds() {
        let index = LineIndex::new();
        let line_count = DIRECT_LINES_MAX + 140_000;
        let mut data = Vec::with_capacity(line_count * 2);

        for _ in 0..line_count {
            data.extend_from_slice(b"x\n");
        }

        index.build_from_data(&data, UtfMode::Default);

        let target = DIRECT_LINES_MAX + 65_536;
        let (left, right) = index.search_bounds_by_line_for_tests(target);

        assert!(left >= DIRECT_LINES_MAX);
        assert!(left <= target);
        assert!(right > target);
        assert!(right - left < 140_000);
        assert_eq!(index.get_line_pos(target), (target * 2) as i64);
    }

    #[test]
    fn test_overstep_position_records_first_chunk_overflow() {
        let index = LineIndex::new();

        index.add_chunk_for_tests(DIRECT_LINES_MAX as u64, 1_000, 1);
        index.add_chunk_for_tests((DIRECT_LINES_MAX + 1) as u64, 1_000 + CHUNK_SIZE + 10, 1);

        assert_eq!(index.chunk_count_for_tests(), 1);
        assert_eq!(index.overstep_pos_for_tests(), 1_000 + CHUNK_SIZE + 10);
    }

    #[test]
    fn test_overstep_position_narrows_search_after_last_chunk() {
        let index = LineIndex::new();
        index.direct_offsets.write().resize(DIRECT_LINES_MAX, 0);
        index.extended_offsets.write().extend_from_slice(&[
            1_000,
            1_000 + CHUNK_SIZE + 10,
            1_000 + CHUNK_SIZE + 20,
        ]);

        index.add_chunk_for_tests(DIRECT_LINES_MAX as u64, 1_000, 1);
        index.add_chunk_for_tests((DIRECT_LINES_MAX + 1) as u64, 1_000 + CHUNK_SIZE + 10, 1);

        let (left, right) = index.search_bounds_by_pos_for_tests(1_000 + CHUNK_SIZE + 20);

        assert_eq!(left, DIRECT_LINES_MAX + 1);
        assert_eq!(right, DIRECT_LINES_MAX + 3);
        *index.total_lines.write() = (DIRECT_LINES_MAX + 3) as u64;
        index.mark_scanned();
        assert_eq!(
            index.get_line_by_pos((1_000 + CHUNK_SIZE + 20) as i64),
            (DIRECT_LINES_MAX + 2) as i64
        );
    }

    #[test]
    fn test_overstep_position_narrows_line_search_after_last_chunk() {
        let index = LineIndex::new();
        index.direct_offsets.write().resize(DIRECT_LINES_MAX, 0);
        index.extended_offsets.write().extend_from_slice(&[
            1_000,
            1_000 + CHUNK_SIZE + 10,
            1_000 + CHUNK_SIZE + 20,
        ]);
        *index.total_lines.write() = (DIRECT_LINES_MAX + 3) as u64;
        index.mark_scanned();

        index.add_chunk_for_tests(DIRECT_LINES_MAX as u64, 1_000, 1);
        index.add_chunk_for_tests((DIRECT_LINES_MAX + 1) as u64, 1_000 + CHUNK_SIZE + 10, 1);

        let (left, right) = index.search_bounds_by_line_for_tests(DIRECT_LINES_MAX + 2);

        assert_eq!(left, DIRECT_LINES_MAX + 1);
        assert_eq!(right, DIRECT_LINES_MAX + 3);
        assert_eq!(
            index.get_line_pos(DIRECT_LINES_MAX + 2),
            (1_000 + CHUNK_SIZE + 20) as i64
        );
    }

    #[test]
    fn test_cancelled_build_leaves_empty_scanned_index() {
        let index = LineIndex::new();
        let cancel_requested = AtomicBool::new(true);

        let completed = index.build_from_data_at_cancelable(
            b"line1\nline2\n",
            0,
            UtfMode::Default,
            &cancel_requested,
        );

        assert!(!completed);
        assert!(index.is_scanned());
        assert_eq!(index.get_line_count(), 0);
        assert_eq!(index.get_line_pos(0), -1);
    }
}
