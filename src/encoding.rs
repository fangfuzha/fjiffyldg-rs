use encoding_rs::{UTF_16LE, UTF_16BE};

#[inline]
pub fn is_ascii_char(byte: u8) -> bool {
    (byte & 0x80) == 0
}

pub fn check_text_ascii(text: &[u8]) -> usize {
    if text.len() < 8 {
        let mut i = 0;
        while i < text.len() {
            if (text[i] & 0x80) != 0 {
                return text.len() - i;
            }
            i += 1;
        }
        return 0;
    }
    
    let offset = (text.as_ptr() as usize) % 8;
    let mut pos = 0;
    
    if offset != 0 {
        let aligned = 8 - offset;
        let lim = (aligned as usize).min(text.len());
        for i in 0..lim {
            if (text[i] & 0x80) != 0 {
                return text.len() - i;
            }
        }
        pos = aligned;
    }
    
    while pos + 8 <= text.len() {
        let chunk = u64::from_le_bytes([
            text[pos], text[pos+1], text[pos+2], text[pos+3],
            text[pos+4], text[pos+5], text[pos+6], text[pos+7]
        ]);
        
        if (chunk & 0x8080808080808080) != 0 {
            for i in pos..text.len() {
                if (text[i] & 0x80) != 0 {
                    return text.len() - i;
                }
            }
        }
        pos += 8;
    }
    
    while pos < text.len() {
        if (text[pos] & 0x80) != 0 {
            return text.len() - pos;
        }
        pos += 1;
    }
    
    0
}

#[inline]
fn get_utf8_char_width(first_byte: u8) -> Option<usize> {
    if (first_byte & 0x80) == 0 {
        Some(1)
    } else if (first_byte & 0xE0) == 0xC0 {
        Some(2)
    } else if (first_byte & 0xF0) == 0xE0 {
        Some(3)
    } else if (first_byte & 0xF8) == 0xF0 {
        Some(4)
    } else {
        None
    }
}

#[inline]
fn is_valid_utf8_continuation(byte: u8) -> bool {
    (byte & 0xC0) == 0x80
}

pub fn check_utf8_char(text: &[u8], width: usize) -> bool {
    if text.len() < width {
        return false;
    }
    
    for i in 1..width {
        if !is_valid_utf8_continuation(text[i]) {
            return false;
        }
    }
    true
}

pub fn check_whole_text_utf8(text: &[u8]) -> usize {
    let mut pos = 0;
    
    while pos < text.len() {
        let Some(width) = get_utf8_char_width(text[pos]) else {
            return text.len() - pos;
        };
        
        if pos + width > text.len() {
            return text.len() - pos;
        }
        
        if !check_utf8_char(&text[pos..], width) {
            return text.len() - pos;
        }
        
        pos += width;
    }
    
    0
}

pub fn get_utf8_char_count(text: &[u8]) -> usize {
    let mut count = 0;
    let mut pos = 0;
    
    while pos < text.len() {
        let Some(width) = get_utf8_char_width(text[pos]) else {
            break;
        };
        
        if pos + width > text.len() || !check_utf8_char(&text[pos..], width) {
            break;
        }
        
        count += 1;
        pos += width;
    }
    
    count
}

pub fn check_extract_text_utf8(text: &[u8]) -> usize {
    if text.is_empty() {
        return 0;
    }
    
    if text.len() < 10 {
        return check_whole_text_utf8(text);
    }
    
    let mut start_offset = 0;
    let mut end_offset = 0;
    
    if let Some(width) = get_utf8_char_width(text[0]) {
        if width > 1 && width <= text.len() {
            let mut cont = 1;
            while cont < width && cont < text.len() {
                if !is_valid_utf8_continuation(text[cont]) {
                    return check_whole_text_utf8(text);
                }
                cont += 1;
            }
            start_offset = width;
        }
    }
    
    if let Some(width) = get_utf8_char_width(text[text.len() - 1]) {
        if width > 1 {
            let remaining = &text[..text.len().saturating_sub(width)];
            let last_char_start = remaining.len();
            
            if last_char_start > 0 {
                if let Some(last_width) = get_utf8_char_width(text[last_char_start]) {
                    if last_width >= width {
                        end_offset = last_width;
                    }
                }
            }
        }
    }
    
    let check_len = text.len() - start_offset - end_offset;
    if check_len == 0 {
        return 0;
    }
    
    let remaining = check_whole_text_utf8(&text[start_offset..start_offset + check_len]);
    if remaining > 0 {
        remaining + end_offset
    } else {
        0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEncoding {
    Ascii,
    Utf8,
    Utf16Le,
    Utf16Be,
    Unknown,
}

pub fn detect_encoding(data: &[u8]) -> TextEncoding {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        return TextEncoding::Utf8;
    }
    if data.len() >= 2 {
        if data[0] == 0xFF && data[1] == 0xFE {
            return TextEncoding::Utf16Le;
        }
        if data[0] == 0xFE && data[1] == 0xFF {
            return TextEncoding::Utf16Be;
        }
    }
    
    if check_whole_text_utf8(data) == 0 {
        TextEncoding::Utf8
    } else if check_text_ascii(data) == 0 {
        TextEncoding::Ascii
    } else {
        TextEncoding::Unknown
    }
}

pub fn convert_to_utf8(data: &[u8], encoding: &TextEncoding) -> std::result::Result<Vec<u8>, ()> {
    match encoding {
        TextEncoding::Utf16Le => {
            let (decoded, _, _) = UTF_16LE.decode(data);
            Ok(decoded.into_owned().into_bytes())
        }
        TextEncoding::Utf16Be => {
            let (decoded, _, _) = UTF_16BE.decode(data);
            Ok(decoded.into_owned().into_bytes())
        }
        _ => Ok(data.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_check() {
        assert_eq!(check_text_ascii(b"hello world"), 0);
        assert_eq!(check_text_ascii(b"hello\xff"), 1);
    }

    #[test]
    fn test_utf8_check() {
        assert_eq!(check_whole_text_utf8("hello".as_bytes()), 0);
        assert_eq!(check_whole_text_utf8("你好".as_bytes()), 0);
        assert_eq!(check_whole_text_utf8(b"hello\xff"), 1);
    }

    #[test]
    fn test_utf8_char_count() {
        assert_eq!(get_utf8_char_count("hello".as_bytes()), 5);
        assert_eq!(get_utf8_char_count("你好世界".as_bytes()), 4);
    }
}