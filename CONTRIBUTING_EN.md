# Contributing Guide

[中文文档](CONTRIBUTING.md)

Thank you for considering a contribution to the Rust version of Fjiffyldg. This guide describes the recommended development workflow and pre-submission checks for this repository.

## Development Environment

- Install the stable Rust toolchain.
- Install `cbindgen` to generate and verify the C/C++ ABI header:

```powershell
cargo install cbindgen --locked
```

- Core Rust tests should work on Windows, Linux, and macOS. The C/C++ ABI smoke check requires local C and C++ compilers.

## Common Check Commands

Before opening a pull request, try to run:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --all-features --no-deps
pwsh -File scripts/check_c_abi.ps1
cargo publish --dry-run
```

`scripts/check_c_abi.ps1` verifies that [include/fjiffyldg.h](include/fjiffyldg.h) matches the Rust FFI source, builds the release dynamic library, and links/runs C and C++ smoke executables.

## Rules for C API Changes

- Do not edit [include/fjiffyldg.h](include/fjiffyldg.h) by hand. After changing [src/ffi.rs](src/ffi.rs) or [cbindgen.toml](cbindgen.toml), run:

```powershell
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

- When changing exported signatures, error codes, pointer lifetimes, or buffer ownership rules, update:
  - [docs/c_api_usage.md](docs/c_api_usage.md)
  - [docs/c_api_usage_en.md](docs/c_api_usage_en.md)
  - Related status entries in [DEVELOPMENT_TODO.md](DEVELOPMENT_TODO.md) / [DEVELOPMENT_TODO_EN.md](DEVELOPMENT_TODO_EN.md)
- C ABI changes can be breaking changes. Call this out explicitly in the commit or pull request description.

## Documentation Synchronization

This project maintains Chinese and English documentation. When adding or editing project documentation, update the matching language version in the same change and keep the language switch link at the top.

## Pull Request Recommendations

- Keep each pull request focused on one topic.
- Add tests or smoke coverage for behavior changes.
- For performance changes, describe the benchmark or measurement method.
- For release-related changes, explain the impact on crates.io, docs.rs, C ABI, or platform support.

## Reporting Issues

When filing an issue, include the operating system, Rust version, reproduction steps, input file size/encoding, expected behavior, actual behavior, and relevant logs when possible.
