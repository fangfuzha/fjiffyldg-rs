# 发布指南

[English](release_en.md)

本文说明 `fjiffyldg` crate 的发布流程、版本规则和发布前检查。

## 版本规则

本项目当前处于 `0.x` 阶段，但仍遵循语义化版本控制的基本原则：

- 修复缺陷时优先维持现有 API 和 C ABI 兼容；
- 新增能力时尽量采用向后兼容的方式扩展；
- 只有在必须破坏兼容性时，才提升主版本前的次级版本并在变更日志中明确说明。

## 发布前检查

在打 tag 之前，建议至少运行以下命令：

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --all-features --no-deps
pwsh -File scripts/check_c_abi.ps1
cargo package --list --allow-dirty --registry crates-io
cargo publish --dry-run --allow-dirty --registry crates-io
```

这些检查会覆盖格式、静态分析、测试、文档、C/C++ ABI smoke 和 crates.io 打包可发布性。

## 发布步骤

1. 更新 [CHANGELOG.md](../CHANGELOG.md) 和 [CHANGELOG_EN.md](../CHANGELOG_EN.md)，记录本次版本变化。
2. 确认 [include/fjiffyldg.h](../include/fjiffyldg.h) 已由 `cbindgen` 重新生成并通过 `scripts/check_c_abi.ps1` 验证。
3. 确认 [docs/CONTRIBUTING.md](CONTRIBUTING.md)、[docs/SECURITY.md](SECURITY.md) 和 [docs/DEVELOPMENT_TODO.md](DEVELOPMENT_TODO.md) 中没有与发布相冲突的待办事项。
4. 创建版本 tag，例如 `v0.1.1`。
5. 由 `.github/workflows/release.yml` 负责在 tag push 时执行跨平台验证、头文件校验和发布。
6. 发布完成后，在 GitHub Release 中补充简要说明和必要的附件说明。

## 变更检查点

如果这次发布包含下列内容，请在发布说明中明确写出：

- C API 签名或返回值语义变化；
- 头文件生成规则变化；
- 文件加载、扫描、编码检测或读取路径变化；
- 平台支持范围变化；
- 任何可能影响 FFI 调用方的行为变化。

## Yanking

如果某个版本被证明存在严重问题，应优先在 crates.io 上 yank，并在变更日志和 GitHub Release 中说明原因与替代版本。
