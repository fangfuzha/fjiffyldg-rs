# Rust Development TODO

[Chinese](DEVELOPMENT_TODO.md)

This document tracks the Rust rewrite work derived from the C++ coverage report.

## Phase 1: Core Correctness Fixes

| Task                             | Status | Notes                                             |
| -------------------------------- | ------ | ------------------------------------------------- |
| Fix CRLF line length calculation | Done   | Covered by CRLF and UTF-16/UTF-32 line scan tests |
| Implement UTF-32 BOM detection   | Done   | Supports UTF-32LE and UTF-32BE BOMs               |
| Implement `RestartScanFile` API  | Done   | Supports offset and UTF mode restart scanning     |

## Phase 2: Performance Optimization

| Task                               | Status | Notes                                                                        |
| ---------------------------------- | ------ | ---------------------------------------------------------------------------- |
| Populate and use chunk indexes     | Done   | Rust keeps exact offsets and uses chunk/overstep bounds for search narrowing |
| Fix `CHUNK_COUNT_MAX`              | Done   | Aligned with the C++ 8,388,608 chunk capacity                                |
| Implement large-file windowed mmap | Done   | Supports read remapping and background scanning over mmap windows            |
| Optimize large file operations     | Done   | clone/save/append/concat use mmap or large buffered paths where appropriate  |

## Phase 3: Completeness Enhancements

| Task                             | Status | Notes                                                                     |
| -------------------------------- | ------ | ------------------------------------------------------------------------- |
| Add C FFI bindings               | Done   | Exports C ABI handle, load, scan, query, read, encoding, and file helpers |
| Implement `BackstageRequestStop` | Done   | Uses cancellation flag and scan completion notification                   |
| Replace scan wait busy-loop      | Done   | Uses `Condvar` completion notification                                    |

## Phase 4: Detail Improvements

| Task                                               | Status | Notes                                                                            |
| -------------------------------------------------- | ------ | -------------------------------------------------------------------------------- |
| Avoid full buffer clone before background scanning | Done   | Uses shared `Arc<[u8]>` / `Arc<Mmap>` where applicable                           |
| Implement overstep handling                        | Done   | Records and uses overflow segment search bounds                                  |
| Advance pointer in `GetUtf8TextCharCount`          | Done   | Adds byte-consumption reporting                                                  |
| Implement `read_line_cut()`                        | Done   | Matches `ReadFileDataLLineCut` short-line batching and 4 KB long-line truncation |
| Improve `is_loaded()` diagnostics                  | Done   | Adds `load_status() -> Result<bool>` while keeping bool convenience API          |

## Release Support Tasks

| Task                                                     | Status | Notes                                                                                                                                                                                                                                             |
| -------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Generate C header with cbindgen and keep C smoke compile | Done   | [cbindgen.toml](../cbindgen.toml), [include/fjiffyldg.h](../include/fjiffyldg.h), [scripts/generate_c_header.ps1](../scripts/generate_c_header.ps1), [tests/c_smoke.c](../tests/c_smoke.c), [scripts/check_c_abi.ps1](../scripts/check_c_abi.ps1) |
| Add C++ reference-header compatibility layer             | Done   | `FJIFFYLDG_API`, `fjiffyldg_ptr`, and lightweight `Fjiffyldg::Fjiffyldg` wrapper are emitted through cbindgen configuration                                                                                                                       |
| Match `GetFileMappedHuge` mmap pointer semantics         | Done   | FFI handle owns `Mmap`; `ClearHugeBuffer` releases it                                                                                                                                                                                             |
| Add C/C++ link-and-run ABI smoke validation              | Done   | `scripts/check_c_abi.ps1` now builds the release dynamic library, links C and C++ smoke executables, and runs them against a sample input                                                                                                         |

## Documentation Tasks

| Task                                           | Status  | Notes                                                                                                                                                                                     |
| ---------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add bilingual developer documentation          | Done    | Each project document has Chinese and English versions, with a language switch link at the top                                                                                            |
| Add bilingual C API usage documentation        | Done    | Documents cbindgen header generation, build/link steps, handle lifetime, load/scan, reads, huge mmap, encoding helpers, error codes, and maintenance flow                                 |
| Cover every C API interface in the usage guide | Done    | Documents every exported interface from [include/fjiffyldg.h](../include/fjiffyldg.h) / [src/ffi.rs](../src/ffi.rs), including signature, parameters, return values, lifetimes, and notes |
| Keep bilingual documents synchronized          | Ongoing | Future documentation edits should update both language versions in the same change                                                                                                        |

## Progress

| Phase           | Tasks  | Done   | Progress |
| --------------- | ------ | ------ | -------- |
| Phase 1         | 3      | 3      | 100%     |
| Phase 2         | 4      | 4      | 100%     |
| Phase 3         | 3      | 3      | 100%     |
| Phase 4         | 5      | 5      | 100%     |
| Version roadmap | 2      | 0      | 0%       |
| Release support | 4      | 4      | 100%     |
| Documentation   | 4      | 3      | 75%      |
| Open governance | 10     | 7      | 70%      |
| **Total**       | **35** | **29** | **83%**  |

## Open Governance and Release Engineering Tasks

This checklist is based on Rust API Guidelines, Cargo publishing metadata, docs.rs metadata, GitHub community health files, Keep a Changelog, SemVer, and C ABI stability practices.

| Task                                               | Status | Notes                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add cross-platform CI quality gates                | Done   | [.github/workflows/ci.yml](../.github/workflows/ci.yml) runs fmt, Clippy, tests, docs, benchmark compilation, C/C++ ABI smoke, and publish dry-run on Windows/Linux/macOS                                                                                                          |
| Add contributing guide and security policy         | Done   | [CONTRIBUTING.md](CONTRIBUTING.md) / [CONTRIBUTING_EN.md](CONTRIBUTING_EN.md), [SECURITY.md](SECURITY.md) / [SECURITY_EN.md](SECURITY_EN.md) document development checks, C API change rules, bilingual docs, vulnerability reporting, supported versions, and security boundaries |
| Add changelog                                      | Done   | [CHANGELOG.md](CHANGELOG.md) / [CHANGELOG_EN.md](CHANGELOG_EN.md) follow Keep a Changelog and record the initial `0.1.0` baseline                                                                                                                                                  |
| Complete Cargo publishing metadata                 | Done   | [Cargo.toml](../Cargo.toml) includes `rust-version`, `documentation`, docs.rs metadata, crates.io-compliant keywords/categories, and package excludes                                                                                                                              |
| Add collaboration templates                        | Done   | Added `.github/ISSUE_TEMPLATE/*` and `.github/PULL_REQUEST_TEMPLATE.md` for bugs, features, C ABI issues, performance regressions, and PR checklist                                                                                                                                |
| Add dependency auto-update workflow                | Done   | [.github/dependabot.yml](../.github/dependabot.yml) automatically tracks Cargo and GitHub Actions updates with grouped PRs and cache-friendly cadence                                                                                                                              |
| Add tag-based release workflow                     | Done   | [.github/workflows/release.yml](../.github/workflows/release.yml) runs cross-platform build/test/verification, packages the header, and publishes the crate on tag pushes with caching                                                                                             |
| Add formatting, newline, and repository attributes | Todo   | Add [rustfmt.toml](../rustfmt.toml), [.editorconfig](../.editorconfig), and [.gitattributes](../.gitattributes) for formatting, line endings, generated-file markers, and reference linguist handling                                                                              |
| Document release and SemVer/ABI rules              | Todo   | Add `docs/release.md` / `docs/release_en.md` for versioning, pre-release checks, packaging, publish dry-run, tags, GitHub Release, docs.rs checks, C ABI compatibility, and yanking                                                                                                |
| Improve README trust signals                       | Todo   | Add badges or links for CI, crates.io, docs.rs, license, MSRV, contributing, security, changelog, and release status                                                                                                                                                               |

## Version Roadmap and Compatibility Strategy

### Task V.1: Make v1.0 Strictly Match C++ Observable Behavior

**Files:** [功能覆盖深度检查报告.md](功能覆盖深度检查报告.md), [feature_coverage_depth_report.md](feature_coverage_depth_report.md), [tests/](../tests/), [include/fjiffyldg.h](../include/fjiffyldg.h), [src/ffi.rs](../src/ffi.rs)

**Goal:** Establish the first stable release as a trustworthy Rust replacement for the C++ implementation.

**Requirements:**

- Align public C ABI function names, signatures, return values, error codes, null pointer handling, out-of-range handling, empty-file behavior, and other edge cases with the C++ reference implementation.
- Lock pointer lifetime and output semantics for `ReadFileData*`, `GetFileMappedHuge`, UTF detection, line index queries, and file operation helpers.
- Build a C++/Rust compatibility matrix and regression tests; require internal route parity only when it affects observable behavior or documented performance guarantees.

**Progress:** Added the first C ABI boundary regressions and fixed three Rust mismatches: `GetFileLineIndex` no longer returns `0` when no line offsets have been built, it no longer falls back to the last line when the queried byte position is past the end of the file, and `ReadFileData` now returns an empty buffer at EOF / past EOF instead of incorrectly reporting failure.

**Status:** Ongoing

### Task V.2: Keep ABI Compatible and Rustify Internals in v1.1+

**Files:** [release.md](release.md), [release_en.md](release_en.md), [CHANGELOG.md](CHANGELOG.md), [CHANGELOG_EN.md](CHANGELOG_EN.md)

**Goal:** After the v1.0 trust baseline is established, improve internals without breaking public ABI behavior.

**Requirements:**

- Use SemVer/ABI rules to distinguish behavior-compatible changes, performance optimizations, and breaking changes.
- Preserve the v1.0 differential tests when Rustifying threading, resource management, error handling, and performance paths.
- Add benchmark or compatibility notes for significant internal route changes.

**Status:** Later

## Remaining Work

- Complete the v1.0 strict C++ observable-behavior compatibility matrix and differential tests.
- Run real huge-file benchmarks beyond the automated 12 MB Criterion smoke workload.
- Quantify clone/save/append/concat performance against the C++ implementation.
- Continue the open governance checklist above.

## References

- Coverage report: [功能覆盖深度检查报告.md](功能覆盖深度检查报告.md) / [feature_coverage_depth_report.md](feature_coverage_depth_report.md)
- C API usage guide: [c_api_usage.md](c_api_usage.md) / [c_api_usage_en.md](c_api_usage_en.md)
- C++ reference implementation: [reference/fjiffyldg/Fjiffyldg/](../reference/fjiffyldg/Fjiffyldg/)
- Rust source: [src/](../src/)
- C/C++ header: [include/fjiffyldg.h](../include/fjiffyldg.h); run `pwsh -File scripts/check_c_abi.ps1` to verify generation, object compilation, dynamic linking, and smoke execution
- Large-file benchmark: [benches/large_file.rs](../benches/large_file.rs)

**Last updated**: 2026-05-21
**Maintainer**: Development team
