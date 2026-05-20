# 变更日志

[English](CHANGELOG_EN.md)

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 的组织方式，并在稳定发布后遵循语义化版本控制。

## [未发布]

### 新增

- 补充开源库发布工程 TODO，覆盖 CI、社区健康、发布元数据、供应链、模板与发布流程。
- 增加 GitHub Actions CI，覆盖格式检查、Clippy、测试、文档构建、benchmark 编译、C/C++ ABI smoke 和发布 dry-run。
- 增加贡献指南与安全政策的中英双语版本。

## [0.1.0] - 2026-05-20

### 新增

- 提供 Rust 文件加载、行索引、编码检测、读取和文件操作 API。
- 提供由 `cbindgen` 生成的 C/C++ ABI 头文件和轻量 C++ RAII wrapper。
- 提供 C/C++ ABI 编译、链接和运行 smoke 验证脚本。
- 提供 Criterion 大文件基准入口。
- 提供中英双语 README、覆盖报告、开发 TODO 和 C API 使用指南。
