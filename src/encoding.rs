use encoding_rs::{UTF_16LE, UTF_16BE};

/// 检查单个字节是否为ASCII字符
#[inline]
pub fn is_ascii_char(byte: u8) -> bool {
    (byte & 0x80) == 0
}

/// 检查文本是否为ASCII编码
///
/// 使用SIMD优化，8字节对齐检查，对齐处理边界情况。
///
/// # 返回值
/// - `0`：整个文本为ASCII
/// - `>0`：第一个非ASCII字符的距离（从末尾开始）
///
/// # 性能
/// 与C++版本equivalent，对8字节对齐做了优化。
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
    
    // 处理对齐前缀
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
    
    // SIMD优化：8字节块检查
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
    
    // 处理尾部字节
    while pos < text.len() {
        if (text[pos] & 0x80) != 0 {
            return text.len() - pos;
        }
        pos += 1;
    }
    
    0
}

/// 获取UTF-8字符的字节宽度
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

/// 检查字节是否为有效的UTF-8续集字节
#[inline]
fn is_valid_utf8_continuation(byte: u8) -> bool {
    (byte & 0xC0) == 0x80
}

/// 检查单个UTF-8字符的有效性
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

/// 完整检查文本UTF-8编码有效性
///
/// # 返回值
/// - 0：整个文本为有效UTF-8
/// - >0：第一个错误位置（从末尾开始）
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

/// 获取UTF-8文本的字符数（仅计算有效字符）
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

/// 抽取式UTF-8检查（适合中间数据片段）
///
/// 忽略首尾可能不完整的多字节字符，仅检查中间部分。
/// 用于流式处理场景。
///
/// # 返回值
/// - 0：中间部分为有效UTF-8
/// - >0：错误位置偏移
pub fn check_extract_text_utf8(text: &[u8]) -> usize {
    if text.is_empty() {
        return 0;
    }
    
    if text.len() < 10 {
        return check_whole_text_utf8(text);
    }
    
    let mut start_offset = 0;
    let mut end_offset = 0;
    
    // 跳过首字节可能不完整的字符
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
    
    // 跳过尾字节可能不完整的字符
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

/// 文本编码类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEncoding {
    /// ASCII 编码
    Ascii,
    /// UTF-8 编码
    Utf8,
    /// UTF-16 小端
    Utf16Le,
    /// UTF-16 大端
    Utf16Be,
    /// 未知编码
    Unknown,
}

/// 检测文本编码
///
/// 优先级：BOM标记 > UTF-8 > ASCII > Unknown
///
/// # 支持的编码
/// - UTF-8（BOM: EF BB BF）
/// - UTF-16LE（BOM: FF FE）
/// - UTF-16BE（BOM: FE FF）
/// - ASCII（无非ASCII字节）
pub fn detect_encoding(data: &[u8]) -> TextEncoding {
    // 检查BOM标记
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
    
    // 尝试UTF-8验证和ASCII检查
    // 先检查ASCII（ASCII是UTF-8的子集，所以要优先判断）
    if check_text_ascii(data) == 0 {
        TextEncoding::Ascii
    } else if check_whole_text_utf8(data) == 0 {
        TextEncoding::Utf8
    } else {
        TextEncoding::Unknown
    }
}

/// 将文本转换为UTF-8
///
/// 对UTF-16/32自动进行编码转换，其他编码直接返回原始数据。
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