//! CRLF 行为验证测试
//!
//! 逐项验证：
//! 1. 行长度不含 \r\n
//! 2. read_line 返回不含 \r
//! 3. read_line_cut 行边界正确
//! 4. 跨窗口 \r\n 正确

use fjiffyldg::Fjiffyldg;
use std::io::Write;
use tempfile::NamedTempFile;

/// 创建 CRLF 文件："line1\r\nline2\r\nline3\r\n"
fn create_crlf_file() -> NamedTempFile {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"line1\r\nline2\r\nline3\r\n").unwrap();
    temp
}

/// 创建 CRLF 文件（无末尾换行）："line1\r\nline2\r\nline3"
fn create_crlf_no_trailing() -> NamedTempFile {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"line1\r\nline2\r\nline3").unwrap();
    temp
}

/// 创建混合换行文件："line1\r\nline2\nline3\r\n"
fn create_mixed_ending() -> NamedTempFile {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"line1\r\nline2\nline3\r\n").unwrap();
    temp
}

// ── 1. 行长度不含 \r\n ──

#[test]
fn test_crlf_line_length_excludes_newline() {
    let temp = create_crlf_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    // "line1\r\nline2\r\nline3\r\n" → 4 行（含末尾空行）
    let lc = fjiff.line_count();
    assert_eq!(lc, 4, "CRLF 文件应有 4 行，实际 {}", lc);

    // line0 = "line1" → 长度 5（不含 \r\n）
    assert_eq!(fjiff.line_length(0), 5, "line0 长度应为 5");
    // line1 = "line2" → 长度 5
    assert_eq!(fjiff.line_length(1), 5, "line1 长度应为 5");
    // line2 = "line3" → 长度 5
    assert_eq!(fjiff.line_length(2), 5, "line2 长度应为 5");
    // line3 = "" → 长度 0
    assert_eq!(fjiff.line_length(3), 0, "line3 长度应为 0");

    // 验证 pos 间距：每行 5 字节内容 + 2 字节 CRLF = 7 字节
    assert_eq!(fjiff.line_pos(0), 0, "line0 起始 0");
    assert_eq!(fjiff.line_pos(1), 7, "line1 起始 7");
    assert_eq!(fjiff.line_pos(2), 14, "line2 起始 14");
    assert_eq!(fjiff.line_pos(3), 21, "line3 起始 21");

    println!("✅ 1. 行长度不含 \\r\\n：通过");
}

#[test]
fn test_mixed_ending_line_length_excludes_newline() {
    let temp = create_mixed_ending();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    // "line1\r\nline2\nline3\r\n" → 4 行
    assert_eq!(fjiff.line_count(), 4, "混合换行文件应有 4 行");
    // line0 = "line1" (5) — \r\n = 2字节
    assert_eq!(fjiff.line_length(0), 5);
    // line1 = "line2" (5) — \n = 1字节
    assert_eq!(fjiff.line_length(1), 5);
    // line2 = "line3" (5) — \r\n = 2字节
    assert_eq!(fjiff.line_length(2), 5);
    // line3 = "" (0)
    assert_eq!(fjiff.line_length(3), 0);

    // pos: line0=0, line1=7(5+2), line2=13(7+5+1), line3=20(13+5+2)
    assert_eq!(fjiff.line_pos(0), 0);
    assert_eq!(fjiff.line_pos(1), 7);
    assert_eq!(fjiff.line_pos(2), 13);
    assert_eq!(fjiff.line_pos(3), 20);

    println!("✅ 1b. 混合换行行长度不含换行符：通过");
}

// ── 2. read_line 返回不含 \r ──

#[test]
fn test_crlf_read_line_excludes_cr() {
    let temp = create_crlf_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    for i in 0..3 {
        let mut bpos = -1i64;
        let mut epos = -1i64;
        let mut len = 0usize;
        let data = fjiff.read_line(i, &mut bpos, &mut epos, &mut len);
        assert!(data.is_some(), "read_line({}) 应返回 Some", i);
        let data = data.unwrap();

        // 不应以 \r 结尾
        assert!(
            !data.ends_with(b"\r"),
            "read_line({}) 不应以 \\r 结尾，实际 {:?}",
            i,
            String::from_utf8_lossy(&data)
        );

        // 不应包含 \r 或 \n
        assert!(
            !data.contains(&b'\r'),
            "read_line({}) 不应包含 \\r",
            i
        );
        assert!(
            !data.contains(&b'\n'),
            "read_line({}) 不应包含 \\n",
            i
        );
    }

    println!("✅ 2. read_line 返回不含 \\r：通过");
}

#[test]
fn test_crlf_no_trailing_read_line() {
    let temp = create_crlf_no_trailing();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    // "line1\r\nline2\r\nline3" → 3 行
    assert_eq!(fjiff.line_count(), 3);

    // 最后一行 "line3" 不以 \r 结尾
    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 0usize;
    let data = fjiff.read_line(2, &mut bpos, &mut epos, &mut len).unwrap();
    assert_eq!(data, b"line3", "最后一行内容应为 'line3'");
    assert!(!data.ends_with(b"\r"));

    println!("✅ 2b. 无末尾换行 CRLF 文件 read_line 正确：通过");
}

// ── 3. read_line_cut 行边界正确 ──

#[test]
fn test_crlf_read_line_cut_boundaries() {
    let temp = create_crlf_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    // read_line_cut 从 index=0 开始，预算足够大时应批量读取多行
    let mut index = 0i64;
    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 4096usize; // 足够大

    let data = fjiff.read_line_cut(&mut index, &mut bpos, &mut epos, &mut len);
    assert!(data.is_some(), "read_line_cut 应返回 Some");
    let data = data.unwrap();

    // 应包含 line1\r\nline2\r\nline3\r\n 的全部内容
    // 因为每行都很短，会批量合并
    assert!(
        data.starts_with(b"line1"),
        "数据应以 'line1' 开头"
    );

    // 数据不应在行中间截断
    // 检查 \r\n 成对出现
    let text = String::from_utf8_lossy(&data);
    let cr_count = text.matches('\r').count();
    let lf_count = text.matches('\n').count();
    // CRLF 模式下，\r 和 \n 数量相等
    assert_eq!(
        cr_count, lf_count,
        "CRLF 文件中 \\r 和 \\n 数量应相等: \\r={}, \\n={}",
        cr_count, lf_count
    );

    // bpos 应为 0（从第一行开始）
    assert_eq!(bpos, 0, "bpos 应为 0");

    println!(
        "✅ 3. read_line_cut 行边界正确：通过 (index推进到{}, 数据{}字节)",
        index,
        data.len()
    );
}

#[test]
fn test_crlf_read_line_cut_single_line_budget() {
    let temp = create_crlf_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    // read_line_cut 的 len 是"可容纳字节预算"，索引推进条件为 (next_pos - begin) <= length。
    // 预算=7 时：line0 结束于 pos 7，(7-0)<=7 成立 → index 推进到 1；
    // line1 结束于 pos 14，(14-0)=14 > 7 → 停止。
    // 实际读取从 begin(0) 到 next_pos(14) = "line1\r\nline2\r\n"（14 字节）。
    // 因为读取范围总是延伸到最后一个完整纳入行的下一行起始处。
    let mut index = 0i64;
    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 7usize;

    let data = fjiff.read_line_cut(&mut index, &mut bpos, &mut epos, &mut len);
    assert!(data.is_some());
    let data = data.unwrap();

    // index 应推进到 1（line0 被完整纳入）
    assert_eq!(index, 1, "index 应推进到 1");
    assert_eq!(bpos, 0, "bpos 应为 0");
    // epos = next_pos of line after last included = 14
    assert_eq!(epos, 14, "epos 应为 14（line2 起始）");
    // 实际数据 = "line1\r\nline2\r\n" (14 字节)
    assert_eq!(data.len(), 14, "数据长度应为 14");
    assert_eq!(&data[..7], b"line1\r\n", "前半应为 line1\\r\\n");
    assert_eq!(&data[7..], b"line2\r\n", "后半应为 line2\\r\\n");

    println!("✅ 3b. read_line_cut 预算边界正确：通过 (index={}, len={})", index, data.len());
}

// ── 4. 跨窗口 \r\n 正确 ──
// 这个需要通过 LineIndex 的 build_from_windows_at_cancelable 来测试
// 因为 Fjiffyldg 的 load_and_scan 内部会自动处理窗口

use fjiffyldg::line_index::LineIndex;
use fjiffyldg::UtfMode;
use std::sync::atomic::AtomicBool;

#[test]
fn test_crlf_across_window_boundary() {
    // "abc\r\ndef" — 窗口大小 4 → 第一个窗口 "abc\r"，第二个窗口 "\ndef"
    let data = b"abc\r\ndef";
    let index = LineIndex::new();
    let cancel = AtomicBool::new(false);

    let ok = index.build_from_windows_at_cancelable(
        0,
        data.len() as u64,
        4, // chunk_size = 4 → 强制 \r\n 跨窗口
        UtfMode::Default,
        &cancel,
        |offset, len| Some(data[offset as usize..(offset + len) as usize].to_vec()),
    );
    assert!(ok, "窗口扫描应成功");

    assert_eq!(index.get_line_count(), 2, "应有 2 行");
    assert_eq!(index.get_line_pos(0), 0);
    assert_eq!(index.get_line_pos(1), 5); // "abc\r\n" = 5 字节
    assert_eq!(index.get_line_length(0), 3, "line0='abc' 长度 3");
    assert_eq!(index.get_line_length(1), 3, "line1='def' 长度 3");

    println!("✅ 4. 跨窗口 \\r\\n 正确：通过");
}

#[test]
fn test_crlf_boundary_at_exact_window_edge() {
    // "ab\r\ncd\r\nef" — 窗口大小 5
    // window1: "ab\r\nc" (offset 0..5)
    // window2: "d\r\nef" (offset 5..10)
    // \r\n 在 window1 内部，但第二个 \r\n 跨窗口吗？
    // window1 处理到 pos=4-1=3 → 处理 "ab\r\n"，保留最后1字节 'c'
    // 实际上 window1 len=5, retain=1, process_len=4 → 扫描 0..4 = "ab\r\n"
    // 拼接后 pending = "c" + "d\r\nef" = "cd\r\nef"
    let data = b"ab\r\ncd\r\nef";
    let index = LineIndex::new();
    let cancel = AtomicBool::new(false);

    let ok = index.build_from_windows_at_cancelable(
        0,
        data.len() as u64,
        5,
        UtfMode::Default,
        &cancel,
        |offset, len| Some(data[offset as usize..(offset + len) as usize].to_vec()),
    );
    assert!(ok);

    assert_eq!(index.get_line_count(), 3, "应有 3 行");
    assert_eq!(index.get_line_length(0), 2, "line0='ab'");
    assert_eq!(index.get_line_length(1), 2, "line1='cd'");
    assert_eq!(index.get_line_length(2), 2, "line2='ef'");

    println!("✅ 4b. 窗口边界精确匹配 CRLF：通过");
}

#[test]
fn test_crlf_split_cr_at_window_end() {
    // 强制 \r 在窗口末尾，\n 在下一窗口开头
    // "a\r\nb" 窗口大小 2
    // window1: "a\r" (offset 0..2) → retain 1 → process 1 → 扫描 'a'，保留 '\r'
    // window2: "\nb" (offset 2..4) → pending = "\r" + "\nb" = "\r\nb"
    // → 扫描 "\r\n" → 行边界正确
    let data = b"a\r\nb";
    let index = LineIndex::new();
    let cancel = AtomicBool::new(false);

    let ok = index.build_from_windows_at_cancelable(
        0,
        data.len() as u64,
        2,
        UtfMode::Default,
        &cancel,
        |offset, len| Some(data[offset as usize..(offset + len) as usize].to_vec()),
    );
    assert!(ok, "窗口扫描应成功");

    assert_eq!(index.get_line_count(), 2, "应有 2 行");
    assert_eq!(index.get_line_pos(0), 0);
    assert_eq!(index.get_line_pos(1), 3); // "a\r\n" = 3 字节
    assert_eq!(index.get_line_length(0), 1, "line0='a'");
    assert_eq!(index.get_line_length(1), 1, "line1='b'");

    println!("✅ 4c. \\r 恰在窗口末尾、\\n 在下一窗口开头：通过");
}
