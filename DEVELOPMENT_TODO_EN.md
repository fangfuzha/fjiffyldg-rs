# Rust Development TODO

[中文文档](DEVELOPMENT_TODO.md)

This document tracks the Rust rewrite work derived from the C++ coverage report.

---

## Phase 1: Core Correctness Fixes

| Task | Status | Notes |
| ---- | ------ | ----- |
| Fix CRLF line length calculation | Done | Covered by CRLF and UTF-16/UTF-32 line scan tests |
| Implement UTF-32 BOM detection | Done | Supports UTF-32LE and UTF-32BE BOMs |
| Implement `RestartScanFile` API | Done | Supports offset and UTF mode restart scanning |

## Phase 2: Performance Optimization

| Task | Status | Notes |
| ---- | ------ | ----- |
| Populate and use chunk indexes | Done | Rust keeps exact offsets and uses chunk/overstep bounds for search narrowing |
| Fix `CHUNK_COUNT_MAX` | Done | Aligned with the C++ 8,388,608 chunk capacity |
| Implement large-file windowed mmap | Done | Supports read remapping and background scanning over mmap windows |
| Optimize large file operations | Done | clone/save/append/concat use mmap or large buffered paths where appropriate |

## Phase 3: Completeness Enhancements

| Task | Status | Notes |
| ---- | ------ | ----- |
| Add C FFI bindings | Done | Exports C ABI handle, load, scan, query, read, encoding, and file helpers |
| Implement `BackstageRequestStop` | Done | Uses cancellation flag and scan completion notification |
| Replace scan wait busy-loop | Done | Uses `Condvar` completion notification |

## Phase 4: Detail Improvements

| Task | Status | Notes |
| ---- | ------ | ----- |
| Avoid full buffer clone before background scanning | Done | Uses shared `Arc<[u8]>` / `Arc<Mmap>` where applicable |
| Implement overstep handling | Done | Records and uses overflow segment search bounds |
| Advance pointer in `GetUtf8TextCharCount` | Done | Adds byte-consumption reporting |
| Implement `read_line_cut()` | Done | Matches `ReadFileDataLLineCut` short-line batching and 4 KB long-line truncation |
| Improve `is_loaded()` diagnostics | Done | Adds `load_status() -> Result<bool>` while keeping bool convenience API |

## Release Support Tasks

| Task | Status | Notes |
| ---- | ------ | ----- |
| Generate C header with cbindgen and keep C smoke compile | Done | [cbindgen.toml](cbindgen.toml), [include/fjiffyldg.h](include/fjiffyldg.h), [scripts/generate_c_header.ps1](scripts/generate_c_header.ps1), [tests/c_smoke.c](tests/c_smoke.c), [scripts/check_c_abi.ps1](scripts/check_c_abi.ps1) |
| Add C++ reference-header compatibility layer | Done | `FJIFFYLDG_API`, `fjiffyldg_ptr`, and lightweight `Fjiffyldg::Fjiffyldg` wrapper are emitted through cbindgen configuration |
| Match `GetFileMappedHuge` mmap pointer semantics | Done | FFI handle owns `Mmap`; `ClearHugeBuffer` releases it |

## Documentation Tasks

| Task | Status | Notes |
| ---- | ------ | ----- |
| Add bilingual developer documentation | Done | Each project document has Chinese and English versions, with a language switch link at the top |
| Add bilingual C API usage documentation | Done | Documents cbindgen header generation, build/link steps, handle lifetime, load/scan, reads, huge mmap, encoding helpers, error codes, and maintenance flow |
| Keep bilingual documents synchronized | Ongoing | Future documentation edits should update both language versions in the same change |

## Progress

| Phase | Tasks | Done | Progress |
| ----- | ----- | ---- | -------- |
| Phase 1 | 3 | 3 | 100% |
| Phase 2 | 4 | 4 | 100% |
| Phase 3 | 3 | 3 | 100% |
| Phase 4 | 5 | 5 | 100% |
| Release support | 3 | 3 | 100% |
| Documentation | 3 | 2 | 67% |

## Remaining Work

- Run real huge-file benchmarks beyond the automated 12 MB Criterion smoke workload.
- Add fuller C/C++ link-and-run integration tests beyond header object compilation.
- Quantify clone/save/append/concat performance against the C++ implementation.

## References

- Coverage report: [docs/功能覆盖深度检查报告.md](docs/功能覆盖深度检查报告.md) / [docs/feature_coverage_depth_report.md](docs/feature_coverage_depth_report.md)
- C API usage guide: [docs/c_api_usage.md](docs/c_api_usage.md) / [docs/c_api_usage_en.md](docs/c_api_usage_en.md)
- C++ reference implementation: [reference/fjiffyldg/Fjiffyldg/](reference/fjiffyldg/Fjiffyldg/)
- Rust source: [src/](src/)
- C/C++ header: [include/fjiffyldg.h](include/fjiffyldg.h)
- Large-file benchmark: [benches/large_file.rs](benches/large_file.rs)

**Last updated**: 2026-05-20
**Maintainer**: Development team
