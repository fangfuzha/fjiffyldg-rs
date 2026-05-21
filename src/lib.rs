//! # Fjiffyldg - 超高性能文件处理库
//!
//! 一个跨平台、超高性能的Rust文件处理库，支持1B~1TB级别的大文件。
//!
//! ## 核心特性
//!
//! - **分级索引**：支持100万+行数文件，内存占用恒定
//! - **智能加载**：文件≤10MB直接加载，>10MB使用内存映射
//! - **异步扫描**：后台线程扫描行结构，主线程不阻塞
//! - **编码自动检测**：支持ASCII、UTF-8、UTF-16、UTF-32
//! - **性能优化**：SIMD加速的ASCII检查，与C++版本性能相当或更优
//!
//! ## 使用示例
//!
//! ```no_run
//! use fjiffyldg::Fjiffyldg;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let fjiffyldg = Fjiffyldg::new();
//!
//! // 加载并扫描文件
//! fjiffyldg.load_and_scan("large_file.txt")?;
//!
//! // 获取文件基本信息
//! println!("Total lines: {}", fjiffyldg.line_count());
//! println!("File size: {} bytes", fjiffyldg.file_size());
//!
//! // 随机访问指定行
//! let pos = fjiffyldg.line_pos(100);
//! let len = fjiffyldg.line_length(100);
//! if let Some(data) = fjiffyldg.read(pos, len as usize) {
//!     println!("Line 100: {:?}", String::from_utf8_lossy(&data));
//! }
//!
//! // 检查编码
//! let encoding = fjiffyldg::detect_encoding(b"hello world");
//! println!("Encoding: {:?}", encoding);
//! # Ok(())
//! # }
//! ```

pub mod encoding;
pub mod error;
pub mod ffi;
pub mod file;
pub mod line_index;

pub use encoding::{
    check_extract_text_utf8, check_text_ascii, check_whole_text_utf8, detect_encoding,
    get_utf8_char_count, get_utf8_char_count_with_offset, TextEncoding,
};
pub use error::{FjiffyldgError, Result, UtfMode};
pub use file::{append_file, clone_file, concatenate_files, get_file_size, save_file, FileModel};

use std::path::Path;

/// Fjiffyldg主API入口
///
/// 提供高层次的文件处理接口，支持链式调用和易用的错误处理。
///
/// # 示例
///
/// ```no_run
/// use fjiffyldg::Fjiffyldg;
///
/// let fjiff = Fjiffyldg::new();
/// fjiff.load_and_scan("data.txt")?;
/// # Result::<_, Box<dyn std::error::Error>>::Ok(())
/// ```
#[derive(Clone)]
pub struct Fjiffyldg {
    inner: std::sync::Arc<FileModel>,
}

impl Fjiffyldg {
    /// 创建新的Fjiffyldg实例
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(FileModel::new()),
        }
    }

    /// 加载文件并异步扫描行结构
    ///
    /// 立即返回，扫描在后台进行。可通过 `is_scanning()` 检查进度。
    pub fn load_and_scan<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.inner.load_and_scan_file(path)
    }

    /// 仅加载文件（不扫描行）
    ///
    /// 用于只需读取原始数据不需要行操作的场景。
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.inner.load_file_only(path)
    }

    /// 检查文件是否已加载
    pub fn is_loaded(&self) -> bool {
        self.inner.is_loaded()
    }

    /// 获取文件加载状态
    ///
    /// 返回 `Ok(true)` 表示已加载，`Ok(false)` 表示尚未加载且没有错误，
    /// 返回 `Err` 表示最近一次加载或文件操作失败。
    pub fn load_status(&self) -> Result<bool> {
        self.inner.get_load_status()
    }

    /// 检查是否仍在扫描中
    pub fn is_scanning(&self) -> bool {
        self.inner.is_scanning()
    }

    /// 等待扫描完成
    pub fn wait_scan(&self) {
        self.inner.wait_scan_complete();
    }

    /// 请求停止后台扫描并清空当前行索引
    pub fn request_stop_scan(&self) {
        self.inner.request_stop_scan();
    }

    /// 获取最后的错误码
    pub fn error_code(&self) -> i32 {
        self.inner.get_error_code()
    }

    /// 获取文件大小（字节）
    pub fn file_size(&self) -> i64 {
        self.inner.get_file_size()
    }

    /// 设置UTF编码模式
    pub fn set_utf_mode(&self, mode: UtfMode) {
        self.inner.set_utf_mode(mode)
    }

    /// 重新扫描已加载文件的行结构
    ///
    /// # 参数
    /// - `offset`：重新扫描的文件起始字节偏移
    /// - `utf_mode`：扫描时使用的定宽编码模式，传入 [`UtfMode::Default`] 时自动按 BOM 检测
    pub fn restart_scan(&self, offset: u64, utf_mode: UtfMode) -> Result<()> {
        self.inner.restart_scan(offset, utf_mode)
    }

    /// 获取当前UTF编码模式
    pub fn utf_mode(&self) -> UtfMode {
        self.inner.get_utf_mode()
    }

    /// 获取文件总行数
    ///
    /// # 返回值
    /// - 若扫描完成：返回行数（≥1）
    /// - 若扫描进行中：返回 -1
    /// - 若文件未加载：返回 -1
    pub fn line_count(&self) -> i64 {
        self.inner.get_line_count()
    }

    /// 获取指定行的起始字节位置
    pub fn line_pos(&self, index: i64) -> i64 {
        self.inner.get_line_pos(index)
    }

    /// 获取指定行的长度（字节）
    pub fn line_length(&self, index: i64) -> i64 {
        self.inner.get_line_length(index)
    }

    /// 根据字节位置查找所在行
    pub fn line_at_pos(&self, pos: i64) -> i64 {
        self.inner.get_line_by_pos(pos)
    }

    /// 从指定位置读取指定长度的数据
    pub fn read(&self, pos: i64, len: usize) -> Option<Vec<u8>> {
        self.inner.read_data(pos, len)
    }

    /// 读取指定行的数据
    ///
    /// # 输出参数
    /// - `bpos`：行起始位置
    /// - `epos`：行结束位置
    /// - `len`：输入最大读取长度，输出实际读取长度；若为 0，最多读取 4KB
    pub fn read_line(
        &self,
        index: i64,
        bpos: &mut i64,
        epos: &mut i64,
        len: &mut usize,
    ) -> Option<Vec<u8>> {
        self.inner.read_line(index, bpos, epos, len)
    }

    /// 按 C++ `ReadFileDataLLineCut` 语义读取一段行数据
    ///
    /// 短行会按行边界批量读取，`len` 用作批量预算而不是硬上限；超长行会按
    /// 4KB 临界值截断。返回时 `index` 会推进到最后一个完整纳入读取范围的行。
    pub fn read_line_cut(
        &self,
        index: &mut i64,
        bpos: &mut i64,
        epos: &mut i64,
        len: &mut usize,
    ) -> Option<Vec<u8>> {
        self.inner.read_line_cut(index, bpos, epos, len)
    }

    /// 从行内指定位置读取到行尾
    pub fn read_to_line_end(&self, index: i64, pos: i64, len: &mut usize) -> Option<Vec<u8>> {
        self.inner.read_to_end_of_line(index, pos, len)
    }

    /// 获取内部FileModel句柄
    pub fn handle(&self) -> &FileModel {
        &self.inner
    }

    /// 清空所有数据
    pub fn clear(&self) {
        self.inner.clear()
    }
}

impl Default for Fjiffyldg {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Fjiffyldg {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::Write;
    use std::ptr;
    use tempfile::NamedTempFile;

    #[test]
    fn test_fjiffyldg_basic() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\nline3\n").unwrap();

        let fjiffyldg = Fjiffyldg::new();
        assert!(fjiffyldg.load_and_scan(temp.path()).is_ok());
        fjiffyldg.wait_scan();
        assert_eq!(fjiffyldg.line_count(), 4);
        assert_eq!(fjiffyldg.line_length(0), 5);
    }

    #[test]
    fn test_utf8_functions() {
        assert_eq!(check_text_ascii(b"hello"), 0);
        let result = check_whole_text_utf8("你好".as_bytes());
        assert_eq!(result, 0);
        assert_eq!(get_utf8_char_count("hello".as_bytes()), 5);
    }

    #[test]
    fn test_file_operations() {
        let mut temp1 = NamedTempFile::new().unwrap();
        let temp2 = NamedTempFile::new().unwrap();

        temp1.write_all(b"hello world").unwrap();

        assert!(clone_file(temp1.path(), temp2.path()).is_ok());

        let content = std::fs::read(temp2.path()).unwrap();
        assert_eq!(content, b"hello world");
    }

    #[test]
    fn test_c_ffi_smoke_load_scan_query_and_read() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert!(!handle.is_null());

        assert_eq!(ffi::GetFileIsLoaded(handle), -1);
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        assert_eq!(ffi::GetFileIsLoaded(handle), 0);
        assert_eq!(ffi::GetFileLineCount(handle), 3);
        assert_eq!(ffi::GetFileLinePos(handle, 1), 6);
        assert_eq!(ffi::GetFileLineLength(handle, 1), 5);
        assert_eq!(ffi::GetFileLineIndex(handle, 7), 1);

        let mut len = 5;
        let data = ffi::ReadFileData(handle, 0, &mut len);
        assert!(!data.is_null());
        let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len as usize) };
        assert_eq!(len, 5);
        assert_eq!(data, b"line1");

        assert_eq!(ffi::LoadAndScanFile(ptr::null_mut(), path.as_ptr()), -1);
        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_line_cut_overwrites_out_params() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"a\nb\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        let mut index = 0;
        let mut bpos = i64::MIN;
        let mut epos = i64::MIN;
        let mut len = 0;
        let data = ffi::ReadFileDataLLineCut(handle, &mut index, &mut bpos, &mut epos, &mut len);

        assert!(!data.is_null());
        assert_eq!(index, 2);
        assert_eq!(bpos, 0);
        assert_eq!(epos, 4);
        assert_eq!(len, 4);
        let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len as usize) };
        assert_eq!(data, b"a\nb\n");

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_line_cut_preserves_bounds_when_lookup_fails() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"a\nb\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        let mut index = 99;
        let mut bpos = 1234;
        let mut epos = 5678;
        let mut len = 42;
        let data = ffi::ReadFileDataLLineCut(handle, &mut index, &mut bpos, &mut epos, &mut len);

        assert!(data.is_null());
        assert_eq!(index, 99);
        assert_eq!(bpos, 1234);
        assert_eq!(epos, 5678);
        assert_eq!(len, 0);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_text_and_file_helpers_handle_boundaries() {
        let mut text = ptr::null();
        assert_eq!(ffi::GetUtf8TextCharCount(&mut text, 0), 0);
        assert!(text.is_null());

        let temp = NamedTempFile::new().unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();
        let data = CString::new("hello").unwrap();

        assert_eq!(ffi::ToSaveFile(path.as_ptr(), data.as_ptr(), 5), 0);
        assert_eq!(ffi::ToAppendFile(path.as_ptr(), data.as_ptr(), 5), 0);
        assert_eq!(std::fs::read(temp.path()).unwrap(), b"hellohello");
    }

    #[test]
    fn test_c_ffi_load_file_only_reports_scan_not_started() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadFileOnly(handle, path.as_ptr()), 0);

        assert_eq!(ffi::GetFileIsLoaded(handle), 0);
        assert_eq!(ffi::GetFileLineCount(handle), 0);
        assert_eq!(ffi::GetFileLinePos(handle, 0), -1);
        assert_eq!(ffi::GetFileLineLength(handle, 0), -1);
        assert_eq!(ffi::GetFileLineIndex(handle, 0), -1);

        let mut len = 5;
        let data = ffi::ReadFileData(handle, 0, &mut len);
        assert!(!data.is_null());
        let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len as usize) };
        assert_eq!(data, b"line1");

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_line_index_rejects_positions_past_file_end() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        assert_eq!(ffi::GetFileLineIndex(handle, 12), 2);
        assert_eq!(ffi::GetFileLineIndex(handle, 13), -1);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_read_data_clamps_positions_at_file_end() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadFileOnly(handle, path.as_ptr()), 0);

        let mut len = 4;
        let at_end = ffi::ReadFileData(handle, 12, &mut len);
        assert!(!at_end.is_null());
        assert_eq!(len, 0);

        len = 4;
        let past_end = ffi::ReadFileData(handle, 999, &mut len);
        assert!(!past_end.is_null());
        assert_eq!(len, 0);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_read_data_clamps_negative_positions_to_start() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadFileOnly(handle, path.as_ptr()), 0);

        let mut len = 5;
        let data = ffi::ReadFileData(handle, -8, &mut len);
        assert!(!data.is_null());
        let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len as usize) };
        assert_eq!(len, 5);
        assert_eq!(data, b"line1");

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_empty_file_matches_cpp_observable_line_state() {
        let temp = NamedTempFile::new().unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        assert_eq!(ffi::GetFileIsLoaded(handle), 0);
        assert_eq!(ffi::GetFileLineCount(handle), 1);
        assert_eq!(ffi::GetFileLinePos(handle, 0), 0);
        assert_eq!(ffi::GetFileLineLength(handle, 0), 0);
        assert_eq!(ffi::GetFileLineIndex(handle, 0), 0);

        let mut len = 8;
        let data = ffi::ReadFileData(handle, 0, &mut len);
        assert!(!data.is_null());
        assert_eq!(len, 0);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_empty_file_load_only_keeps_scan_not_started() {
        let temp = NamedTempFile::new().unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadFileOnly(handle, path.as_ptr()), 0);

        assert_eq!(ffi::GetFileIsLoaded(handle), 0);
        assert_eq!(ffi::GetFileLineCount(handle), 0);
        assert_eq!(ffi::GetFileLinePos(handle, 0), -1);
        assert_eq!(ffi::GetFileLineLength(handle, 0), -1);
        assert_eq!(ffi::GetFileLineIndex(handle, 0), -1);

        let mut len = 8;
        let data = ffi::ReadFileData(handle, 0, &mut len);
        assert!(!data.is_null());
        assert_eq!(len, 0);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_read_to_end_of_line_returns_empty_at_line_boundary() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        let mut len = 8;
        let data = ffi::ReadFileDataEndOfLine(handle, 0, 6, &mut len);
        assert!(!data.is_null());
        assert_eq!(len, 0);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_read_to_end_of_line_returns_empty_at_file_end() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        let mut len = 8;
        let data = ffi::ReadFileDataEndOfLine(handle, 2, 12, &mut len);
        assert!(!data.is_null());
        assert_eq!(len, 0);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_c_ffi_restart_scan_distinguishes_auto_detect_from_default() {
        let mut temp = NamedTempFile::new().unwrap();
        let mut data = vec![0xFF, 0xFE];
        for unit in "skip\nline\n".encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        temp.write_all(&data).unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::WaitFileScanTaskFinished(handle);

        ffi::RestartScanFile(handle, path.as_ptr(), 12, -1);
        ffi::WaitFileScanTaskFinished(handle);
        assert_eq!(ffi::GetFileLinePos(handle, 0), 12);
        assert_eq!(ffi::GetFileLinePos(handle, 1), 22);

        ffi::RestartScanFile(handle, path.as_ptr(), 12, 0);
        ffi::WaitFileScanTaskFinished(handle);
        assert_eq!(ffi::GetFileLinePos(handle, 0), 12);
        assert_eq!(ffi::GetFileLinePos(handle, 1), 21);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_load_status_reports_unloaded_error_and_loaded() {
        let fjiffyldg = Fjiffyldg::new();
        assert_eq!(fjiffyldg.load_status(), Ok(false));

        let missing_path = std::env::temp_dir().join("fjiffyldg-rs-missing-load-status.txt");
        let _ = std::fs::remove_file(&missing_path);
        assert_eq!(
            fjiffyldg.load(&missing_path),
            Err(FjiffyldgError::FileInaccessible)
        );
        assert_eq!(
            fjiffyldg.load_status(),
            Err(FjiffyldgError::FileInaccessible)
        );

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello").unwrap();
        fjiffyldg.load(temp.path()).unwrap();

        assert_eq!(fjiffyldg.load_status(), Ok(true));
    }

    #[test]
    fn test_encoding_detection() {
        assert_eq!(detect_encoding(b"hello"), TextEncoding::Ascii);
        assert_eq!(detect_encoding("你好".as_bytes()), TextEncoding::Utf8);
    }

    #[test]
    fn test_restart_scan_from_offset() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"skip\nline1\nline2\n").unwrap();

        let fjiffyldg = Fjiffyldg::new();
        fjiffyldg.load_and_scan(temp.path()).unwrap();
        fjiffyldg.wait_scan();

        fjiffyldg.restart_scan(5, UtfMode::Default).unwrap();
        fjiffyldg.wait_scan();

        assert_eq!(fjiffyldg.line_count(), 3);
        assert_eq!(fjiffyldg.line_pos(0), 5);
        assert_eq!(fjiffyldg.line_length(0), 5);
    }

    #[test]
    fn test_request_stop_scan_is_safe_when_no_scan_is_running() {
        let fjiffyldg = Fjiffyldg::new();
        fjiffyldg.request_stop_scan();
        assert!(!fjiffyldg.is_scanning());
    }

    #[test]
    fn test_request_stop_scan_clears_index_after_background_scan() {
        let mut temp = NamedTempFile::new().unwrap();
        let mut data = Vec::with_capacity(200_000);
        for _ in 0..100_000 {
            data.extend_from_slice(b"x\n");
        }
        temp.write_all(&data).unwrap();

        let fjiffyldg = Fjiffyldg::new();
        fjiffyldg.load_and_scan(temp.path()).unwrap();
        fjiffyldg.request_stop_scan();

        assert!(!fjiffyldg.is_scanning());
        assert_eq!(fjiffyldg.line_count(), 0);
        assert_eq!(fjiffyldg.line_pos(0), -1);
    }

    #[test]
    fn test_c_ffi_backstage_request_stop_clears_index() {
        let mut temp = NamedTempFile::new().unwrap();
        let mut data = Vec::with_capacity(200_000);
        for _ in 0..100_000 {
            data.extend_from_slice(b"x\n");
        }
        temp.write_all(&data).unwrap();
        let path = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

        let handle = ffi::fjiffyldg_create();
        assert_eq!(ffi::LoadAndScanFile(handle, path.as_ptr()), 0);
        ffi::BackstageRequestStop(handle);

        assert_eq!(ffi::GetFileLineCount(handle), 0);
        assert_eq!(ffi::GetFileLinePos(handle, 0), -1);

        ffi::fjiffyldg_clear(handle);
    }

    #[test]
    fn test_read_line_defaults_to_long_line_cutoff() {
        let mut temp = NamedTempFile::new().unwrap();
        let long_line = vec![b'x'; 5 * 1024];
        temp.write_all(&long_line).unwrap();
        temp.write_all(b"\nnext\n").unwrap();

        let fjiffyldg = Fjiffyldg::new();
        fjiffyldg.load_and_scan(temp.path()).unwrap();
        fjiffyldg.wait_scan();

        let mut begin = -1;
        let mut end = -1;
        let mut len = 0;
        let data = fjiffyldg
            .read_line(0, &mut begin, &mut end, &mut len)
            .unwrap();

        assert_eq!(begin, 0);
        assert_eq!(end, 4096);
        assert_eq!(len, 4096);
        assert_eq!(data.len(), 4096);

        let mut index = 0;
        let mut begin = -1;
        let mut end = -1;
        let mut len = 0;
        let data = fjiffyldg
            .read_line_cut(&mut index, &mut begin, &mut end, &mut len)
            .unwrap();

        assert_eq!(index, 0);
        assert_eq!(begin, 0);
        assert_eq!(end, 4096);
        assert_eq!(len, 4096);
        assert_eq!(data.len(), 4096);
    }

    #[test]
    fn test_read_line_cut_batches_short_lines() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"a\nb\nc\n").unwrap();

        let fjiffyldg = Fjiffyldg::new();
        fjiffyldg.load_and_scan(temp.path()).unwrap();
        fjiffyldg.wait_scan();

        let mut index = 0;
        let mut begin = -1;
        let mut end = -1;
        let mut len = 0;
        let data = fjiffyldg
            .read_line_cut(&mut index, &mut begin, &mut end, &mut len)
            .unwrap();

        assert_eq!(index, 3);
        assert_eq!(begin, 0);
        assert_eq!(end, 6);
        assert_eq!(len, 6);
        assert_eq!(data, b"a\nb\nc\n");

        let mut index = 0;
        let mut begin = -1;
        let mut end = -1;
        let mut len = 1;
        let data = fjiffyldg
            .read_line_cut(&mut index, &mut begin, &mut end, &mut len)
            .unwrap();

        assert_eq!(index, 0);
        assert_eq!(begin, 0);
        assert_eq!(end, 2);
        assert_eq!(len, 2);
        assert_eq!(data, b"a\n");
    }

    #[test]
    fn test_restart_scan_default_uses_file_bom_before_offset() {
        let mut temp = NamedTempFile::new().unwrap();
        let mut data = vec![0xFF, 0xFE];
        for unit in "skip\nline\n".encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        temp.write_all(&data).unwrap();

        let fjiffyldg = Fjiffyldg::new();
        fjiffyldg.load_and_scan(temp.path()).unwrap();
        fjiffyldg.wait_scan();

        fjiffyldg.restart_scan(12, UtfMode::Default).unwrap();
        fjiffyldg.wait_scan();

        assert_eq!(fjiffyldg.line_count(), 2);
        assert_eq!(fjiffyldg.line_pos(0), 12);
        assert_eq!(fjiffyldg.line_length(0), 8);
        assert_eq!(fjiffyldg.line_pos(1), 22);
    }
}

pub mod prelude {
    pub use super::encoding::TextEncoding;
    pub use super::{Fjiffyldg, FjiffyldgError, Result, UtfMode};
}
