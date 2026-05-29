//! UTF-16LE 和 UTF-32LE 综合验证测试
//!
//! 验证项：
//! 1. BOM 检测正确
//! 2. 行偏移为原始字节偏移
//! 3. restart_scan 指定模式正确扫描
//! 4. convert_to_utf8 BOM 剥离一致

use fjiffyldg::{convert_to_utf8, detect_encoding, Fjiffyldg, TextEncoding, UtfMode};
use std::io::Write;
use tempfile::NamedTempFile;

// ── 辅助：构造 UTF-16LE 文件 ────────────────────────────────────────
// 内容: "alpha\nbeta\n" （\n = 0A 00 in UTF-16LE）
fn make_utf16le_file() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    // BOM: FF FE
    f.write_all(&[0xFF, 0xFE]).unwrap();
    // 'a' 00 'l' 00 'p' 00 'h' 00 'a' 00  0A 00
    f.write_all(&[
        b'a', 0x00, b'l', 0x00, b'p', 0x00, b'h', 0x00, b'a', 0x00, 0x0A, 0x00,
    ])
    .unwrap();
    // 'b' 00 'e' 00 't' 00 'a' 00  0A 00
    f.write_all(&[b'b', 0x00, b'e', 0x00, b't', 0x00, b'a', 0x00, 0x0A, 0x00])
        .unwrap();
    f.flush().unwrap();
    f
}

// ── 辅助：构造 UTF-32LE 文件 ────────────────────────────────────────
// 内容: "alpha\nbeta\n"
fn make_utf32le_file() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    // BOM: FF FE 00 00
    f.write_all(&[0xFF, 0xFE, 0x00, 0x00]).unwrap();
    // 'a' 00 00 00  ... 'alpha' then \n 00 00 00
    for ch in ['a', 'l', 'p', 'h', 'a', '\n'] {
        let cp = ch as u32;
        f.write_all(&cp.to_le_bytes()).unwrap();
    }
    for ch in ['b', 'e', 't', 'a', '\n'] {
        let cp = ch as u32;
        f.write_all(&cp.to_le_bytes()).unwrap();
    }
    f.flush().unwrap();
    f
}

// ── 1. BOM 检测 ──────────────────────────────────────────────────────

#[test]
fn test_bom_detection_utf16le() {
    let f = make_utf16le_file();
    let data = std::fs::read(f.path()).unwrap();
    assert_eq!(
        detect_encoding(&data),
        TextEncoding::Utf16Le,
        "UTF-16LE BOM(FF FE)应被检测为Utf16Le"
    );
}

#[test]
fn test_bom_detection_utf32le() {
    let f = make_utf32le_file();
    let data = std::fs::read(f.path()).unwrap();
    assert_eq!(
        detect_encoding(&data),
        TextEncoding::Utf32Le,
        "UTF-32LE BOM(FF FE 00 00)应被检测为Utf32Le"
    );
}

// ── 2. 行偏移为原始字节偏移 ──────────────────────────────────────────

#[test]
fn test_line_offsets_are_raw_byte_offsets_utf16le() {
    let f = make_utf16le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // UTF-16LE "alpha\nbeta\n":
    //   行0: pos=0, BOM(2B)+"alpha"(5×2=10B) = 12B内容, +\n(2B) → 行1起始=14
    //   行1: pos=14, "beta"(4×2=8B) = 8B内容, +\n(2B) → 行2起始=24
    //   行2: pos=24, 空行
    // 注意：BOM字节包含在行0内容中，行偏移是原始文件字节偏移
    assert_eq!(fj.line_count(), 3, "应有3行: alpha, beta, 空行");
    assert_eq!(fj.line_pos(0), 0, "行0起始=0 (含BOM)");
    assert_eq!(fj.line_pos(1), 14, "行1起始=14");
    assert_eq!(fj.line_pos(2), 24, "行2起始=24 (beta+LF后)");
}

#[test]
fn test_line_offsets_are_raw_byte_offsets_utf32le() {
    let f = make_utf32le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // UTF-32LE "alpha\nbeta\n":
    //   行0: pos=0, BOM(4B)+"alpha"(5×4=20B) = 24B内容, +\n(4B) → 行1起始=28
    //   行1: pos=28, "beta"(4×4=16B) = 16B内容, +\n(4B) → 行2起始=48
    //   行2: pos=48, 空行
    // 注意：BOM字节包含在行0内容中，行偏移是原始文件字节偏移
    assert_eq!(fj.line_count(), 3, "应有3行: alpha, beta, 空行");
    assert_eq!(fj.line_pos(0), 0, "行0起始=0 (含BOM)");
    assert_eq!(fj.line_pos(1), 28, "行1起始=28");
    assert_eq!(fj.line_pos(2), 48, "行2起始=48 (beta+LF后)");
}

#[test]
fn test_line_lengths_raw_bytes_utf16le() {
    let f = make_utf16le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 行0: BOM(2B) + "alpha"(10B) = 12B (不含LF)
    // 行1: "beta"(8B) (不含LF)
    // 行2: 空行 = 0
    assert_eq!(
        fj.line_length(0),
        12,
        "alpha行长度=12 (BOM 2B + 5 chars × 2B)"
    );
    assert_eq!(fj.line_length(1), 8, "beta行长度=8 (4 chars × 2B)");
    assert_eq!(fj.line_length(2), 0, "空行长度=0");
}

#[test]
fn test_line_lengths_raw_bytes_utf32le() {
    let f = make_utf32le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 行0: BOM(4B) + "alpha"(20B) = 24B (不含LF)
    // 行1: "beta"(16B) (不含LF)
    // 行2: 空行 = 0
    assert_eq!(
        fj.line_length(0),
        24,
        "alpha行长度=24 (BOM 4B + 5 chars × 4B)"
    );
    assert_eq!(fj.line_length(1), 16, "beta行长度=16 (4 chars × 4B)");
    assert_eq!(fj.line_length(2), 0, "空行长度=0");
}

// ── 3. restart_scan 指定模式正确扫描 ─────────────────────────────────

#[test]
fn test_restart_scan_utf16le_explicit_mode() {
    let f = make_utf16le_file();
    let fj = Fjiffyldg::new();
    // 初始以 Default 加载（此时自动检测也会生效，但显式指定模式）
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 使用 restart_scan 显式指定 UTF-16LE 重新扫描
    fj.restart_scan(0, UtfMode::Utf16Le).unwrap();
    fj.wait_scan();

    assert_eq!(fj.line_count(), 3, "restart_scan(Utf16Le)应得到3行");
    assert_eq!(fj.line_pos(0), 0, "restart_scan后行0起始=0 (含BOM)");
    assert_eq!(fj.line_pos(1), 14, "restart_scan后行1起始=14");
    assert_eq!(fj.utf_mode(), UtfMode::Utf16Le, "utf_mode应为Utf16Le");
}

#[test]
fn test_restart_scan_utf32le_explicit_mode() {
    let f = make_utf32le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    fj.restart_scan(0, UtfMode::Utf32Le).unwrap();
    fj.wait_scan();

    assert_eq!(fj.line_count(), 3, "restart_scan(Utf32Le)应得到3行");
    assert_eq!(fj.line_pos(0), 0, "restart_scan后行0起始=0 (含BOM)");
    assert_eq!(fj.line_pos(1), 28, "restart_scan后行1起始=28");
    assert_eq!(fj.utf_mode(), UtfMode::Utf32Le, "utf_mode应为Utf32Le");
}

#[test]
fn test_restart_scan_from_offset_utf16le() {
    let f = make_utf16le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 从行1起始(偏移14)重新扫描
    fj.restart_scan(14, UtfMode::Utf16Le).unwrap();
    fj.wait_scan();

    // 从偏移14开始扫描：应有2行（beta + 空行）
    assert_eq!(fj.line_count(), 2, "从offset=14重新扫描应有2行");
    // 行0起始 = 14 (base_offset)
    assert_eq!(fj.line_pos(0), 14, "offset=14后行0起始=14");
}

#[test]
fn test_restart_scan_from_offset_utf32le() {
    let f = make_utf32le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 从行1起始(偏移28)重新扫描
    fj.restart_scan(28, UtfMode::Utf32Le).unwrap();
    fj.wait_scan();

    assert_eq!(fj.line_count(), 2, "从offset=28重新扫描应有2行");
    assert_eq!(fj.line_pos(0), 28, "offset=28后行0起始=28");
}

// ── 4. convert_to_utf8 BOM 剥离一致 ─────────────────────────────────

#[test]
fn test_convert_to_utf8_bom_stripped_utf16le() {
    let f = make_utf16le_file();
    let data = std::fs::read(f.path()).unwrap();

    // 整文件转换（含 BOM）
    let utf8_all = convert_to_utf8(&data, &TextEncoding::Utf16Le).unwrap();
    let text_all = String::from_utf8(utf8_all).unwrap();
    assert_eq!(
        text_all, "alpha\nbeta\n",
        "convert_to_utf8(含BOM)应得到正确文本"
    );

    // 剥离 BOM 后转换
    let utf8_no_bom = convert_to_utf8(&data[2..], &TextEncoding::Utf16Le).unwrap();
    let text_no_bom = String::from_utf8(utf8_no_bom).unwrap();
    assert_eq!(
        text_no_bom, "alpha\nbeta\n",
        "convert_to_utf8(无BOM)也应得到相同文本"
    );

    // 一致性：两者结果应完全相同
    assert_eq!(text_all, text_no_bom, "BOM剥离前后结果应一致");
}

#[test]
fn test_convert_to_utf8_bom_stripped_utf32le() {
    let f = make_utf32le_file();
    let data = std::fs::read(f.path()).unwrap();

    // 整文件转换（含 BOM）
    let utf8_all = convert_to_utf8(&data, &TextEncoding::Utf32Le).unwrap();
    let text_all = String::from_utf8(utf8_all).unwrap();
    assert_eq!(
        text_all, "alpha\nbeta\n",
        "convert_to_utf8(含BOM)应得到正确文本"
    );

    // 剥离 BOM 后转换
    let utf8_no_bom = convert_to_utf8(&data[4..], &TextEncoding::Utf32Le).unwrap();
    let text_no_bom = String::from_utf8(utf8_no_bom).unwrap();
    assert_eq!(
        text_no_bom, "alpha\nbeta\n",
        "convert_to_utf8(无BOM)也应得到相同文本"
    );

    // 一致性：两者结果应完全相同
    assert_eq!(text_all, text_no_bom, "BOM剥离前后结果应一致");
}

#[test]
fn test_convert_to_utf8_no_extra_bom_bytes_utf16le() {
    let data = [0xFF, 0xFE, b'h', 0x00, b'i', 0x00]; // "hi" with BOM
    let result = convert_to_utf8(&data, &TextEncoding::Utf16Le).unwrap();
    assert_eq!(result, b"hi", "不应在UTF-8结果中包含BOM字节");
    assert_ne!(result[0], 0xEF, "不应有UTF-8 BOM (EF BB BF)");
}

#[test]
fn test_convert_to_utf8_no_extra_bom_bytes_utf32le() {
    let data = [0xFF, 0xFE, 0x00, 0x00, b'h', 0, 0, 0, b'i', 0, 0, 0]; // "hi" UTF-32LE with BOM
    let result = convert_to_utf8(&data, &TextEncoding::Utf32Le).unwrap();
    assert_eq!(result, b"hi", "不应在UTF-8结果中包含BOM字节");
    assert_ne!(result[0], 0xEF, "不应有UTF-8 BOM (EF BB BF)");
}

// ── 5. 读取原始字节并转换验证 ────────────────────────────────────────

#[test]
fn test_read_raw_then_convert_utf16le() {
    let f = make_utf16le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 行0原始字节: pos=0, length=12 (含BOM)
    let raw = fj.read(fj.line_pos(0), fj.line_length(0) as usize).unwrap();
    assert_eq!(raw.len(), 12, "alpha行原始字节=12 (含BOM)");

    // 转换为 UTF-8（convert_to_utf8自动剥离BOM）
    let utf8 = convert_to_utf8(&raw, &TextEncoding::Utf16Le).unwrap();
    assert_eq!(utf8, b"alpha", "alpha行转UTF-8后='alpha'");
}

#[test]
fn test_read_raw_then_convert_utf32le() {
    let f = make_utf32le_file();
    let fj = Fjiffyldg::new();
    fj.load_and_scan(f.path()).unwrap();
    fj.wait_scan();

    // 行0原始字节: pos=0, length=24 (含BOM)
    let raw = fj.read(fj.line_pos(0), fj.line_length(0) as usize).unwrap();
    assert_eq!(raw.len(), 24, "alpha行原始字节=24 (含BOM)");

    // 转换为 UTF-8（convert_to_utf8自动剥离BOM）
    let utf8 = convert_to_utf8(&raw, &TextEncoding::Utf32Le).unwrap();
    assert_eq!(utf8, b"alpha", "alpha行转UTF-8后='alpha'");
}
