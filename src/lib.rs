pub mod encoding;
pub mod error;
pub mod file;
pub mod line_index;

pub use encoding::*;
pub use error::{FjiffyldgError, Result, UtfMode};
pub use file::{FileModel, get_file_size, clone_file, save_file, append_file, concatenate_files};

use std::path::Path;

#[derive(Clone)]
pub struct Fjiffyldg {
    inner: std::sync::Arc<FileModel>,
}

impl Fjiffyldg {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(FileModel::new()),
        }
    }

    pub fn load_and_scan<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.inner.load_and_scan_file(path)
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.inner.load_file_only(path)
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.is_loaded()
    }

    pub fn error_code(&self) -> i32 {
        self.inner.get_error_code()
    }

    pub fn file_size(&self) -> i64 {
        self.inner.get_file_size()
    }

    pub fn set_utf_mode(&self, mode: UtfMode) {
        self.inner.set_utf_mode(mode)
    }

    pub fn utf_mode(&self) -> UtfMode {
        self.inner.get_utf_mode()
    }

    pub fn line_count(&self) -> i64 {
        self.inner.get_line_count()
    }

    pub fn line_pos(&self, index: i64) -> i64 {
        self.inner.get_line_pos(index)
    }

    pub fn line_length(&self, index: i64) -> i64 {
        self.inner.get_line_length(index)
    }

    pub fn line_at_position(&self, pos: i64) -> i64 {
        self.inner.get_line_by_pos(pos)
    }

    pub fn read(&self, pos: i64, len: usize) -> Option<Vec<u8>> {
        self.inner.read_data(pos, len)
    }

    pub fn read_line(&self, index: i64, bpos: &mut i64, epos: &mut i64, len: &mut usize) -> Option<Vec<u8>> {
        self.inner.read_line(index, bpos, epos, len)
    }

    pub fn read_to_eol(&self, index: i64, pos: i64, len: &mut usize) -> Option<Vec<u8>> {
        self.inner.read_to_end_of_line(index, pos, len)
    }

    pub fn get_handle(&self) -> &FileModel {
        &self.inner
    }

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

pub fn check_ascii(text: &[u8]) -> usize {
    encoding::check_text_ascii(text)
}

pub fn check_utf8(text: &[u8]) -> usize {
    encoding::check_whole_text_utf8(text)
}

pub fn check_utf8_extract(text: &[u8]) -> usize {
    encoding::check_extract_text_utf8(text)
}

pub fn utf8_char_count(text: &[u8]) -> usize {
    encoding::get_utf8_char_count(text)
}

pub fn detect_text_encoding(data: &[u8]) -> encoding::TextEncoding {
    encoding::detect_encoding(data)
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
        assert_eq!(fjiffyldg.line_count(), 4);
        assert_eq!(fjiffyldg.line_length(0), 5);
    }

    #[test]
    fn test_utf8_functions() {
        assert_eq!(check_ascii(b"hello"), 0);
        let result = check_utf8("你好".as_bytes());
        assert_eq!(result, 0);
        assert_eq!(utf8_char_count("hello".as_bytes()), 5);
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
}

pub mod prelude {
    pub use super::{Fjiffyldg, FjiffyldgError, Result, UtfMode};
    pub use super::encoding::TextEncoding;
}