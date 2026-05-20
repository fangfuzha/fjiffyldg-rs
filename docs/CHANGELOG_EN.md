# Changelog

[中文文档](CHANGELOG.md)

This project follows the structure of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and will follow Semantic Versioning after stable releases.

## [Unreleased]

### Added

- Added open-source library release-engineering TODOs covering CI, community health, release metadata, supply chain checks, templates, and release workflow.
- Added GitHub Actions CI covering formatting, Clippy, tests, documentation build, benchmark compilation, C/C++ ABI smoke, and publish dry-run.
- Added bilingual contributing guides and security policies.

## [0.1.0] - 2026-05-20

### Added

- Added Rust APIs for file loading, line indexing, encoding detection, reads, and file operations.
- Added a `cbindgen`-generated C/C++ ABI header and lightweight C++ RAII wrapper.
- Added C/C++ ABI compile, link, and runtime smoke validation script.
- Added a Criterion large-file benchmark entry.
- Added bilingual README, coverage report, development TODO, and C API usage guide.
