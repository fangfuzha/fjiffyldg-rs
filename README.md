# Fjiffyldg (Rust)

[English](README_EN.md)

[![CI](https://img.shields.io/github/actions/workflow/status/fangfuzha/fjiffyldg-rs/ci.yml?branch=main&label=CI)](https://github.com/fangfuzha/fjiffyldg-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fjiffyldg)](https://crates.io/crates/fjiffyldg)
[![docs.rs](https://img.shields.io/docsrs/fjiffyldg)](https://docs.rs/fjiffyldg)
[![license](https://img.shields.io/crates/l/fjiffyldg)](LICENSE)

高性能、跨平台的文件处理库，专为现代大文件的高效加载与访问而设计。

> 这是原 C++ [Fjiffyldg 项目](https://github.com/ceepuka/fjiffyldg) 的 Rust 重写版本。

## 文档入口

- 中文索引：[docs/README.md](docs/README.md)
- 英文索引：[docs/README_EN.md](docs/README_EN.md)

## 核心特性

- 跨平台：Windows / Linux / macOS / Android
- 超大文件：理论上限低于 8 EB，实际适用于超大输入
- 智能行索引：可直接定位百万行级文件中的目标行
- 内存映射：支持高效大文件访问与窗口重映射
- 编码支持：UTF-8 / UTF-16 (LE/BE) / UTF-32 (LE/BE)
- C/C++ ABI：提供公共头文件、C 函数声明、句柄别名、导出宏和轻量 C++ RAII 包装
- 基准测试：提供大文件加载、扫描、查询和读取路径的 Criterion 基准
- 内存安全：基于 Rust 所有权和借用模型，运行时开销低

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
fjiffyldg = "0.1"
```

## 使用示例

```rust
use fjiffyldg::Fjiffyldg;

fn main() -> fjiffyldg::Result<()> {
    let fjiffyldg = Fjiffyldg::new();

    fjiffyldg.load_and_scan("large_file.txt")?;

    println!("文件大小: {} 字节", fjiffyldg.file_size());
    println!("总行数: {}", fjiffyldg.line_count());

    if let Some(data) = fjiffyldg.read(0, 1024) {
        println!("前 1024 字节: {:?}", data);
    }

    println!("第 5 行起始位置: {}", fjiffyldg.line_pos(5));
    println!("第 10 行长度: {}", fjiffyldg.line_length(10));
    println!("位置 100000 所在行: {}", fjiffyldg.line_at_pos(100000));

    Ok(())
}
```

## C/C++ ABI

公共 C/C++ 头文件位于 [include/fjiffyldg.h](include/fjiffyldg.h)。它由 `cbindgen` 根据 Rust FFI 源码生成，提供 C 声明、`fjiffyldg_ptr` 句柄别名、导出宏和轻量 C++ RAII 包装。内置检查会验证头文件是否最新、C/C++ 声明是否可编译、release 动态库是否可链接，以及最小 smoke 可执行文件是否能运行。完整使用说明和 API 参考见 [docs/c_api_usage.md](docs/c_api_usage.md)。

```powershell
cargo install cbindgen --locked
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

## 基准测试

```bash
cargo bench --bench large_file
```

基准会生成约 12 MB 的临时文件，覆盖 mmap 加载扫描、随机行查询和随机读取。

## 许可证

项目采用多重许可证：

- BSD-3-Clause，与原 C++ 项目保持一致
- MIT，Rust 生态常见许可证
- Apache-2.0，提供可选的专利友好授权

## 项目结构

```text
fjiffyldg-rs/
├── src/                # Rust API、FFI、编码、索引与文件操作
├── include/            # cbindgen 生成的 C/C++ ABI 头文件
├── examples/           # 示例程序
├── benches/            # Criterion 基准
├── scripts/            # 发布与校验脚本
├── tests/              # 集成测试与 C/C++ ABI smoke 输入
├── docs/               # 覆盖报告、TODO、发布与项目文档
├── Cargo.toml
├── README.md           # 中文 README
└── README_EN.md        # 英文 README
```

## 与 C++ 版本比较

| 特性     | C++ 版本     | Rust 版本                         |
| -------- | ------------ | --------------------------------- |
| 许可证   | BSD 3-Clause | BSD-3-Clause OR MIT OR Apache-2.0 |
| 平台依赖 | U++ 框架     | Rust 标准库 + 轻量依赖            |
| 内存管理 | 手动 + RAII  | 所有权 + 借用检查                 |
| 跨平台   | 是           | 是                                |
| 生态     | C++          | crates.io                         |

## 贡献

欢迎提交 Issue 和 Pull Request。仓库提供 bug、feature、C ABI 和性能回归模板，并通过 GitHub Actions 运行格式化、Clippy、测试、文档、基准目标编译、C/C++ ABI smoke 和发布 dry-run。文档入口见 [docs/README.md](docs/README.md)。参与前建议阅读 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) 和 [docs/CODE_OF_CONDUCT.md](docs/CODE_OF_CONDUCT.md)，安全问题请参考 [docs/SECURITY.md](docs/SECURITY.md)，版本历史见 [docs/CHANGELOG.md](docs/CHANGELOG.md)，开发路线图见 [docs/DEVELOPMENT_TODO.md](docs/DEVELOPMENT_TODO.md)，发布流程见 [docs/release.md](docs/release.md)。

## 联系方式

- 项目主页：https://github.com/fangfuzha/fjiffyldg-rs
- 文档主页：https://docs.rs/fjiffyldg
- 作者：fangfuzha
