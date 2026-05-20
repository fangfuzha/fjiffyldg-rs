# 贡献指南

[English](CONTRIBUTING_EN.md)

感谢你考虑为 Fjiffyldg Rust 版本贡献代码、文档或问题反馈。本文说明本仓库推荐的开发流程与提交前检查。

## 开发环境

- 安装稳定版 Rust 工具链。
- 安装 `cbindgen`，用于生成和验证 C/C++ ABI 头文件：

```powershell
cargo install cbindgen --locked
```

- Windows、Linux、macOS 均应能运行核心 Rust 测试；C/C++ ABI smoke 需要本机 C 与 C++ 编译器。

## 常用检查命令

提交 PR 前请尽量运行：

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --all-features --no-deps
pwsh -File scripts/check_c_abi.ps1
cargo publish --dry-run
```

其中 `scripts/check_c_abi.ps1` 会验证 [include/fjiffyldg.h](../include/fjiffyldg.h) 与 Rust FFI 源码一致，构建 release 动态库，并链接运行 C/C++ smoke。

## 修改 C API 的规则

- 不要手工编辑 [include/fjiffyldg.h](../include/fjiffyldg.h)。修改 [src/ffi.rs](../src/ffi.rs) 或 [cbindgen.toml](../cbindgen.toml) 后运行：

```powershell
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

- 修改导出函数签名、错误码、指针生命周期或缓冲区所有权时，需要同步更新：
    - [c_api_usage.md](c_api_usage.md)
    - [c_api_usage_en.md](c_api_usage_en.md)
    - [DEVELOPMENT_TODO.md](DEVELOPMENT_TODO.md) / [DEVELOPMENT_TODO_EN.md](DEVELOPMENT_TODO_EN.md) 中相关状态
- C ABI 变更可能是破坏性变更，提交说明中应明确标注。

## 文档同步

本项目维护中文与英文文档。新增或修改项目文档时，请在同一提交中同步对应语言版本，并在文档顶部保留语言切换链接。

## Pull Request 建议

- PR 应聚焦一个主题，避免混合不相关修改。
- 为行为变化补充测试或 smoke 覆盖。
- 对性能相关修改，尽量说明基准或测量方式。
- 对发布相关修改，说明对 crates.io、docs.rs、C ABI 或平台支持的影响。

## 报告问题

提交 issue 时请尽量提供：操作系统、Rust 版本、复现步骤、输入文件规模/编码、期望行为、实际行为和相关日志。
