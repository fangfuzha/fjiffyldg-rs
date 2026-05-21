# Rust 版本开发 TODO 清单

[English](DEVELOPMENT_TODO_EN.md)

基于功能覆盖深度检查报告，以下是后续开发的优先级任务清单。

---

## 📋 第一阶段：核心功能修复（优先级：高）

### ✅ Task 1.1: 修复 CRLF 行长度计算

- **文件**：[src/line_index.rs](../src/line_index.rs)
- **问题**：行长度计算对 `\r\n` 只减 1 字节，应减 2
- **影响**：Windows 文本文件行长度偏差 1 字节
- **修复方案**：

    ```rust
    // 当前（错误）
    let len = next_pos - current_pos - 1;

    // 应改为
    let len = if data[next_pos - 2..next_pos] == [b'\r', b'\n'] {
        next_pos - current_pos - 2
    } else {
        next_pos - current_pos - 1
    };
    ```

- **测试**：已添加 CRLF、UTF-16LE/BE、UTF-32LE/BE 行扫描单元测试
- **状态**：✅ 已完成

---

### ✅ Task 1.2: 实现 UTF-32 BOM 检测

- **文件**：[src/encoding.rs](../src/encoding.rs)
- **问题**：`UtfMode::Utf32Le/Utf32Be` 已定义但 `detect_encoding()` 未检测
- **影响**：无法识别 UTF-32 编码文件
- **修复方案**：
    - 在 `detect_encoding()` 中添加 UTF-32 BOM 检测
    - UTF-32LE BOM: `0xFF 0xFE 0x00 0x00`
    - UTF-32BE BOM: `0x00 0x00 0xFE 0xFF`
- **测试**：添加 UTF-32 BOM 检测单元测试
- **状态**：✅ 已完成

---

### ✅ Task 1.3: 实现 `RestartScanFile` API

- **文件**：[src/file.rs](../src/file.rs)、[src/lib.rs](../src/lib.rs)
- **问题**：C++ 版本支持重新扫描指定 offset 和 UTF 模式，Rust 版本缺失
- **影响**：无法重新扫描文件或更改编码模式
- **修复方案**：
    ```rust
    pub fn restart_scan(&mut self, offset: u64, utf_mode: UtfMode) -> Result<(), FjiffyldgError> {
        // 停止当前扫描
        self.stop_scan();
        // 重置扫描状态
        self.line_index.reset();
        // 从指定 offset 重新扫描
        self.start_background_scan(offset, utf_mode)
    }
    ```
- **测试**：添加重新扫描功能测试
- **状态**：✅ 已完成

---

## 🚀 第二阶段：性能优化（优先级：高）

### ✅ Task 2.1: 实现分块索引真正的填充和查询

- **文件**：[src/line_index.rs](../src/line_index.rs)
- **问题**：`chunks` 字段定义了但 `build_from_data()` 未填充，查询函数未使用
- **影响**：>100 万行文件定位性能从 `O(log n)` 退化到 `O(n)`
- **修复方案**：
    1. 在 `build_from_data()` 中填充 `chunks` 向量
    2. 修改 `get_line_pos()` 使用三层索引查询
    3. 添加分块索引的二分查找逻辑
- **测试**：已添加百万行后续偏移保存、chunk 填充、chunk/overstep 溢出记录、按位置与按行号查询范围裁剪单元测试
- **状态**：✅ 已完成（2026-05-20；Rust 保留完整偏移数组以保证 O(1) 精确读取，同时通过 chunk/overstep 裁剪行号与位置查询范围）
- **复杂度**：高

---

### ✅ Task 2.2: 修正 `CHUNK_COUNT_MAX` 值

- **文件**：[src/line_index.rs](../src/line_index.rs)
- **问题**：Rust 中为 8192，C++ 中为 8,388,608，相差 1024 倍
- **影响**：即使实现分块索引，容量也严重不足
- **修复方案**：

    ```rust
    // 当前（错误）
    const CHUNK_COUNT_MAX: usize = 8192;

    // 应改为
    const CHUNK_COUNT_MAX: usize = 8_388_608;
    ```

- **测试**：验证超大文件分块容量
- **状态**：✅ 已完成

---

### ✅ Task 2.3: 实现大文件分块映射

- **文件**：[src/file.rs](../src/file.rs)
- **问题**：仅一次映射整个文件，超大文件受限地址空间
- **影响**：无法处理超过地址空间限制的文件
- **修复方案**：
    - 实现 1GB 分块映射（`MMAP_FILECHUNK`）
    - 维护当前 mmap 窗口偏移与窗口大小
    - 在读取时动态切换分块
    - 后台扫描按 mmap 窗口顺序推进，并保留窗口尾部字节处理跨窗口换行
- **测试**：已添加 `test_read_data_remaps_window_for_far_mmap_offset`、`test_scan_uses_all_mmap_windows`、`test_windowed_scan_supports_unaligned_restart_offset` 与 `test_windowed_scan_preserves_crlf_across_boundary`，通过小窗口注入验证跨窗口读取、扫描、非对齐重扫偏移与 CRLF 边界
- **状态**：✅ 已完成（2026-05-20）
- **复杂度**：高

---

### ⏳ Task 2.4: 优化文件操作的大文件 mmap

- **文件**：[src/file.rs](../src/file.rs)
- **问题**：`clone_file`/`save_file` 等未对大文件使用 mmap
- **影响**：大文件操作性能不佳
- **修复方案**：
    - 对 >10MB 文件使用 mmap 加速 copy
    - 使用 `memcpy` 或 `sendfile` 优化
- **测试**：已添加 `test_append_file_creates_missing_target`、`test_save_file_large_buffer_round_trips`、`test_clone_file_large_input_round_trips`、`test_concatenate_files_large_input_appends_to_output`
- **状态**：✅ 已完成（2026-05-19）

---

## 🔧 第三阶段：完整性增强（优先级：中）

### ✅ Task 3.1: 添加 C FFI 绑定

- **文件**：[src/lib.rs](../src/lib.rs)、建议新增 `src/ffi.rs`
- **问题**：Rust 当前只有原生 API，没有 `extern "C"` 导出函数
- **影响**：无法覆盖 C++ 版本面向 C ABI 的调用方式
- **修复方案**：新增 `src/ffi.rs`，导出 opaque handle API、加载/扫描/等待/行查询/读取、编码检查与基础文件操作 C ABI；保留 `crate-type = ["lib", "cdylib", "staticlib"]`
- **测试**：已添加 `test_c_ffi_smoke_load_scan_query_and_read`
- **状态**：✅ 已完成（2026-05-19；后续可补真实 C 集成测试与发布用头文件）

---

### ⏳ Task 3.2: 实现 `BackstageRequestStop` 扫描中止

- **文件**：[src/file.rs](../src/file.rs)、[src/line_index.rs](../src/line_index.rs)
- **问题**：后台扫描启动后无法主动取消
- **影响**：大文件扫描期间，重新加载或退出只能等待扫描自然结束
- **修复方案**：
    - 为扫描任务增加取消标记
    - 在行扫描循环中周期性检查取消标记
    - 取消时保证索引状态一致，并返回可诊断状态
- **测试**：已添加 `test_cancelled_build_leaves_empty_scanned_index`、`test_request_stop_scan_clears_index_after_background_scan`、`test_c_ffi_backstage_request_stop_clears_index`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 3.3: 用条件变量替代扫描等待 busy-loop

- **文件**：[src/file.rs](../src/file.rs)
- **问题**：`wait_scan_complete()` 此前使用 `sleep(10ms)` 轮询
- **影响**：等待不够精细，且语义不如 join/通知明确
- **修复方案**：新增内部 `ScanState`，用 Condvar 完成通知替代 `sleep(10ms)` 轮询，保持 `Fjiffyldg::wait_scan()` 公开 API 不变
- **测试**：已添加 `test_scan_state_notifies_waiters_on_finish`
- **状态**：✅ 已完成（2026-05-19）

---

## 📝 第四阶段：细节优化（优先级：低）

### ⏳ Task 4.1: 优化后台扫描数据克隆

- **文件**：[src/file.rs](../src/file.rs)
- **问题**：后台扫描克隆整个文件数据，大文件多一份内存
- **影响**：内存占用翻倍
- **修复方案**：
    - 使用 `Arc<[u8]>` 共享数据而非克隆
    - 或使用引用计数的内存映射
- **测试**：已添加 `test_scan_buffer_reuses_small_file_storage`、`test_scan_buffer_reuses_mmap_storage`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 4.2: 实现 `overstep` 溢出处理

- **文件**：[src/line_index.rs](../src/line_index.rs)
- **问题**：`overstep_pos` 字段定义但未使用，极端超大文件会丢失行信息
- **影响**：极端超大文件的按位置查询无法正确裁剪到 chunk 溢出段
- **修复方案**：
    - 在 `build_from_data()` 中填充 `overstep_pos`
    - 在查询函数中使用溢出信息
- **测试**：已添加 `test_overstep_position_records_first_chunk_overflow`、`test_overstep_position_narrows_search_after_last_chunk`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 4.3: 完善 `GetUtf8TextCharCount` 指针推进

- **文件**：[src/encoding.rs](../src/encoding.rs)
- **问题**：Rust 版本不更新调用者位置（C++ 版本更新 `const char**` 指针）
- **影响**：调用方无法获知已处理的字节数
- **修复方案**：新增 `get_utf8_char_count_with_offset()`，保留原 `get_utf8_char_count()` 返回值兼容性
- **测试**：已添加 `test_utf8_char_count_reports_consumed_bytes`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 4.4: 实现 `read_line_cut()` 超长行截断

- **文件**：[src/file.rs](../src/file.rs)
- **问题**：缺少 >4KB 超长行截断，与 C++ `ReadFileDataLLineCut` 语义不一致
- **影响**：超长行处理不一致
- **修复方案**：新增 `read_line_cut()` 对齐 C++ `ReadFileDataLLineCut`：短行批量读取，长行按 4KB 临界值截断；保留 `read_line()` 作为单行读取 helper
- **测试**：已添加 `test_read_line_cut_batches_short_lines`、`test_read_line_defaults_to_long_line_cutoff`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 4.5: 改进 `is_loaded()` 返回值

- **文件**：[src/file.rs](../src/file.rs)、[src/lib.rs](../src/lib.rs)
- **问题**：返回 `bool` 而非错误码，调用方无法区分"未加载"和"错误"
- **影响**：错误诊断不清晰
- **修复方案**：保留 `is_loaded()` 作为 bool 快捷查询，新增 `load_status() -> Result<bool>` 返回未加载、已加载或最近一次加载错误
- **测试**：已添加 `test_load_status_reports_unloaded_error_and_loaded`
- **状态**：✅ 已完成（2026-05-19）

---

## 📊 进度追踪

| 阶段     | 任务数 | 完成   | 进度    |
| -------- | ------ | ------ | ------- |
| 第一阶段 | 3      | 3      | 100%    |
| 第二阶段 | 4      | 4      | 100%    |
| 第三阶段 | 3      | 3      | 100%    |
| 第四阶段 | 5      | 5      | 100%    |
| 版本路线 | 2      | 0      | 0%      |
| 发布配套 | 4      | 4      | 100%    |
| 文档配套 | 4      | 3      | 75%     |
| 开源治理 | 10     | 7      | 70%     |
| **总计** | **35** | **29** | **83%** |

---

## 📦 发布配套任务

### ✅ Task R.1: 使用 cbindgen 生成 C 头文件与 C smoke 编译

- **文件**：[cbindgen.toml](../cbindgen.toml)、[include/fjiffyldg.h](../include/fjiffyldg.h)、[scripts/generate_c_header.ps1](../scripts/generate_c_header.ps1)、[tests/c_smoke.c](../tests/c_smoke.c)、[scripts/check_c_abi.ps1](../scripts/check_c_abi.ps1)
- **要求**：公共 C API 头文件必须由 `cbindgen` 从 Rust FFI 源码生成，不再手动编写；检查脚本需验证生成结果未过期
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task R.2: 补齐 C++ 参考头文件兼容层

- **文件**：[cbindgen.toml](../cbindgen.toml)、[include/fjiffyldg.h](../include/fjiffyldg.h)、[tests/cpp_smoke.cpp](../tests/cpp_smoke.cpp)、[scripts/check_c_abi.ps1](../scripts/check_c_abi.ps1)
- **问题**：Rust 发布头文件此前缺少 `FJIFFYLDG_API`、`fjiffyldg_ptr` 与 C++ RAII 包装入口
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task R.3: `GetFileMappedHuge` 真实 mmap 指针语义

- **文件**：[src/ffi.rs](../src/ffi.rs)
- **问题**：Rust FFI 曾返回内部 `Vec<u8>` 拷贝，C++ 参考实现返回由句柄持有的真实文件映射指针
- **影响**：超大文件场景下会产生额外内存拷贝，语义未完全覆盖
- **修复方案**：在 FFI 句柄中持有 `Mmap` 资源，`GetFileMappedHuge` 返回映射指针，`ClearHugeBuffer` 释放映射；Rust 高层 `get_huge_buffer()` 保留安全拷贝 API
- **测试**：已添加 `get_file_mapped_huge_retains_mmap_until_clear`
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task R.4: 补齐 C/C++ 链接运行 smoke 验证

- **文件**：[scripts/check_c_abi.ps1](../scripts/check_c_abi.ps1)、[tests/c_smoke.c](../tests/c_smoke.c)、[tests/cpp_smoke.cpp](../tests/cpp_smoke.cpp)
- **问题**：此前 C/C++ smoke 仅验证头文件可编译为 object，未验证 release 动态库可被真实链接和调用
- **修复方案**：检查脚本在验证 cbindgen 头文件后构建 release 库，分别链接 C 与 C++ smoke 可执行文件，并运行加载、扫描、行查询、读取、huge mmap、编码工具和 C++ RAII wrapper 的最小闭环
- **状态**：✅ 已完成（2026-05-20）

---

## 📝 文档配套任务

### ✅ Task D.1: 添加便于开发者使用的双语文档

- **文件**：[README.md](../README.md)、[README_EN.md](../README_EN.md)、[DEVELOPMENT_TODO.md](DEVELOPMENT_TODO.md)、[DEVELOPMENT_TODO_EN.md](DEVELOPMENT_TODO_EN.md)、[功能覆盖深度检查报告.md](功能覆盖深度检查报告.md)、[feature_coverage_depth_report.md](feature_coverage_depth_report.md)
- **要求**：所有项目文档都需要提供中文和英文两个版本，并在文档头部链接到另一个语言版本
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task D.2: 补充 C API 使用双语文档

- **文件**：[c_api_usage.md](c_api_usage.md)、[c_api_usage_en.md](c_api_usage_en.md)
- **要求**：详细说明 `cbindgen` 头文件生成、库构建、C/C++ 编译链接、句柄生命周期、加载扫描、读取、huge mmap、编码工具、错误码和维护流程
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task D.3: C API 使用指南覆盖全部接口

- **文件**：[c_api_usage.md](c_api_usage.md)、[c_api_usage_en.md](c_api_usage_en.md)、[include/fjiffyldg.h](../include/fjiffyldg.h)、[src/ffi.rs](../src/ffi.rs)
- **要求**：C API 使用指南必须包含 `include/fjiffyldg.h` / `src/ffi.rs` 导出的全部接口，逐项说明签名、参数、返回值、错误码、指针或缓冲区生命周期和注意事项
- **状态**：✅ 已完成（2026-05-20）

### ⏳ Task D.4: 保持双语文档同步

- **要求**：后续新增或修改项目文档时，同一提交内同步更新对应语言版本
- **状态**：⏳ 持续执行

---

## 🌐 开源治理与发布工程任务

基于 Rust API Guidelines、Cargo 发布规范、docs.rs 元数据、GitHub community health 文件、Keep a Changelog、SemVer 与 C ABI 稳定性实践，后续按以下清单补齐完善开源库项目结构。

### ✅ Task O.1: 建立跨平台 CI 质量门禁

- **文件**：[.github/workflows/ci.yml](../.github/workflows/ci.yml)
- **要求**：覆盖 Windows/Linux/macOS，运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`、`cargo doc --all-features --no-deps`、benchmark 编译、C/C++ ABI smoke、`cargo publish --dry-run`
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task O.2: 补齐贡献指南与安全政策

- **文件**：[CONTRIBUTING.md](CONTRIBUTING.md)、[CONTRIBUTING_EN.md](CONTRIBUTING_EN.md)、[SECURITY.md](SECURITY.md)、[SECURITY_EN.md](SECURITY_EN.md)
- **要求**：说明开发环境、提交前检查、C API 修改规则、双语文档同步、漏洞报告方式、支持版本和安全边界
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task O.3: 建立变更日志

- **文件**：[CHANGELOG.md](CHANGELOG.md)、[CHANGELOG_EN.md](CHANGELOG_EN.md)
- **要求**：采用 Keep a Changelog 结构，保留未发布区段并记录首个 `0.1.0` 能力基线
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task O.4: 完善 Cargo 发布元数据

- **文件**：[Cargo.toml](../Cargo.toml)
- **要求**：补充 `rust-version`、`documentation`、docs.rs metadata、crates.io 合规 keywords/categories，并通过 `exclude` 避免打包参考源码和 CI 配置
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task O.5: 补齐社区协作模板

- **文件**：`.github/ISSUE_TEMPLATE/*`、`.github/PULL_REQUEST_TEMPLATE.md`
- **要求**：提供 bug、feature、C ABI、性能回归 issue 模板和 PR checklist
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task O.6: 增加依赖自动更新工作流

- **文件**：[.github/dependabot.yml](../.github/dependabot.yml)
- **要求**：使用 Dependabot 自动检查 GitHub Actions 与 Cargo 依赖更新，并通过缓存与分组减少依赖更新噪音
- **状态**：✅ 已完成（2026-05-20）

### ✅ Task O.7: 增加标签发布工作流

- **文件**：[.github/workflows/release.yml](../.github/workflows/release.yml)
- **要求**：在 tag push 时跨平台构建、测试/校验、生成并附带头文件、发布 crates.io 库，并使用缓存优化构建效率
- **状态**：✅ 已完成（2026-05-20）

### ⏳ Task O.8: 补齐格式化、换行和仓库属性配置

- **文件**：[rustfmt.toml](../rustfmt.toml)、[.editorconfig](../.editorconfig)、[.gitattributes](../.gitattributes)
- **要求**：统一 Rust/Markdown/PowerShell 格式、换行符策略、生成文件标记与 reference 目录 linguist 处理
- **状态**：⏳ 待完成

### ⏳ Task O.9: 建立发布流程与 SemVer/ABI 规则

- **文件**：`docs/release.md`、`docs/release_en.md`
- **要求**：说明版本号、发布前检查、`cargo package --list`、`cargo publish --dry-run`、tag、GitHub Release、docs.rs 检查、C ABI 兼容与 yank 流程
- **状态**：⏳ 待完成

### ⏳ Task O.10: 增强 README 项目可信度入口

- **文件**：[README.md](README.md)、[README_EN.md](README_EN.md)
- **要求**：增加 CI、crates.io、docs.rs、license、MSRV 徽章或链接，并明确贡献、安全、变更日志与发布状态入口
- **状态**：⏳ 待完成

---

## 🧭 版本路线与兼容策略

### ⏳ Task V.1: v1.0 严格对应 C++ 可观察行为

- **文件**：[功能覆盖深度检查报告.md](功能覆盖深度检查报告.md)、[feature_coverage_depth_report.md](feature_coverage_depth_report.md)、[tests/](../tests/)、[include/fjiffyldg.h](../include/fjiffyldg.h)、[src/ffi.rs](../src/ffi.rs)
- **目标**：第一个正式版本优先建立“Rust 版可可靠替代 C++ 版”的可信基线。
- **要求**：
    - 公开 C ABI 的函数名、签名、返回值、错误码、空指针、越界、空文件等边界行为与 C++ 参考实现严格对齐
    - 锁定 `ReadFileData*`、`GetFileMappedHuge`、UTF 检测、行索引查询和文件操作的指针生命周期与输出语义
    - 建立 C++/Rust 对照清单与回归测试；内部路线只在影响可观察行为或性能承诺时强制对齐
- **进展**：已补首批 C ABI 边界回归测试，并修正七处 Rust 偏差：未建立任何行偏移时 `GetFileLineIndex` 错误返回 `0`，查询位置超过文件末尾时误返回最后一行，`ReadFileData` 在负偏移时未钳到文件起点、在 EOF/越过 EOF 时错误返回失败而非空缓冲区，`ReadFileDataEndOfLine` 在行尾边界误读到下一段数据，`ReadFileDataLLineCut` 失败时错误覆盖调用方已有边界值，以及 `GetFileMappedHuge` 在失败调用后错误保留旧映射的问题。
- **状态**：⏳ 持续执行

### ⏳ Task V.2: v1.1+ 保持 ABI 兼容并逐步 Rust 化

- **文件**：[release.md](release.md)、[release_en.md](release_en.md)、[CHANGELOG.md](CHANGELOG.md)、[CHANGELOG_EN.md](CHANGELOG_EN.md)
- **目标**：v1.0 可信基线建立后，在不破坏公开 ABI 行为的前提下优化内部实现。
- **要求**：
    - 通过 SemVer/ABI 规则区分行为兼容、性能优化和破坏性变化
    - Rust 化内部线程、资源管理、错误封装和性能路径时，必须保留 v1.0 对照测试
    - 重大内部路线调整需要补充 benchmark 或兼容性说明
- **状态**：⏳ 后续执行

---

## �🎯 建议执行策略

1. **v1.0 可信基线**：优先完成 Task V.1，将 C++ 参考实现的公开行为固化为对照清单、回归测试和发布说明
2. **发布前治理**：并行补齐 O.8、O.9、O.10，确保格式、发布流程、README 信号与严格对应目标一致
3. **v1.1+ 内部优化**：完成 Task V.2，在 ABI 行为不破坏的前提下逐步 Rust 化内部路线并补充 benchmark

---

## 📚 参考资源

- **功能覆盖报告**：[功能覆盖深度检查报告.md](功能覆盖深度检查报告.md) / [feature_coverage_depth_report.md](feature_coverage_depth_report.md)
- **开发 TODO**：[DEVELOPMENT_TODO.md](DEVELOPMENT_TODO.md) / [DEVELOPMENT_TODO_EN.md](DEVELOPMENT_TODO_EN.md)
- **C++ 参考实现**：[reference/fjiffyldg/Fjiffyldg/](../reference/fjiffyldg/Fjiffyldg/)
- **Rust 源代码**：[src/](../src/)
- **测试文件**：[tests/](../tests/)（包含 C ABI smoke 输入）
- **C API 使用指南**：[c_api_usage.md](c_api_usage.md) / [c_api_usage_en.md](c_api_usage_en.md)
- **C/C++ 头文件**：[include/fjiffyldg.h](../include/fjiffyldg.h)，由 [cbindgen.toml](../cbindgen.toml) 和 [scripts/generate_c_header.ps1](../scripts/generate_c_header.ps1) 生成，运行 `pwsh -File scripts/check_c_abi.ps1` 验证生成结果、声明编译、动态库链接与 smoke 运行
- **大文件基准**：[benches/large_file.rs](../benches/large_file.rs)，运行 `cargo bench --bench large_file`
- **贡献指南**：[CONTRIBUTING.md](CONTRIBUTING.md) / [CONTRIBUTING_EN.md](CONTRIBUTING_EN.md)
- **安全政策**：[SECURITY.md](SECURITY.md) / [SECURITY_EN.md](SECURITY_EN.md)
- **变更日志**：[CHANGELOG.md](CHANGELOG.md) / [CHANGELOG_EN.md](CHANGELOG_EN.md)

---

## 🔗 相关 Issue 和 PR

（待补充）

---

**最后更新**：2026-05-21（已同步第一阶段、百万行索引修复、chunk 填充与按行号/按位置查询范围裁剪、overstep 查询范围裁剪、1GB windowed mmap 读取与扫描、UTF-8 offset、read_line_cut、UTF-16 offset 重扫、UTF-16BE/UTF-32 行扫描补测、加载状态 API、Condvar 扫描等待、C/C++ FFI、cbindgen 头文件生成与链接运行 smoke、huge mmap 指针语义、Criterion 大文件基准入口、双语文档规范、C API 双语使用文档、C API 全接口参考、开源治理 TODO、CI、贡献指南、安全政策、变更日志、Cargo 发布元数据、Issue/PR 模板、依赖自动更新工作流、标签发布工作流、v1.0 严格对应与 v1.1+ Rust 化路线）
**维护者**：开发团队
