use crate::encoding::{detect_encoding, convert_to_utf8, TextEncoding};
use crate::error::{FjiffyldgError, Result, UtfMode};
use crate::line_index::LineIndex;
use memmap2::Mmap;
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

const KB: usize = 1024;
const MB: usize = 1024 * KB;
const USUALLY_IO_SIZE_MAX: u64 = 10 * MB as u64;
const BUFFER_SIZE: usize = 128 * KB;
const CRITICAL_LONGLINE_LEN: usize = 4 * KB;

pub struct FileModel {
    line_index: Arc<LineIndex>,
    data: RwLock<Option<Vec<u8>>>,
    mmap: RwLock<Option<Mmap>>,
    file: RwLock<Option<File>>,
    file_size: RwLock<u64>,
    utf_mode: RwLock<UtfMode>,
    error_code: RwLock<i32>,
    is_loaded: RwLock<bool>,
}

impl FileModel {
    pub fn new() -> Self {
        Self {
            line_index: Arc::new(LineIndex::new()),
            data: RwLock::new(None),
            mmap: RwLock::new(None),
            file: RwLock::new(None),
            file_size: RwLock::new(0),
            utf_mode: RwLock::new(UtfMode::Default),
            error_code: RwLock::new(0),
            is_loaded: RwLock::new(false),
        }
    }

    pub fn get_error_code(&self) -> i32 {
        *self.error_code.read()
    }

    pub fn is_loaded(&self) -> bool {
        *self.is_loaded.read()
    }

    pub fn get_file_size(&self) -> i64 {
        *self.file_size.read() as i64
    }

    pub fn get_utf_mode(&self) -> UtfMode {
        *self.utf_mode.read()
    }

    pub fn set_utf_mode(&self, mode: UtfMode) {
        *self.utf_mode.write() = mode;
        self.line_index.set_utf_mode(mode);
    }

    pub fn get_line_count(&self) -> i64 {
        if !self.is_loaded() {
            return -1;
        }
        self.line_index.get_line_count()
    }

    pub fn get_line_pos(&self, index: i64) -> i64 {
        self.line_index.get_line_pos(index as usize)
    }

    pub fn get_line_length(&self, index: i64) -> i64 {
        self.line_index.get_line_length(index as usize)
    }

    pub fn get_line_by_pos(&self, pos: i64) -> i64 {
        self.line_index.get_line_by_pos(pos)
    }

    pub fn load_and_scan_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.load_file(path, true)
    }

    pub fn load_file_only<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.load_file(path, false)
    }

    fn load_file<P: AsRef<Path>>(&self, path: P, enable_scan: bool) -> Result<()> {
        let path = path.as_ref();
        
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
            return Ok(());
        }
        
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
            self.scan_lines()?;
        }
        
        Ok(())
    }

    fn scan_lines(&self) -> Result<()> {
        let data = self.get_raw_data();
        if data.is_empty() {
            return Ok(());
        }
        
        let encoding = detect_encoding(&data);
        let mut effective_data = data.clone();
        
        match &encoding {
            TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
                if let Ok(converted) = convert_to_utf8(&data, &encoding) {
                    effective_data = converted;
                }
            }
            _ => {}
        }
        
        self.line_index.build_from_data(&effective_data, UtfMode::Default);
        
        Ok(())
    }

    fn get_raw_data(&self) -> Vec<u8> {
        if let Some(ref data) = *self.data.read() {
            return data.clone();
        }
        
        if let Some(ref mmap) = *self.mmap.read() {
            return mmap.to_vec();
        }
        
        Vec::new()
    }

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

    pub fn read_line(&self, index: i64, bpos: &mut i64, epos: &mut i64, len: &mut usize) -> Option<Vec<u8>> {
        let begin = self.line_index.get_line_pos(index as usize);
        if begin < 0 {
            *len = 0;
            return None;
        }
        
        *bpos = begin;
        
        let mut actual_len = *len;
        if actual_len == 0 {
            actual_len = BUFFER_SIZE;
        }
        
        let mut current_index = index;
        let mut current_pos = begin;
        
        while current_index + 1 < self.line_index.get_line_count() {
            let next_pos = self.line_index.get_line_pos((current_index + 1) as usize);
            if next_pos < 0 {
                break;
            }
            
            let line_len = next_pos - current_pos;
            if line_len > CRITICAL_LONGLINE_LEN as i64 {
                break;
            }
            
            if current_pos + line_len > begin + actual_len as i64 {
                break;
            }
            
            current_index += 1;
            current_pos = next_pos;
        }
        
        *epos = current_pos;
        actual_len = (*epos - *bpos) as usize;
        *len = actual_len;
        
        self.read_data(*bpos, actual_len)
    }

    pub fn read_to_end_of_line(&self, index: i64, pos: i64, len: &mut usize) -> Option<Vec<u8>> {
        let file_size = *self.file_size.read() as i64;
        
        let line_start = self.line_index.get_line_pos(index as usize);
        if line_start < 0 || pos < line_start || pos >= file_size {
            *len = 0;
            return None;
        }
        
        let mut end = file_size;
        if index + 1 < self.line_index.get_line_count() {
            let next_line = self.line_index.get_line_pos((index + 1) as usize);
            if next_line > 0 {
                end = next_line;
            }
        }
        
        if pos > end {
            *len = 0;
            return None;
        }
        
        let mut actual_len = *len;
        if actual_len == 0 {
            actual_len = CRITICAL_LONGLINE_LEN;
        }
        
        actual_len = (actual_len as i64).min(end - pos) as usize;
        *len = actual_len;
        
        self.read_data(pos, actual_len)
    }

    pub fn clear(&self) {
        *self.data.write() = None;
        *self.mmap.write() = None;
        *self.file.write() = None;
        *self.file_size.write() = 0;
        *self.is_loaded.write() = false;
        *self.error_code.write() = -1;
        self.line_index.clear();
    }
}

impl Default for FileModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for FileModel {
    fn drop(&mut self) {
        self.clear();
    }
}

pub fn get_file_size<P: AsRef<Path>>(path: P) -> i64 {
    std::fs::metadata(path)
        .map(|m| m.len() as i64)
        .unwrap_or(-1)
}

pub fn clone_file<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    dest: Q,
) -> std::result::Result<(), std::io::Error> {
    std::fs::copy(source, dest)?;
    Ok(())
}

pub fn save_file<P: AsRef<Path>>(path: P, data: &[u8]) -> std::result::Result<(), std::io::Error> {
    std::fs::write(path, data)
}

pub fn append_file<P: AsRef<Path>>(path: P, data: &[u8]) -> std::result::Result<(), std::io::Error> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    file.write_all(data)?;
    Ok(())
}

pub fn concatenate_files<P: AsRef<Path>, Q: AsRef<Path>>(
    target: P,
    source: Q,
) -> std::result::Result<(), std::io::Error> {
    let source_data = std::fs::read(source)?;
    append_file(&target, &source_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_model_load() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello\nworld\n").unwrap();
        
        let model = FileModel::new();
        assert!(model.load_and_scan_file(temp.path()).is_ok());
        assert_eq!(model.get_line_count(), 3);
    }

    #[test]
    fn test_read_data() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello world").unwrap();
        
        let model = FileModel::new();
        model.load_file_only(temp.path()).unwrap();
        
        let data = model.read_data(0, 5);
        assert_eq!(data, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_get_file_size() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"test").unwrap();
        
        assert_eq!(get_file_size(temp.path()), 4);
    }
}