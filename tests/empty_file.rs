//! 验证空文件的 5 项行为：
//! 1. load_and_scan → line_count == 1
//! 2. load → line_count == 0
//! 3. read_line(0) → Some(vec![])
//! 4. read_line_cut(0) → Some(vec![])
//! 5. read_data(0, 0) → Some(vec![])

use fjiffyldg::Fjiffyldg;
use tempfile::NamedTempFile;

/// 创建空临时文件
fn create_empty_file() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}

#[test]
fn test_empty_file_load_and_scan_line_count_is_1() {
    let temp = create_empty_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    let lc = fjiff.line_count();
    assert_eq!(lc, 1, "load_and_scan 空文件 line_count 应为 1，实际为 {}", lc);
    println!("✅ 1. load_and_scan → line_count = {}", lc);
}

#[test]
fn test_empty_file_load_line_count_is_0() {
    let temp = create_empty_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load(temp.path()).unwrap();

    let lc = fjiff.line_count();
    assert_eq!(lc, 0, "load(不扫描) 空文件 line_count 应为 0，实际为 {}", lc);
    println!("✅ 2. load → line_count = {}", lc);
}

#[test]
fn test_empty_file_read_line_returns_empty() {
    let temp = create_empty_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 0usize;
    let data = fjiff.read_line(0, &mut bpos, &mut epos, &mut len);

    assert!(data.is_some(), "read_line(0) 应返回 Some");
    let data = data.unwrap();
    assert!(data.is_empty(), "read_line(0) 数据应为空，实际 len={}", data.len());
    assert_eq!(bpos, 0, "bpos 应为 0，实际为 {}", bpos);
    assert_eq!(epos, 0, "epos 应为 0，实际为 {}", epos);
    assert_eq!(len, 0, "len 应为 0，实际为 {}", len);
    println!("✅ 3. read_line(0) → Some(vec![]), bpos={}, epos={}, len={}", bpos, epos, len);
}

#[test]
fn test_empty_file_read_line_cut_returns_empty() {
    let temp = create_empty_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    let mut index = 0i64;
    let mut bpos = -1i64;
    let mut epos = -1i64;
    let mut len = 0usize;
    let data = fjiff.read_line_cut(&mut index, &mut bpos, &mut epos, &mut len);

    assert!(data.is_some(), "read_line_cut(0) 应返回 Some");
    let data = data.unwrap();
    assert!(data.is_empty(), "read_line_cut(0) 数据应为空，实际 len={}", data.len());
    assert_eq!(bpos, 0, "bpos 应为 0，实际为 {}", bpos);
    assert_eq!(epos, 0, "epos 应为 0，实际为 {}", epos);
    assert_eq!(len, 0, "len 应为 0，实际为 {}", len);
    println!("✅ 4. read_line_cut(0) → Some(vec![]), bpos={}, epos={}, len={}", bpos, epos, len);
}

#[test]
fn test_empty_file_read_data_returns_empty() {
    let temp = create_empty_file();
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(temp.path()).unwrap();
    fjiff.wait_scan();

    let data = fjiff.read(0, 0);

    assert!(data.is_some(), "read_data(0,0) 应返回 Some");
    let data = data.unwrap();
    assert!(data.is_empty(), "read_data(0,0) 数据应为空，实际 len={}", data.len());
    println!("✅ 5. read_data(0,0) → Some(vec![]), len={}", data.len());
}
