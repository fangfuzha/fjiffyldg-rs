//! 文本编码检测、校验与 UTF-8 转换工具。
//!
//! 本模块提供独立的编码检测与文本校验函数，无需文件句柄即可使用。
//!
//! # 功能概览
//!
//! | 函数 | 用途 |
//! |------|------|
//! | [`detect_encoding`] | 根据 BOM 头检测文本编码 |
//! | [`check_text_ascii`] | SIMD 加速的 ASCII 检查 |
//! | [`check_whole_text_utf8`] | 完整 UTF-8 有效性校验 |
//! | [`check_extract_text_utf8`] | 抽取式 UTF-8 检查（适合流式片段） |
//! | [`get_utf8_char_count`] | 统计有效 UTF-8 字符数 |
//! | [`get_utf8_char_count_with_offset`] | 字符数 + 已消费字节数 |
//! | [`is_ascii_char`] | 检查单个字节是否为 ASCII |
//! | [`check_utf8_char`] | 检查单个 UTF-8 字符的有效性 |
//! | [`convert_to_utf8`] | 将文本从 UTF-16/UTF-32 转换为 UTF-8 |
//! | [`TextEncoding`] | 文本编码类型枚举 |
//!
//! # 示例
//!
//! ```
//! use fjiffyldg::{detect_encoding, check_text_ascii, get_utf8_char_count, TextEncoding};
//!
//! // BOM 检测
//! assert_eq!(detect_encoding(b"\xFF\xFE\x00\x00"), TextEncoding::Utf32Le);
//!
//! // ASCII 检查（0 = 全部 ASCII）
//! assert_eq!(check_text_ascii(b"hello"), 0);
//!
//! // UTF-8 字符计数
//! assert_eq!(get_utf8_char_count("你好".as_bytes()), 2);
//! ```

use encoding_rs::{UTF_16BE, UTF_16LE};

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
        let lim = aligned.min(text.len());
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
            text[pos],
            text[pos + 1],
            text[pos + 2],
            text[pos + 3],
            text[pos + 4],
            text[pos + 5],
            text[pos + 6],
            text[pos + 7],
        ]);

        if (chunk & 0x8080808080808080) != 0 {
            // 掩码已确认非 ASCII 字节在当前 8 字节块内，只扫描块内字节
            for i in pos..(pos + 8).min(text.len()) {
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
///
/// 验证续接字节模式、超长编码（overlong）、UTF-16 代理对半区和超出 U+10FFFF 的码点。
pub fn check_utf8_char(text: &[u8], width: usize) -> bool {
    if text.len() < width {
        return false;
    }

    // 检查续接字节模式
    if !text
        .iter()
        .take(width)
        .skip(1)
        .all(|byte| is_valid_utf8_continuation(*byte))
    {
        return false;
    }

    // 拒绝超长编码（overlong encoding）
    // 2 字节序列的有效范围：U+0080..U+07FF（首字节 C2..DF）
    if width == 2 && text[0] < 0xC2 {
        return false;
    }

    // 3 字节 overlong：E0 80..9F 编码 U+0000..U+07FF（应用 1-2 字节）
    if width == 3 && text[0] == 0xE0 && text[1] < 0xA0 {
        return false;
    }

    // 4 字节 overlong：F0 80..8F 编码 U+0000..U+0FFF（应用 1-3 字节）
    if width == 4 && text[0] == 0xF0 && text[1] < 0x90 {
        return false;
    }

    // 拒绝 UTF-16 代理对半区（U+D800..U+DFFF）
    // 3 字节序列首字节 ED，第二字节 A0..BF
    // 续接字节检查已保证 text[1] ∈ 0x80..=0xBF，只需检查 ≥ 0xA0
    if width == 3 && text[0] == 0xED && text[1] >= 0xA0 {
        return false;
    }

    // 拒绝超出 U+10FFFF 的码点
    // 4 字节序列首字节 > F4，或首字节 F4 但第二字节 > 8F
    if width == 4 && (text[0] > 0xF4 || (text[0] == 0xF4 && text[1] > 0x8F)) {
        return false;
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
    get_utf8_char_count_with_offset(text).0
}

/// 获取UTF-8文本的有效字符数和已消费字节数
///
/// 返回 `(字符数, 已消费字节数)`。遇到非法或不完整 UTF-8 字节序列时停止，
/// `已消费字节数` 指向第一个未处理字节，便于调用方继续处理或定位错误。
pub fn get_utf8_char_count_with_offset(text: &[u8]) -> (usize, usize) {
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

    (count, pos)
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
    while start_offset < 3
        && start_offset < text.len()
        && is_valid_utf8_continuation(text[start_offset])
    {
        start_offset += 1;
    }

    let text = &text[start_offset..];
    // 确认首字节是有效的 UTF-8 起始字节
    if !text
        .first()
        .map_or(false, |b| get_utf8_char_width(*b).is_some())
    {
        return 0;
    }

    let mut trailing_slice = 1;
    while trailing_slice < 4
        && trailing_slice < text.len()
        && is_valid_utf8_continuation(text[text.len() - trailing_slice])
    {
        trailing_slice += 1;
    }

    if text.len() < trailing_slice {
        return 0;
    }

    let tail_start = text.len() - trailing_slice;
    let tail_width = get_utf8_char_width(text[tail_start]).unwrap_or(0);
    if tail_width > trailing_slice {
        // 末尾字符不完整（需要更多续接字节），截断到该字符之前
        // 错误位置 = 截断点到 text 末尾的距离 + 被截掉的尾部长度
        let remaining = check_whole_text_utf8(&text[..tail_start]);
        return if remaining != 0 {
            remaining + trailing_slice
        } else {
            0
        };
    }
    if tail_width < trailing_slice {
        // 续接字节数超过字符宽度，说明尾部结构异常
        return check_whole_text_utf8(text);
    }

    let check_len = text.len() - trailing_slice;
    let remaining = check_whole_text_utf8(&text[..check_len]);
    if remaining != 0 {
        remaining + trailing_slice
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
    /// UTF-32 小端
    Utf32Le,
    /// UTF-32 大端
    Utf32Be,
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
/// - UTF-32LE（BOM: FF FE 00 00）
/// - UTF-32BE（BOM: 00 00 FE FF）
/// - ASCII（无非ASCII字节）
pub fn detect_encoding(data: &[u8]) -> TextEncoding {
    // 检查BOM标记
    if data.len() >= 4 {
        if data[0..4] == [0xFF, 0xFE, 0x00, 0x00] {
            return TextEncoding::Utf32Le;
        }
        if data[0..4] == [0x00, 0x00, 0xFE, 0xFF] {
            return TextEncoding::Utf32Be;
        }
    }
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
#[allow(clippy::result_unit_err)]
pub fn convert_to_utf8(data: &[u8], encoding: &TextEncoding) -> std::result::Result<Vec<u8>, ()> {
    match encoding {
        TextEncoding::Utf16Le => {
            // 剥离 UTF-16LE BOM (FF FE) 后再解码，与 UTF-32 路径行为一致
            let data = if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
                &data[2..]
            } else {
                data
            };
            let (decoded, _, _) = UTF_16LE.decode(data);
            Ok(decoded.into_owned().into_bytes())
        }
        TextEncoding::Utf16Be => {
            // 剥离 UTF-16BE BOM (FE FF) 后再解码
            let data = if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
                &data[2..]
            } else {
                data
            };
            let (decoded, _, _) = UTF_16BE.decode(data);
            Ok(decoded.into_owned().into_bytes())
        }
        TextEncoding::Utf32Le => convert_utf32_to_utf8(data, true),
        TextEncoding::Utf32Be => convert_utf32_to_utf8(data, false),
        _ => Ok(data.to_vec()),
    }
}

/// 将 UTF-32 数据转换为 UTF-8 字节
fn convert_utf32_to_utf8(data: &[u8], little_endian: bool) -> std::result::Result<Vec<u8>, ()> {
    let mut output = String::new();
    let mut chunks = data.chunks_exact(4);

    for (index, chunk) in chunks.by_ref().enumerate() {
        let value = if little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };

        if index == 0 && value == 0xFEFF {
            continue;
        }

        let Some(ch) = char::from_u32(value) else {
            return Err(());
        };
        output.push(ch);
    }

    if !chunks.remainder().is_empty() {
        return Err(());
    }

    Ok(output.into_bytes())
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

    #[test]
    fn test_utf8_char_count_reports_consumed_bytes() {
        assert_eq!(get_utf8_char_count_with_offset("a你b".as_bytes()), (3, 5));
        assert_eq!(get_utf8_char_count_with_offset(b"ok\xfftail"), (2, 2));
        assert_eq!(get_utf8_char_count_with_offset(&[0xE4, 0xBD]), (0, 0));
    }

    #[test]
    fn test_utf32_bom_detection() {
        assert_eq!(
            detect_encoding(&[0xFF, 0xFE, 0x00, 0x00, b'a', 0, 0, 0]),
            TextEncoding::Utf32Le
        );
        assert_eq!(
            detect_encoding(&[0x00, 0x00, 0xFE, 0xFF, 0, 0, 0, b'a']),
            TextEncoding::Utf32Be
        );
    }

    #[test]
    fn test_utf32_conversion_skips_bom() {
        let utf32le = [0xFF, 0xFE, 0x00, 0x00, b'a', 0, 0, 0, b'\n', 0, 0, 0];
        let utf32be = [0x00, 0x00, 0xFE, 0xFF, 0, 0, 0, b'a', 0, 0, 0, b'\n'];

        assert_eq!(
            convert_to_utf8(&utf32le, &TextEncoding::Utf32Le).unwrap(),
            b"a\n"
        );
        assert_eq!(
            convert_to_utf8(&utf32be, &TextEncoding::Utf32Be).unwrap(),
            b"a\n"
        );
    }

    #[test]
    fn test_utf8_overlong_rejected() {
        // 2 字节 overlong: C0 80 编码 U+0000（应 1 字节）
        assert!(!check_utf8_char(&[0xC0, 0x80], 2));
        assert!(check_whole_text_utf8(&[0xC0, 0x80]) > 0);

        // 3 字节 overlong: E0 80 80 编码 U+0000
        assert!(!check_utf8_char(&[0xE0, 0x80, 0x80], 3));
        assert!(check_whole_text_utf8(&[0xE0, 0x80, 0x80]) > 0);

        // 4 字节 overlong: F0 80 80 80 编码 U+0000
        assert!(!check_utf8_char(&[0xF0, 0x80, 0x80, 0x80], 4));
        assert!(check_whole_text_utf8(&[0xF0, 0x80, 0x80, 0x80]) > 0);
    }

    #[test]
    fn test_utf8_surrogate_pair_rejected() {
        // U+D800 代理对: ED A0 80
        assert!(!check_utf8_char(&[0xED, 0xA0, 0x80], 3));
        assert!(check_whole_text_utf8(&[0xED, 0xA0, 0x80]) > 0);

        // U+DFFF 代理对: ED BF BF
        assert!(!check_utf8_char(&[0xED, 0xBF, 0xBF], 3));
        assert!(check_whole_text_utf8(&[0xED, 0xBF, 0xBF]) > 0);

        // 合法 3 字节（非代理）: ED 9F BF = U+07FF 边界
        assert!(check_utf8_char(&[0xED, 0x9F, 0xBF], 3));
    }

    #[test]
    fn test_utf8_out_of_range_rejected() {
        // U+110000: F4 90 80 80
        assert!(!check_utf8_char(&[0xF4, 0x90, 0x80, 0x80], 4));
        assert!(check_whole_text_utf8(&[0xF4, 0x90, 0x80, 0x80]) > 0);

        // F5 开头总是超范围
        assert!(!check_utf8_char(&[0xF5, 0x80, 0x80, 0x80], 4));

        // 合法最大码点 U+10FFFF: F4 8F BF BF
        assert!(check_utf8_char(&[0xF4, 0x8F, 0xBF, 0xBF], 4));
    }

    #[test]
    fn test_convert_to_utf8_utf16_strips_bom() {
        // UTF-16LE with BOM: FF FE + 'a' (61 00) + '\n' (0A 00)
        let utf16le_bom = [0xFF, 0xFE, 0x61, 0x00, 0x0A, 0x00];
        // UTF-16LE without BOM
        let utf16le_raw = [0x61, 0x00, 0x0A, 0x00];

        let result_bom = convert_to_utf8(&utf16le_bom, &TextEncoding::Utf16Le).unwrap();
        let result_raw = convert_to_utf8(&utf16le_raw, &TextEncoding::Utf16Le).unwrap();
        assert_eq!(result_bom, result_raw);
        assert_eq!(result_bom, b"a\n");

        // UTF-16BE with BOM: FE FF + 'a' (00 61) + '\n' (00 0A)
        let utf16be_bom = [0xFE, 0xFF, 0x00, 0x61, 0x00, 0x0A];
        let utf16be_raw = [0x00, 0x61, 0x00, 0x0A];

        let result_bom = convert_to_utf8(&utf16be_bom, &TextEncoding::Utf16Be).unwrap();
        let result_raw = convert_to_utf8(&utf16be_raw, &TextEncoding::Utf16Be).unwrap();
        assert_eq!(result_bom, result_raw);
        assert_eq!(result_bom, b"a\n");
    }

    #[test]
    fn test_check_extract_utf8_error_position() {
        // 构造 10+ 字节文本，中间有非法 UTF-8 字节 0xFF
        let mut text = vec![b'a'; 5];
        text.push(0xFF); // 非法字节
        text.extend_from_slice(&[b'b'; 5]);

        let result = check_extract_text_utf8(&text);
        assert!(result > 0, "应检测到非法字节，返回值 > 0");
        // 错误位置 = text.len() - error_pos = 11 - 5 = 6
        assert_eq!(result, 6);
    }
}
