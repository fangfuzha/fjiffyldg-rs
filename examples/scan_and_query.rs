//! 大文件扫描与行查询示例
//!
//! 演示：
//! - 异步加载扫描与等待
//! - 行索引随机查询
//! - 按行读取与按位置读取
//! - 取消后台扫描
//! - 重新扫描（restart_scan）

use fjiffyldg::{Fjiffyldg, UtfMode};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. 准备测试文件 ──────────────────────────────────────
    let path = "scan_demo.txt";
    let mut file = std::fs::File::create(path)?;
    for i in 0..100_000 {
        writeln!(file, "这是第 {i} 行，包含一些示例文本内容。")?;
    }
    println!("已创建测试文件: {path} (100,000 行)\n");

    // ── 2. 加载并扫描 ────────────────────────────────────────
    let fjiff = Fjiffyldg::new();
    fjiff.load_and_scan(path)?;

    // load_and_scan 立即返回，扫描在后台进行
    println!("文件已加载，后台扫描中...");
    println!("  正在扫描: {}", fjiff.is_scanning());

    // 等待扫描完成（阻塞）
    fjiff.wait_scan();
    println!("  扫描完成，总行数: {}", fjiff.line_count());
    println!("  文件大小: {} 字节\n", fjiff.file_size());

    // ── 3. 行索引随机查询 ────────────────────────────────────
    println!("行索引查询:");
    let indices = [0, 1, 49_999, 99_999];
    for &i in &indices {
        let pos = fjiff.line_pos(i);
        let len = fjiff.line_length(i);
        println!("  行 {i:>6}: 偏移={pos:>10}, 长度={len}");
    }

    // 反向查询：根据字节位置查找所在行
    let test_pos = fjiff.line_pos(50_000);
    let found_line = fjiff.line_at_pos(test_pos);
    println!("\n  位置 {test_pos} 对应行: {found_line}");
    println!(
        "  越界查询 line_at_pos(file_size+1): {}",
        fjiff.line_at_pos(fjiff.file_size() + 1)
    );

    // ── 4. 按位置读取 ────────────────────────────────────────
    println!("\n按位置读取 (read):");
    if let Some(data) = fjiff.read(0, 50) {
        println!("  前 50 字节: {:?}", String::from_utf8_lossy(&data));
    }

    // 读取文件末尾
    let file_size = fjiff.file_size();
    if let Some(data) = fjiff.read(file_size - 20, 100) {
        println!("  末尾 20 字节: {:?}", String::from_utf8_lossy(&data));
    }

    // ── 5. 按行读取 ──────────────────────────────────────────
    println!("\n按行读取 (read_line):");
    let mut bpos = 0i64;
    let mut epos = 0i64;
    let mut len = 0usize; // 0 = 默认最多 4KB
    if let Some(data) = fjiff.read_line(42, &mut bpos, &mut epos, &mut len) {
        println!(
            "  行 42: bpos={bpos}, epos={epos}, 实际长度={len}, 内容={:?}",
            String::from_utf8_lossy(&data)
        );
    }

    // ── 6. 批量行读取 (read_line_cut) ────────────────────────
    println!("\n批量行读取 (read_line_cut):");
    let mut idx = 10;
    let mut bpos = 0i64;
    let mut epos = 0i64;
    let mut len = 200; // 预算 200 字节
    if let Some(data) = fjiff.read_line_cut(&mut idx, &mut bpos, &mut epos, &mut len) {
        println!("  起始行 10, 预算 200 字节: 实际读取 {len} 字节, 索引推进到行 {idx}");
        println!("  范围: [{bpos}, {epos})");
        println!(
            "  内容前 100 字节: {:?}",
            String::from_utf8_lossy(&data[..100.min(data.len())])
        );
    }

    // ── 7. 读取到行尾 (read_to_line_end) ─────────────────────
    println!("\n读取到行尾 (read_to_line_end):");
    let line_start = fjiff.line_pos(0);
    let mid = line_start + 5; // 从行首偏移 5 字节开始
    let mut read_len = 0usize;
    if let Some(data) = fjiff.read_to_line_end(0, mid, &mut read_len) {
        println!(
            "  行 0 从偏移 {mid} 到行尾: {read_len} 字节, 内容={:?}",
            String::from_utf8_lossy(&data)
        );
    }

    // ── 8. 取消扫描演示 ──────────────────────────────────────
    println!("\n取消扫描演示:");
    let fjiff2 = Fjiffyldg::new();
    fjiff2.load_and_scan(path)?;
    println!("  正在扫描: {}", fjiff2.is_scanning());
    fjiff2.request_stop_scan();
    println!("  已取消，正在扫描: {}", fjiff2.is_scanning());
    println!("  行数（清空后）: {}", fjiff2.line_count());

    // ── 9. 重新扫描 ──────────────────────────────────────────
    println!("\n重新扫描 (restart_scan):");
    fjiff2.load_and_scan(path)?;
    fjiff2.wait_scan();
    println!("  重新加载后行数: {}", fjiff2.line_count());

    // 从偏移量 100 开始重新扫描，使用默认编码模式
    fjiff2.restart_scan(100, UtfMode::Default)?;
    fjiff2.wait_scan();
    println!("  从偏移 100 重扫后行数: {}", fjiff2.line_count());

    // ── 10. load_status 诊断 ─────────────────────────────────
    println!("\nload_status 诊断:");
    let fjiff3 = Fjiffyldg::new();
    println!("  未加载: {:?}", fjiff3.load_status());

    let missing = "不存在的文件.txt";
    let _ = fjiff.load(missing);
    println!("  加载失败后: {:?}", fjiff.load_status());

    // 清理
    std::fs::remove_file(path).ok();
    println!("\n示例完成。");
    Ok(())
}
