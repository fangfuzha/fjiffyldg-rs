use crate::error::UtfMode;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct LineIndex {
    offsets: RwLock<Vec<u64>>,
    lengths: RwLock<Vec<u64>>,
    is_scanned: AtomicBool,
    utf_mode: RwLock<UtfMode>,
}

impl LineIndex {
    pub fn new() -> Self {
        Self {
            offsets: RwLock::new(Vec::with_capacity(1024)),
            lengths: RwLock::new(Vec::with_capacity(1024)),
            is_scanned: AtomicBool::new(false),
            utf_mode: RwLock::new(UtfMode::Default),
        }
    }

    pub fn set_utf_mode(&self, mode: UtfMode) {
        *self.utf_mode.write() = mode;
    }

    pub fn get_utf_mode(&self) -> UtfMode {
        *self.utf_mode.read()
    }

    pub fn clear(&self) {
        self.offsets.write().clear();
        self.lengths.write().clear();
        self.is_scanned.store(false, Ordering::SeqCst);
    }

    pub fn is_scanned(&self) -> bool {
        self.is_scanned.load(Ordering::SeqCst)
    }

    pub fn mark_scanned(&self) {
        self.is_scanned.store(true, Ordering::SeqCst);
    }

    pub fn get_line_count(&self) -> i64 {
        if self.is_scanned() {
            self.lengths.read().len() as i64
        } else {
            -1
        }
    }

    pub fn build_from_data(&self, data: &[u8], _utf_mode: UtfMode) {
        let mut offsets = Vec::new();
        let mut lengths = Vec::new();
        
        offsets.push(0);
        
        let line_ends: Vec<usize> = self.find_line_ends(data);
        
        let mut prev_pos = 0;
        
        for &pos in &line_ends {
            offsets.push((pos + 1) as u64);
            
            let line_len = if pos >= prev_pos { pos - prev_pos } else { 0 };
            lengths.push(line_len as u64);
            
            prev_pos = pos + 1;
        }
        
        let last_line_len = if prev_pos <= data.len() {
            data.len() - prev_pos
        } else {
            0
        };
        
        if !data.is_empty() || last_line_len > 0 {
            lengths.push(last_line_len as u64);
        }
        
        *self.offsets.write() = offsets;
        *self.lengths.write() = lengths;
        
        self.mark_scanned();
    }

    fn find_line_ends(&self, data: &[u8]) -> Vec<usize> {
        let mut line_ends = Vec::new();
        
        let mut pos = 0;
        while pos < data.len() {
            let byte = data[pos];
            
            if byte == b'\n' {
                line_ends.push(pos);
            } else if byte == b'\r' {
                line_ends.push(pos);
                
                if pos + 1 < data.len() && data[pos + 1] == b'\n' {
                    pos += 1;
                }
            }
            pos += 1;
        }
        
        line_ends
    }

    pub fn get_line_pos(&self, index: usize) -> i64 {
        let offsets = self.offsets.read();
        if index < offsets.len() {
            offsets[index] as i64
        } else {
            -1
        }
    }

    pub fn get_line_length(&self, index: usize) -> i64 {
        let lengths = self.lengths.read();
        if index < lengths.len() {
            lengths[index] as i64
        } else {
            -1
        }
    }

    pub fn get_line_by_pos(&self, pos: i64) -> i64 {
        let offsets = self.offsets.read();
        if offsets.is_empty() {
            return -1;
        }
        
        let pos = pos as usize;
        let mut left = 0;
        let mut right = offsets.len();
        
        while left < right {
            let mid = (left + right) / 2;
            if offsets[mid] <= pos as u64 {
                if mid + 1 == offsets.len() || offsets[mid + 1] > pos as u64 {
                    return mid as i64;
                }
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        0
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
}