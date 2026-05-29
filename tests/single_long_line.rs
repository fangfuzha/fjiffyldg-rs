//! 验证单行 ≥4KB 无换行文件的各项行为：
//! - line_count == 1
//! - read_line(0) 截断到 4KB
//! - read_line_cut(0) 截断 + index 推进
//! - read_to_end_of_line(0, 0, 0) 截断到 4KB
//! - line_at_pos(file_size - 1) == 0

use fjiffyldg::Fjiffyldg;
use std::io::Write;
use tempfile::NamedTempFile;

const KB: usize = 1024;
const CRITICAL: usize = 4 * KB; // 4096 — 与 src/file.rs 中 CRITICAL_LONGLINE_LEN 一致

/// 创建一个包含 8KB 单行（无换行符）的临时文件
fn create_8kb_single_line_file() -> NamedTempFile {
    let mut temp = NamedTempFile::new().unwrap();
    let data = vec![b'A'; 8 * KB]; // 8192 字节，无换行
    temp.write_all(&data).unwrap();
    temp
}

#[test]
fn test_single_long_line_all_properties() {
    let temp = create_8kb_single_line_file();
    let file_size = std::fs::metadata(temp.path()).unwrap().len() as i64;
    assert_eq!(file_size, 8 * KB as i64, "预设文件大小应为 8KB");

    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    // ── 1. line_count == 1 ──
    let lc = fjiff.line_count();
    assert_eq!(lc, 1, "单行无换行文件 line_count 应为 1，实际为 {}", lc);
    println!("✅ line_count = {}", lc);

    // ── 2. read_line(0) 截断到 4KB ──
    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 0usize; // 0 → 默认上限 4KB
    let data = fjiff.read_line(0, &mut bpos, &mut epos, &mut len);
    assert!(data.is_some(), "read_line(0) 应返回 Some");
    let data = data.unwrap();
    assert_eq!(bpos, 0, "read_line bpos 应为 0");
    assert_eq!(epos, CRITICAL as i64, "read_line epos 应为 {}", CRITICAL);
    assert_eq!(len, CRITICAL, "read_line len 应为 {}", CRITICAL);
    assert_eq!(data.len(), CRITICAL, "read_line 数据长度应为 {}", CRITICAL);
    assert!(data.iter().all(|&b| b == b'A'), "read_line 数据内容应全为 'A'");
    println!("✅ read_line(0): bpos={}, epos={}, len={}, data.len()={}", bpos, epos, len, data.len());

    // ── 3. read_line_cut(0) 截断 + index 推进 ──
    let mut index = 0i64;
    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 0usize; // 0 → 使用 BUFFER_SIZE=128KB，但超长行仍截断到 4KB
    let data = fjiff.read_line_cut(&mut index, &mut bpos, &mut epos, &mut len);
    assert!(data.is_some(), "read_line_cut(0) 应返回 Some");
    let data = data.unwrap();
    assert_eq!(index, 1, "read_line_cut index 应推进到 1（下一行）");
    assert_eq!(bpos, 0, "read_line_cut bpos 应为 0");
    assert_eq!(epos, CRITICAL as i64, "read_line_cut epos 应为 {}", CRITICAL);
    assert_eq!(len, CRITICAL, "read_line_cut len 应为 {}", CRITICAL);
    assert_eq!(data.len(), CRITICAL, "read_line_cut 数据长度应为 {}", CRITICAL);
    assert!(data.iter().all(|&b| b == b'A'), "read_line_cut 数据内容应全为 'A'");
    println!("✅ read_line_cut(0): index={}, bpos={}, epos={}, len={}, data.len()={}", index, bpos, epos, len, data.len());

    // ── 4. read_to_end_of_line(0, 0, 0) 截断到 4KB ──
    let mut len = 0usize; // 0 → 默认上限 4KB
    let data = fjiff.read_to_line_end(0, 0, &mut len);
    assert!(data.is_some(), "read_to_end_of_line(0,0,0) 应返回 Some");
    let data = data.unwrap();
    // 对于无换行的单行，line_end = file_size，所以实际读取量 = min(4KB, file_size) = 4KB
    assert_eq!(len, CRITICAL, "read_to_end_of_line len 应为 {}", CRITICAL);
    assert_eq!(data.len(), CRITICAL, "read_to_end_of_line 数据长度应为 {}", CRITICAL);
    assert!(data.iter().all(|&b| b == b'A'), "read_to_end_of_line 数据内容应全为 'A'");
    println!("✅ read_to_end_of_line(0,0,0): len={}, data.len()={}", len, data.len());

    // ── 5. line_at_pos(file_size - 1) == 0 ──
    let line = fjiff.line_at_pos(file_size - 1);
    assert_eq!(line, 0, "line_at_pos(file_size-1) 应为 0，实际为 {}", line);
    println!("✅ line_at_pos({}) = {}", file_size - 1, line);

    println!("\n🎉 单行≥4KB无换行文件：全部 5 项验证通过");
}
