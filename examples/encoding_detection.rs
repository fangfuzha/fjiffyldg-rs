//! 编码检测与文本校验示例
//!
//! 演示：
//! - `detect_encoding`：BOM 自动检测编码
//! - `check_text_ascii`：SIMD 加速 ASCII 检查
//! - `check_whole_text_utf8`：完整 UTF-8 校验
//! - `check_extract_text_utf8`：抽取式 UTF-8 检查
//! - `get_utf8_char_count` / `get_utf8_char_count_with_offset`：UTF-8 字符计数
//! - `UtfMode`：指定定宽编码模式扫描文件

use fjiffyldg::{
    check_extract_text_utf8, check_text_ascii, check_whole_text_utf8, detect_encoding,
    get_utf8_char_count, get_utf8_char_count_with_offset, Fjiffyldg, TextEncoding, UtfMode,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. detect_encoding：BOM 检测 ─────────────────────────
    println!("1. detect_encoding (BOM 检测):");
    let samples: &[(&[u8], &str)] = &[
        (b"hello world", "纯 ASCII"),
        ("你好世界".as_bytes(), "UTF-8 无 BOM"),
        (&[0xEF, 0xBB, 0xBF, b'h', b'i'], "UTF-8 with BOM"),
        (&[0xFF, 0xFE, b'h', 0x00, b'i', 0x00], "UTF-16LE BOM"),
        (&[0xFE, 0xFF, 0x00, b'h', 0x00, b'i'], "UTF-16BE BOM"),
        (
            &[0xFF, 0xFE, 0x00, 0x00, b'h', 0x00, 0x00, 0x00],
            "UTF-32LE BOM",
        ),
        (
            &[0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, b'h'],
            "UTF-32BE BOM",
        ),
    ];
    for (data, label) in samples {
        println!("   {label}: {:?}", detect_encoding(data));
    }

    // ── 2. check_text_ascii：SIMD ASCII 检查 ─────────────────
    println!("\n2. check_text_ascii:");
    let texts: &[(&[u8], &str)] = &[
        (b"hello", "纯 ASCII"),
        (b"abc\x80def", "含非 ASCII"),
        ("你好".as_bytes(), "纯中文"),
        (b"", "空文本"),
    ];
    for (data, label) in texts {
        let result = check_text_ascii(data);
        println!("   {label}: 剩余非 ASCII 长度 = {result}");
    }

    // ── 3. check_whole_text_utf8：完整 UTF-8 校验 ────────────
    println!("\n3. check_whole_text_utf8:");
    let texts: &[(&[u8], &str)] = &[
        (b"hello", "纯 ASCII"),
        ("你好世界".as_bytes(), "有效 UTF-8"),
        (&[0xC0, 0x80], "过长编码 (invalid)"),
        (&[0xE4, 0xBD], "截断的 UTF-8"),
        (b"", "空文本"),
    ];
    for (data, label) in texts {
        let result = check_whole_text_utf8(data);
        let status = if result == 0 {
            "✓ 有效"
        } else {
            "✗ 无效"
        };
        println!("   {label}: 剩余无效长度 = {result} ({status})");
    }

    // ── 4. check_extract_text_utf8：抽取式检查 ───────────────
    println!("\n4. check_extract_text_utf8 (适合流式片段):");
    // 模拟流式数据中截取的片段（首尾可能截断多字节字符）
    let full = "你好世界Hello".as_bytes();
    let slice = &full[1..13]; // 截断了"你"的第一个字节和最后一个字节
    println!("   完整文本: {:?}", String::from_utf8_lossy(full));
    println!("   截取片段: {:?}", String::from_utf8_lossy(slice));
    println!("   check_whole_text_utf8: {}", check_whole_text_utf8(slice));
    println!(
        "   check_extract_text_utf8: {} (忽略首尾截断)",
        check_extract_text_utf8(slice)
    );

    // ── 5. get_utf8_char_count：字符计数 ─────────────────────
    println!("\n5. get_utf8_char_count:");
    let samples: &[(&[u8], &str)] = &[
        (b"hello", "5 个 ASCII 字符"),
        ("你好".as_bytes(), "2 个中文字符"),
        ("Hello你好World".as_bytes(), "混合文本"),
    ];
    for (data, label) in samples {
        println!("   {label}: 字符数 = {}", get_utf8_char_count(data));
    }

    // ── 6. get_utf8_char_count_with_offset：带消费字节数 ─────
    println!("\n6. get_utf8_char_count_with_offset (遇非法字节停止):");
    // "a你好" + 非法字节 0xFF + "tail"
    let mixed = b"a\xe4\xbd\xa0\xe5\xa5\xbd\xfftail";
    let (count, consumed) = get_utf8_char_count_with_offset(mixed);
    println!("   输入: \"a你好\\xfftail\"");
    println!("   有效字符数: {count}");
    println!("   已消费字节: {consumed} (停在非法字节 0xFF 前)");
    println!("   未消费尾部: {:?}", &mixed[consumed..]);

    // ── 7. UtfMode：指定编码扫描文件 ─────────────────────────
    println!("\n7. UtfMode (指定编码扫描):");
    let path = "encoding_demo.txt";
    std::fs::write(path, "Line 1\nLine 2\nLine 3\n")?;

    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(path)?;
    fjiff.wait_scan();
    println!("   默认模式行数: {}", fjiff.line_count());

    // 使用 restart_scan 指定 UTF-16LE 模式重新扫描
    fjiff.restart_scan(0, UtfMode::Utf16Le)?;
    fjiff.wait_scan();
    println!("   UTF-16LE 模式行数: {}", fjiff.line_count());

    // 自动检测 BOM（传入 Default）
    fjiff.restart_scan(0, UtfMode::Default)?;
    fjiff.wait_scan();
    println!("   自动检测模式行数: {}", fjiff.line_count());

    // ── 8. TextEncoding 枚举 ─────────────────────────────────
    println!("\n8. TextEncoding 枚举值:");
    let encodings = [
        TextEncoding::Ascii,
        TextEncoding::Utf8,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Utf32Le,
        TextEncoding::Utf32Be,
        TextEncoding::Unknown,
    ];
    for enc in &encodings {
        println!("   {enc:?}");
    }

    // 清理
    std::fs::remove_file(path).ok();
    println!("\n示例完成。");
    Ok(())
}
