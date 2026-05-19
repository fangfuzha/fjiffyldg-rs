use fjiffyldg::Fjiffyldg;

fn main() {
    println!("=== Fjiffyldg Rust 版本示例 ===\n");

    let fjiffyldg = Fjiffyldg::new();

    let test_file = "test.txt";

    std::fs::write(test_file, "Hello, World!\nLine 2 here.\nLine 3 content.\n").unwrap();

    println!("1. 加载文件并构建行索引:");
    match fjiffyldg.load_and_scan(test_file) {
        Ok(_) => println!("   ✓ 文件加载成功"),
        Err(e) => {
            println!("   ✗ 文件加载失败: {:?}", e);
            return;
        }
    }

    println!("\n2. 文件信息:");
    println!("   文件大小: {} 字节", fjiffyldg.file_size());
    println!("   总行数: {}", fjiffyldg.line_count());

    println!("\n3. 行索引信息:");
    for i in 0..fjiffyldg.line_count() {
        let pos = fjiffyldg.line_pos(i);
        let len = fjiffyldg.line_length(i);
        println!("   行 {}: 偏移={}, 长度={}", i, pos, len);
    }

    println!("\n4. 读取指定位置数据:");
    if let Some(data) = fjiffyldg.read(0, 5) {
        println!("   位置 0-5: {:?}", std::str::from_utf8(&data).unwrap_or("无效UTF-8"));
    }

    println!("\n5. 按行读取:");
    let mut bpos = 0;
    let mut epos = 0;
    let mut len = 100;
    if let Some(data) = fjiffyldg.read_line(0, &mut bpos, &mut epos, &mut len) {
        println!("   行 0: {:?}", std::str::from_utf8(&data).unwrap_or("无效UTF-8"));
    }

    println!("\n6. 查找位置所在行:");
    let line = fjiffyldg.line_at_pos(20);
    println!("   位置 20 位于行: {}", line);

    println!("\n7. 编码检测:");
    let sample = "你好世界".as_bytes();
    let is_ascii = fjiffyldg::check_text_ascii(sample) == 0;
    let is_utf8 = fjiffyldg::check_whole_text_utf8(sample) == 0;
    println!("   ASCII: {}, UTF-8: {}", is_ascii, is_utf8);
    println!("   UTF-8 字符数: {}", fjiffyldg::get_utf8_char_count(sample));

    std::fs::remove_file(test_file).ok();

    println!("\n=== 示例完成 ===");
}