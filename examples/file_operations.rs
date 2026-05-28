//! 文件操作示例
//!
//! 演示：
//! - `clone_file`：克隆文件
//! - `save_file`：保存数据到文件
//! - `append_file`：追加数据到文件
//! - `concatenate_files`：合并多个文件
//! - `get_file_size`：获取文件大小

use fjiffyldg::{append_file, clone_file, concatenate_files, get_file_size, save_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join("fjiffyldg_demo");
    std::fs::create_dir_all(&dir)?;

    // ── 1. save_file：保存数据到文件 ─────────────────────────
    println!("1. save_file:");
    let path_a = dir.join("a.txt");
    save_file(&path_a, b"Hello, Fjiffyldg!\n")?;
    println!(
        "   保存成功: {} ({} 字节)",
        path_a.display(),
        get_file_size(&path_a)?
    );

    // 大数据保存（>10MB 自动走 mmap 路径）
    let large_data = vec![b'x'; 11 * 1024 * 1024]; // 11MB
    let path_large = dir.join("large.bin");
    save_file(&path_large, &large_data)?;
    println!(
        "   大文件保存: {} ({} 字节)",
        path_large.display(),
        get_file_size(&path_large)?
    );

    // ── 2. clone_file：克隆文件 ──────────────────────────────
    println!("\n2. clone_file:");
    let path_b = dir.join("b.txt");
    clone_file(&path_a, &path_b)?;
    println!(
        "   克隆 {} -> {} ({} 字节)",
        path_a.display(),
        path_b.display(),
        get_file_size(&path_b)?
    );

    // 验证内容一致
    let content_a = std::fs::read(&path_a)?;
    let content_b = std::fs::read(&path_b)?;
    assert_eq!(content_a, content_b);
    println!("   内容验证: ✓ 一致");

    // ── 3. append_file：追加数据 ─────────────────────────────
    println!("\n3. append_file:");
    let path_c = dir.join("c.txt");
    save_file(&path_c, "第一行\n".as_bytes())?;
    append_file(&path_c, "第二行\n".as_bytes())?;
    append_file(&path_c, "第三行\n".as_bytes())?;
    println!("   追加后大小: {} 字节", get_file_size(&path_c)?);
    println!(
        "   内容: {:?}",
        String::from_utf8_lossy(&std::fs::read(&path_c)?)
    );

    // ── 4. concatenate_files：合并多个文件 ───────────────────
    println!("\n4. concatenate_files:");
    let path_d = dir.join("d.txt");
    save_file(&path_d, "--- D 部分 ---\n".as_bytes())?;
    let path_merged = dir.join("merged.txt");
    save_file(&path_merged, "=== 合并文件 ===\n".as_bytes())?;

    concatenate_files(
        [path_a.as_path(), path_c.as_path(), path_d.as_path()],
        &path_merged,
    )?;
    println!("   合并后大小: {} 字节", get_file_size(&path_merged)?);
    println!("   内容预览:");
    for line in String::from_utf8_lossy(&std::fs::read(&path_merged)?)
        .lines()
        .take(6)
    {
        println!("     {line}");
    }

    // ── 5. get_file_size ─────────────────────────────────────
    println!("\n5. get_file_size:");
    println!("   {}: {} 字节", path_a.display(), get_file_size(&path_a)?);
    println!(
        "   {}: {} 字节",
        path_large.display(),
        get_file_size(&path_large)?
    );

    // 不存在的文件
    match get_file_size("不存在的文件.txt") {
        Err(e) => println!("   不存在的文件: {e}"),
        Ok(size) => println!("   不存在的文件: {size} 字节"),
    }

    // ── 6. 错误码语义演示 ────────────────────────────────────
    println!("\n6. 错误码语义:");
    println!("   clone_file 成功: 0");
    println!("   clone_file 源不存在: -1");

    // 清理
    let _ = std::fs::remove_dir_all(&dir);
    println!("\n示例完成。");
    Ok(())
}
