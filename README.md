# Fjiffyldg (Rust)

[English](README_EN.md)

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
    println!("位置100000所在行: {}", fjiffyldg.line_at_pos(100000));

    Ok(())
}
```

## 📊 功能特性

| 功能      | 说明                                            |
| --------- | ----------------------------------------------- |
| 文件加载  | 按需加载，支持内存映射                          |
| 行索引    | 自动构建行偏移表，O(log n) 查找                 |
| 编码检测  | UTF-8/UTF-16 自动识别                           |
| 文件操作  | 复制、保存、追加、连接                          |
| C/C++ ABI | 提供 C/C++ 头文件与 smoke 编译检查              |
| 基准验证  | 提供 Criterion 大文件加载、扫描、查询和读取基准 |
| 零依赖    | 最小依赖，仅需 memmap2/encoding_rs              |

## 🧩 C ABI

公共 C/C++ 头文件位于 `include/fjiffyldg.h`，由 `cbindgen` 根据 Rust FFI 源码生成，提供 C 函数声明、`fjiffyldg_ptr` 句柄别名、导出宏和轻量 C++ RAII 包装。内置检查会验证头文件未过期、C/C++ 声明可编译、release 动态库可链接并能运行最小 smoke。详细使用说明与全接口参考见 [docs/c_api_usage.md](docs/c_api_usage.md)。

```powershell
cargo install cbindgen --locked
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

## 🧪 基准测试

```bash
cargo bench --bench large_file
```

基准会生成约 12MB 的临时文件，覆盖 mmap 加载扫描、随机行查询和随机读取路径。

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
├── include/            # cbindgen 生成的 C ABI 头文件
├── examples/           # 示例代码
├── benches/            # Criterion 基准
├── scripts/            # 发布与验证脚本
├── tests/              # 集成与 C ABI smoke 输入
├── docs/               # 覆盖报告、TODO、发布与项目文档
├── Cargo.toml
├── README.md           # 中文 README
└── README_EN.md        # English README
```

## 🔄 与 C++ 版本比较

| 特性     | C++ 版本     | Rust 版本                         |
| -------- | ------------ | --------------------------------- |
| 许可证   | BSD 3-Clause | BSD-3-Clause OR MIT OR Apache-2.0 |
| 平台依赖 | U++ 框架     | Rust 标准库 + 轻量依赖            |
| 内存管理 | 手动 + RAII  | 所有权 + 借用检查器               |
| 跨平台   | ✅           | ✅                                |
| 生态系统 | C++          | crates.io                         |

## 🤝 贡献

欢迎提交 Issue 和 Pull Request。仓库提供 bug、feature、C ABI 和性能回归模板，并通过 GitHub Actions 运行格式、Clippy、测试、文档、benchmark 编译、C/C++ ABI smoke 和发布 dry-run。参与前建议阅读 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)，安全问题请参考 [docs/SECURITY.md](docs/SECURITY.md)，版本变化见 [docs/CHANGELOG.md](docs/CHANGELOG.md)，开发计划见 [docs/DEVELOPMENT_TODO.md](docs/DEVELOPMENT_TODO.md)。

## 📞 联系方式

- 项目主页: https://github.com/fangfuzha/fjiffyldg-rs
- 文档主页: https://docs.rs/fjiffyldg
- 作者: fangfuzha
