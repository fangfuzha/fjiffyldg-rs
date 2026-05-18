# Fjiffyldg (Rust)

高性能、跨平台的通用文件处理开发库，专为现代大文件高效加载设计。

> 🚀 这是原 C++ [Fjiffyldg 项目](https://github.com/ceepuka/fjiffyldg) 的 Rust 重写版本。

## ✨ 核心特性

- ✅ **跨平台**: Windows / Linux / macOS / Android
- ✅ **超大文件**: 理论限制 < 8EB，实际应用无上限
- ✅ **智能行索引**: 直接定位百万行级文件
- ✅ **内存映射**: 高效大文件访问
- ✅ **编码支持**: UTF-8 / UTF-16 (LE/BE)
- ✅ **高性能**: Rust 零成本抽象
- ✅ **内存安全**: 无需 GC，运行时开销极低

## 📦 快速安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
fjiffyldg = "0.1"
```

## 🚀 使用示例

```rust
use fjiffyldg::Fjiffyldg;

fn main() -> fjiffyldg::Result<()> {
    let fjiffyldg = Fjiffyldg::new();
    
    // 加载文件并构建行索引
    fjiffyldg.load_and_scan("large_file.txt")?;
    
    // 获取文件信息
    println!("文件大小: {} 字节", fjiffyldg.file_size());
    println!("总行数: {}", fjiffyldg.line_count());
    
    // 读取指定位置数据
    if let Some(data) = fjiffyldg.read(0, 1024) {
        println!("前1024字节: {:?}", data);
    }
    
    // 查询行信息
    println!("第5行起始位置: {}", fjiffyldg.line_pos(5));
    println!("第10行长度: {}", fjiffyldg.line_length(10));
    println!("位置100000所在行: {}", fjiffyldg.line_at_position(100000));
    
    Ok(())
}
```

## 📊 功能特性

| 功能 | 说明 |
|------|------|
| 文件加载 | 按需加载，支持内存映射 |
| 行索引 | 自动构建行偏移表，O(log n) 查找 |
| 编码检测 | UTF-8/UTF-16 自动识别 |
| 文件操作 | 复制、保存、追加、连接 |
| 零依赖 | 最小依赖，仅需 memmap2/encoding_rs |

## 📋 许可证

项目采用多重许可证：
- BSD-3-Clause（与原 C++ 项目保持一致）
- MIT（Rust 社区主流选择）
- Apache-2.0（可选，更宽松的专利授权）

## 📂 项目结构

```
fjiffyldg-rs/
├── src/
│   ├── lib.rs          # 公共 API
│   ├── error.rs        # 错误类型
│   ├── encoding.rs     # 编码检测
│   ├── line_index.rs   # 行索引
│   └── file.rs         # 文件操作
├── examples/           # 示例代码
├── Cargo.toml
└── README.md
```

## 🔄 与 C++ 版本比较

| 特性 | C++ 版本 | Rust 版本 |
|------|---------|----------|
| 许可证 | BSD 3-Clause | BSD-3-Clause OR MIT OR Apache-2.0 |
| 平台依赖 | U++ 框架 | Rust 标准库 + 轻量依赖 |
| 内存管理 | 手动 + RAII | 所有权 + 借用检查器 |
| 跨平台 | ✅ | ✅ |
| 生态系统 | C++ | crates.io |

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📞 联系方式

- 项目主页: https://github.com/fangfuzha/fjiffyldg-rs
- 作者: fangfuzha
