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
    /// 缓存的行索引
    cached_line: RwLock<u64>,
    /// 缓存的字节位置
    cached_pos: RwLock<u64>,
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
            cached_line: RwLock::new(u64::MAX),
            cached_pos: RwLock::new(0),
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
        self.clear();

        if data.is_empty() {
            self.mark_scanned();
            return;
        }

        self.add_line(base_offset);

        let mut line_start = 0usize;
        let mut scan_pos = 0usize;

        while scan_pos < data.len() {
            if let Some(newline_len) = Self::newline_len_at(data, scan_pos, utf_mode) {
                self.add_line_length((scan_pos - line_start) as u64);
                scan_pos += newline_len;
                line_start = scan_pos;
                self.add_line(base_offset + line_start as u64);
            } else {
                scan_pos += Self::scan_step(data, scan_pos, utf_mode);
            }
        }

        self.add_line_length((data.len() - line_start) as u64);

        *self.total_lines.write() = self.line_lengths.read().len() as u64;

        self.mark_scanned();
    }

    /// 获取指定行的长度
    pub fn get_line_length(&self, index: usize) -> i64 {
        let total_lines = *self.total_lines.read() as usize;

        if index >= total_lines {
            return -1;
        }

        self.line_lengths.read()[index] as i64
    }

    /// 获取指定行的起始字节位置
    pub fn get_line_pos(&self, index: usize) -> i64 {
        let total_lines = *self.total_lines.read() as usize;

        if index >= total_lines {
            return -1;
        }

        // 检查缓存
        let cached_line = *self.cached_line.read();
        if cached_line == index as u64 {
            return *self.cached_pos.read() as i64;
        }

        let dir_offs = self.direct_offsets.read();
        let ext_offs = self.extended_offsets.read();

        if index < dir_offs.len() {
            let pos = dir_offs[index] as i64;
            drop(dir_offs);
            drop(ext_offs);

            *self.cached_line.write() = index as u64;
            *self.cached_pos.write() = pos as u64;
            pos
        } else {
            let pos = ext_offs[index - dir_offs.len()] as i64;
            drop(dir_offs);
            drop(ext_offs);

            *self.cached_line.write() = index as u64;
            *self.cached_pos.write() = pos as u64;
            pos
        }
    }

    /// 根据字节位置查找所在行索引
    pub fn get_line_by_pos(&self, pos: i64) -> i64 {
        if pos < 0 {
            return -1;
        }

        let dir_offs = self.direct_offsets.read();
        let ext_offs = self.extended_offsets.read();
        let pos = pos as u64;

        // 二分查找
        let mut left = 0;
        let mut right = dir_offs.len() + ext_offs.len();

        while left < right {
            let mid = (left + right) / 2;
            let offset = if mid < dir_offs.len() {
                dir_offs[mid] as u64
            } else {
                ext_offs[mid - dir_offs.len()]
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
        self.direct_offsets.write().clear();
        self.extended_offsets.write().clear();
        self.line_lengths.write().clear();
        self.chunks.write().clear();
        *self.total_lines.write() = 0;
        *self.overstep_pos.write() = 0;
        *self.cached_line.write() = u64::MAX;
        *self.cached_pos.write() = 0;
        self.is_scanned.store(false, Ordering::Release);
    }

    /// 添加一行的起始位置
    fn add_line(&self, pos: u64) {
        let mut dir_offs = self.direct_offsets.write();

        if dir_offs.len() < DIRECT_LINES_MAX {
            // 直接索引阶段
            if pos <= u32::MAX as u64 {
                dir_offs.push(pos as u32);
            } else {
                drop(dir_offs);
                let mut ext_offs = self.extended_offsets.write();
                ext_offs.push(pos);
            }
        } else {
            drop(dir_offs);
            let mut ext_offs = self.extended_offsets.write();
            ext_offs.push(pos);
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
            UtfMode::Default => 1,
        };

        width.min(data.len() - pos)
    }

    /// 判断当前位置是否为换行符并返回行尾字节数
    fn newline_len_at(data: &[u8], pos: usize, utf_mode: UtfMode) -> Option<usize> {
        match utf_mode {
            UtfMode::Default => Self::newline_len_u8(data, pos),
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
}
