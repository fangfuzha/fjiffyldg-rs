# Rust Version vs C++ Reference Implementation - Coverage Depth Report

[中文文档](feature_coverage_depth_report.md)

**Review date**: 2026-05-20
**Overall assessment**: The Rust version covers the core C++ feature set. The remaining work is focused on real huge-file benchmarking and broader release validation.

---

## 1. Public API Coverage

| C++ API                                    | Rust counterpart                                                      | Status | Notes                                                                          |
| ------------------------------------------ | --------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------ |
| `fjiffyldg_create()` / `fjiffyldg_clear()` | `ffi::fjiffyldg_create()` / `ffi::fjiffyldg_clear()`                  | Done   | Opaque C ABI handle                                                            |
| `LoadAndScanFile()`                        | `load_and_scan()`                                                     | Done   | Implemented                                                                    |
| `LoadFileOnly()`                           | `load()`                                                              | Done   | Implemented                                                                    |
| `GetFileIsLoaded()`                        | `is_loaded()` / `load_status()` / `error_code()`                      | Done   | Keeps bool convenience API and exposes detailed status                         |
| `RestartScanFile(offset, utf)`             | `restart_scan(offset, utf_mode)`                                      | Done   | Supports offset and UTF mode restart scanning                                  |
| `WaitFileScanTaskFinished()`               | `wait_scan()`                                                         | Done   | Uses `Condvar` completion notification                                         |
| `BackstageRequestStop()`                   | `request_stop_scan()` / `ffi::BackstageRequestStop()`                 | Done   | Cancels scanning and clears current index                                      |
| Line count/position/length/index queries   | `line_count()`, `line_pos()`, `line_length()`, `line_at_pos()`        | Done   | Implemented                                                                    |
| `ReadFileData()`                           | `read()`                                                              | Done   | Implemented                                                                    |
| `ReadFileDataLLineCut()`                   | `read_line_cut()`                                                     | Done   | Supports short-line batching and 4 KB long-line truncation                     |
| `ReadFileDataEndOfLine()`                  | `read_to_line_end()`                                                  | Done   | Implemented                                                                    |
| `GetFileMappedHuge()`                      | `ffi::GetFileMappedHuge()` / `get_huge_buffer()`                      | Done   | FFI returns a real mmap pointer; Rust high-level API keeps safe copy semantics |
| `ClearHugeBuffer()`                        | `ffi::ClearHugeBuffer()`                                              | Done   | Releases the mmap resource held by the FFI handle                              |
| File helpers                               | `clone_file()`, `save_file()`, `append_file()`, `concatenate_files()` | Done   | Large inputs use mmap or large-buffer paths                                    |

## 2. Encoding Coverage

| C++ feature                                  | Rust status | Notes                                                      |
| -------------------------------------------- | ----------- | ---------------------------------------------------------- |
| UTF-8/UTF-16/UTF-32 BOM detection            | Done        | UTF-32LE/BE detection is covered                           |
| ASCII check with 8-byte mask                 | Done        | Equivalent optimized implementation                        |
| Whole UTF-8 validation                       | Done        | Implemented                                                |
| Extracted UTF-8 validation                   | Done        | Implemented                                                |
| `GetUtf8TextCharCount()` pointer advancement | Done        | `get_utf8_char_count_with_offset()` reports consumed bytes |
| UTF-16/UTF-32 to UTF-8 conversion            | Done        | UTF-16 uses `encoding_rs`; UTF-32 uses native conversion   |

## 3. File Loading and I/O

| C++ feature                          | Rust status              | Notes                                                                                                     |
| ------------------------------------ | ------------------------ | --------------------------------------------------------------------------------------------------------- |
| Small-file direct buffer             | Done                     | Files up to 10 MB use memory buffers                                                                      |
| Large-file mmap                      | Done                     | Uses windowed mmap access                                                                                 |
| 1 GB mmap chunks                     | Done                     | Read and background scan both advance through mmap windows                                                |
| `BUFFER_SIZE` 128 KB default         | Done                     | Defined and used                                                                                          |
| `GetFileMappedHuge` internal pointer | Done                     | FFI returns a handle-owned real mmap pointer                                                              |
| Large clone/save/append/concat paths | Done                     | Uses mmap or large buffered paths                                                                         |
| `FILEBLOCK` 1 MB stream block        | Intentionally not reused | Rust keeps a simpler full small-file buffer design                                                        |
| C++ header RAII wrapper              | Done                     | cbindgen configuration emits the lightweight `Fjiffyldg::Fjiffyldg` wrapper                               |
| C API usage documentation            | Done                     | Bilingual guides cover generation, build, link, lifetime, maintenance flows, and a complete API reference |
| C/C++ link-and-run smoke             | Done                     | The check script builds the release dynamic library, links C/C++ smoke, and runs it                       |

## 4. Line Index System

| Layer / function                  | Rust status | Notes                                                                        |
| --------------------------------- | ----------- | ---------------------------------------------------------------------------- |
| Direct offsets for first 1M lines | Done        | `direct_offsets: Vec<u32>`                                                   |
| Extended offsets beyond 4 GB      | Done        | `extended_offsets: Vec<u64>`                                                 |
| Chunk index                       | Done        | `chunks: Vec<ChunkIndex>` is populated and used for bounds narrowing         |
| Chunk capacity                    | Done        | Aligned with the C++ 8,388,608 limit                                         |
| Overstep handling                 | Done        | Tracks the first overflow position and narrows searches after the last chunk |
| Cached line/position              | Done        | `cached_line` / `cached_pos`                                                 |
| `GetLindexPos` equivalent         | Done        | `get_line_pos()` uses chunk/overstep search bounds and exact offsets         |
| `GetLineByPos` equivalent         | Done        | `get_line_by_pos()` uses chunk/overstep bounds                               |
| Line length calculation           | Done        | CRLF, UTF-16LE/BE, and UTF-32LE/BE byte offsets are covered                  |

## 5. Error Handling

The Rust version maps C-compatible error codes while also exposing richer Rust errors: `InvalidOffset`, `InvalidLineIndex`, `BufferTooSmall`, `EncodingError`, and `IoError`.

## 6. Threads and Concurrency

| C++ mechanism              | Rust mechanism                         | Status |
| -------------------------- | -------------------------------------- | ------ |
| U++ background scan thread | `rayon::spawn`                         | Done   |
| Thread wait / join         | `Condvar` completion notification      | Done   |
| Scan cancellation          | Cancellation flag + condition variable | Done   |
| Shared scan data           | `Arc<[u8]>` / `Arc<Mmap>`              | Done   |
| Atomic scan flag           | `AtomicBool`                           | Done   |

## 7. Completed Validation Highlights

- CRLF and unterminated final-line handling
- UTF-16LE/BE and UTF-32LE/BE original byte offsets
- UTF-32 BOM detection and UTF-32 to UTF-8 conversion
- Restart scanning from offset and encoding modes
- Million-line index continuation and chunk/overstep search narrowing
- `ReadFileDataLLineCut` line batching and 4 KB truncation
- UTF-8 pointer advancement semantics
- Load status diagnostics
- Scan wait notification and cancellation
- Shared scan buffers instead of full pre-scan cloning
- Large file clone/save/append/concatenate I/O paths
- C FFI smoke coverage
- cbindgen-generated C/C++ header smoke compilation
- C/C++ ABI link-and-run smoke validation
- Bilingual C API usage documentation with a complete exported-interface reference
- `GetFileMappedHuge` real mmap pointer semantics
- Windowed mmap reading and background scanning

## 8. Remaining Items

| Priority | Item                                          | Impact                                                                   | Recommendation                                                        |
| -------- | --------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| Medium   | Real huge-file benchmark validation           | Automated benchmark currently uses an approximately 12 MB temporary file | Run Criterion and/or external benchmarking on real very large files   |
| Low      | clone/save/append/concat benchmark comparison | Performance has not yet been quantified against the C++ version          | Add I/O benchmark cases and compare with the reference implementation |

## 9. V1 Observable-Behavior Alignment Matrix

This table records only the public C ABI behaviors that are already locked down by regression tests and explicitly aligned with the C++ reference implementation. It is not a full API reference; it is the auditable v1.0 behavior baseline.

| Behavior                                         | C++ observable contract                                                                                         | Rust status | Evidence                                                                                                                                                                     |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LoadFileOnly` without scanning                  | `GetFileIsLoaded == 0`, `GetFileLineCount == 0`, line-index queries stay unavailable                            | Done        | `test_c_ffi_load_file_only_reports_scan_not_started`                                                                                                                         |
| `GetFileLineIndex` before any line offsets exist | Returns `-1`, never line `0` by default                                                                         | Done        | `test_c_ffi_load_file_only_reports_scan_not_started`                                                                                                                         |
| `GetFileLineIndex` past EOF                      | Returns `-1`, does not fall back to the last line                                                               | Done        | `test_c_ffi_line_index_rejects_positions_past_file_end`                                                                                                                      |
| `ReadFileData` with negative position            | Clamps to the file start before reading                                                                         | Done        | `test_c_ffi_read_data_clamps_negative_positions_to_start`                                                                                                                    |
| `ReadFileData` at EOF / past EOF                 | Returns an empty buffer and writes `len = 0` instead of failing                                                 | Done        | `test_c_ffi_read_data_clamps_positions_at_file_end`                                                                                                                          |
| `ReadFileDataEndOfLine` at line end or file end  | Returns an empty buffer and writes `len = 0`                                                                    | Done        | `test_c_ffi_read_to_end_of_line_returns_empty_at_line_boundary`, `test_c_ffi_read_to_end_of_line_returns_empty_at_file_end`                                                  |
| `ReadFileDataLLineCut` lookup failure            | Returns null, resets only `len`, preserves caller `index/bpos/epos`                                             | Done        | `test_c_ffi_line_cut_preserves_bounds_when_lookup_fails`                                                                                                                     |
| `ReadFileDataLLineCut` with short budget         | `len` is a batching budget, not a hard cap; short budgets still return complete line boundaries                 | Done        | `test_c_ffi_line_cut_returns_complete_line_with_short_budget`                                                                                                                |
| `GetFileMappedHuge` repeated-call lifetime       | A new call invalidates the previous pointer even when the new mapping fails                                     | Done        | `get_file_mapped_huge_retains_mmap_until_clear`, `get_file_mapped_huge_clears_previous_mapping_on_failure`                                                                   |
| Empty file on scan path                          | Exposes one empty line at offset `0` with length `0`                                                            | Done        | `test_c_ffi_empty_file_matches_cpp_observable_line_state`                                                                                                                    |
| Empty file on `LoadFileOnly` path                | Stays loaded-but-not-scanned; does not expose the synthetic empty line early                                    | Done        | `test_c_ffi_empty_file_load_only_keeps_scan_not_started`                                                                                                                     |
| Queries while scanning is in progress            | `GetFileLinePos` may expose built prefixes; `GetFileLineIndex` and `GetFileLineLength` remain unavailable       | Done        | `get_file_line_pos_exposes_scanned_prefix_while_scanning`, `get_file_line_index_stays_unavailable_while_scanning`, `test_c_ffi_line_length_stays_unavailable_while_scanning` |
| `GetFileSizeByteCount` for missing files         | Returns `-1`, matching the C++ `GetFileLength` failure contract                                                 | Done        | `test_c_ffi_get_file_size_missing_file_returns_reference_error`                                                                                                              |
| File-write helpers with empty buffers            | A null buffer is accepted when `len == 0`; saving/appending empty content succeeds                              | Done        | `test_c_ffi_file_helpers_accept_null_empty_buffers`                                                                                                                          |
| File helper severe failures                      | `ToCloneFile` / `ToSaveFile` / `ToAppendFile` / `ToConcatenateFile` return `-1` on failure                      | Done        | `test_c_ffi_file_helpers_use_reference_failure_codes`                                                                                                                        |
| Encoding helpers with empty input                | Null pointers are accepted when `len == 0` and return success-style values                                      | Covered     | `test_c_ffi_text_and_file_helpers_handle_boundaries`                                                                                                                         |
| `CheckExtractTextUtf8` slice boundaries          | Ignores UTF-8 characters that may be truncated at either slice boundary and validates the complete middle range | Done        | `test_c_ffi_text_and_file_helpers_handle_boundaries`                                                                                                                         |
| `GetUtf8TextCharCount` invalid byte boundary     | Stops at the first invalid or incomplete UTF-8 byte and advances the input pointer to that byte                 | Covered     | `test_c_ffi_text_and_file_helpers_handle_boundaries`                                                                                                                         |
| `RestartScanFile` provided path                  | Always reloads/scans the caller-provided `name` instead of reusing the already loaded file                      | Done        | `test_c_ffi_restart_scan_uses_provided_path`                                                                                                                                 |
| `RestartScanFile` missing path                   | Stops and clears the old line index before rescan; missing paths do not keep exposing old index results         | Covered     | `test_c_ffi_restart_scan_missing_path_clears_previous_index`                                                                                                                 |
| `RestartScanFile` negative offset                | Negative offsets do not keep exposing old index results; the old line index becomes unavailable                 | Done        | `test_c_ffi_restart_scan_negative_offset_clears_previous_index`                                                                                                              |
| Loading missing files                            | `LoadAndScanFile` / `LoadFileOnly` return `-1` for paths that do not exist                                      | Done        | `test_c_ffi_load_missing_file_returns_reference_error`                                                                                                                       |

## 10. Conclusion

The Rust version covers the common file loading, scanning, line positioning, encoding detection, and reading scenarios from the C++ reference implementation. Recent work also aligned cbindgen-generated C/C++ ABI packaging, bilingual C API documentation with complete interface coverage, `GetFileMappedHuge` mmap pointer behavior, windowed mmap scanning, and large-file helper paths.

Future work should focus on release validation: real huge-file benchmarks and quantified I/O performance comparison against the C++ implementation.
