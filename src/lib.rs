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
pub mod file;
pub mod line_index;

pub use encoding::{
    check_text_ascii, check_whole_text_utf8, check_extract_text_utf8, detect_encoding, get_utf8_char_count,
    TextEncoding,
};
pub use error::{FjiffyldgError, Result, UtfMode};
pub use file::{
    append_file, clone_file, concatenate_files, get_file_size, save_file, FileModel,
};

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

    /// 检查是否仍在扫描中
    pub fn is_scanning(&self) -> bool {
        self.inner.is_scanning()
    }

    /// 等待扫描完成
    pub fn wait_scan(&self) {
        self.inner.wait_scan_complete();
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
    /// - `len`：实际读取长度
    pub fn read_line(
        &self,
        index: i64,
        bpos: &mut i64,
        epos: &mut i64,
        len: &mut usize,
    ) -> Option<Vec<u8>> {
        self.inner.read_line(index, bpos, epos, len)
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
    use std::io::Write;
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
    fn test_encoding_detection() {
        assert_eq!(detect_encoding(b"hello"), TextEncoding::Ascii);
        assert_eq!(detect_encoding("你好".as_bytes()), TextEncoding::Utf8);
    }
}

pub mod prelude {
    pub use super::{Fjiffyldg, FjiffyldgError, Result, UtfMode};
    pub use super::encoding::TextEncoding;
}