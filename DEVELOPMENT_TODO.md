# Rust 版本开发 TODO 清单

基于功能覆盖深度检查报告，以下是后续开发的优先级任务清单。

---

## 📋 第一阶段：核心功能修复（优先级：高）

### ✅ Task 1.1: 修复 CRLF 行长度计算

- **文件**：[src/line_index.rs](src/line_index.rs)
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

- **文件**：[src/encoding.rs](src/encoding.rs)
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

- **文件**：[src/file.rs](src/file.rs)、[src/lib.rs](src/lib.rs)
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

### 🔄 Task 2.1: 实现分块索引真正的填充和查询

- **文件**：[src/line_index.rs](src/line_index.rs)
- **问题**：`chunks` 字段定义了但 `build_from_data()` 未填充，查询函数未使用
- **影响**：>100 万行文件定位性能从 `O(log n)` 退化到 `O(n)`
- **修复方案**：
  1. 在 `build_from_data()` 中填充 `chunks` 向量
  2. 修改 `get_line_pos()` 使用三层索引查询
  3. 添加分块索引的二分查找逻辑
- **测试**：添加大文件（>100 万行）性能测试
- **状态**：🔄 部分完成（已修复直接索引上限后的扩展索引承接，chunk 查询仍待实现）
- **复杂度**：高

---

### ✅ Task 2.2: 修正 `CHUNK_COUNT_MAX` 值

- **文件**：[src/line_index.rs](src/line_index.rs)
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

### ⏳ Task 2.3: 实现大文件分块映射

- **文件**：[src/file.rs](src/file.rs)
- **问题**：仅一次映射整个文件，超大文件受限地址空间
- **影响**：无法处理超过地址空间限制的文件
- **修复方案**：
  - 实现 1GB 分块映射（`MMAP_FILECHUNK`）
  - 维护多个 mmap 句柄
  - 在读取时动态切换分块
- **测试**：添加超大文件（>4GB）映射测试
- **状态**：⏳ 待开始
- **复杂度**：高

---

### ⏳ Task 2.4: 优化文件操作的大文件 mmap

- **文件**：[src/file.rs](src/file.rs)
- **问题**：`clone_file`/`save_file` 等未对大文件使用 mmap
- **影响**：大文件操作性能不佳
- **修复方案**：
  - 对 >10MB 文件使用 mmap 加速 copy
  - 使用 `memcpy` 或 `sendfile` 优化
- **测试**：添加大文件 copy 性能测试
- **状态**：⏳ 待开始

---

## 🔧 第三阶段：完整性增强（优先级：中）

### ✅ Task 3.1: 添加 C FFI 绑定

- **文件**：[src/lib.rs](src/lib.rs)、建议新增 `src/ffi.rs`
- **问题**：Rust 当前只有原生 API，没有 `extern "C"` 导出函数
- **影响**：无法覆盖 C++ 版本面向 C ABI 的调用方式
- **修复方案**：新增 `src/ffi.rs`，导出 opaque handle API、加载/扫描/等待/行查询/读取、编码检查与基础文件操作 C ABI；保留 `crate-type = ["lib", "cdylib", "staticlib"]`
- **测试**：已添加 `test_c_ffi_smoke_load_scan_query_and_read`
- **状态**：✅ 已完成（2026-05-19；后续可补真实 C 集成测试与发布用头文件）

---

### ⏳ Task 3.2: 实现 `BackstageRequestStop` 扫描中止

- **文件**：[src/file.rs](src/file.rs)、[src/line_index.rs](src/line_index.rs)
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

- **文件**：[src/file.rs](src/file.rs)
- **问题**：`wait_scan_complete()` 此前使用 `sleep(10ms)` 轮询
- **影响**：等待不够精细，且语义不如 join/通知明确
- **修复方案**：新增内部 `ScanState`，用 Condvar 完成通知替代 `sleep(10ms)` 轮询，保持 `Fjiffyldg::wait_scan()` 公开 API 不变
- **测试**：已添加 `test_scan_state_notifies_waiters_on_finish`
- **状态**：✅ 已完成（2026-05-19）

---

## 📝 第四阶段：细节优化（优先级：低）

### ⏳ Task 4.1: 优化后台扫描数据克隆

- **文件**：[src/file.rs](src/file.rs)
- **问题**：后台扫描克隆整个文件数据，大文件多一份内存
- **影响**：内存占用翻倍
- **修复方案**：
  - 使用 `Arc<[u8]>` 共享数据而非克隆
  - 或使用引用计数的内存映射
- **测试**：已添加 `test_scan_buffer_reuses_small_file_storage`、`test_scan_buffer_reuses_mmap_storage`
- **状态**：✅ 已完成（2026-05-19）

---

### ⏳ Task 4.2: 实现 `overstep` 溢出处理

- **文件**：[src/line_index.rs](src/line_index.rs)
- **问题**：`overstep_pos` 字段定义但未使用，极端超大文件会丢失行信息
- **影响**：极端超大文件（>2^64 字节）处理不完整
- **修复方案**：
  - 在 `build_from_data()` 中填充 `overstep_pos`
  - 在查询函数中使用溢出信息
- **测试**：添加极端超大文件测试（模拟）
- **状态**：⏳ 待开始

---

### ✅ Task 4.3: 完善 `GetUtf8TextCharCount` 指针推进

- **文件**：[src/encoding.rs](src/encoding.rs)
- **问题**：Rust 版本不更新调用者位置（C++ 版本更新 `const char**` 指针）
- **影响**：调用方无法获知已处理的字节数
- **修复方案**：新增 `get_utf8_char_count_with_offset()`，保留原 `get_utf8_char_count()` 返回值兼容性
- **测试**：已添加 `test_utf8_char_count_reports_consumed_bytes`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 4.4: 实现 `read_line_cut()` 超长行截断

- **文件**：[src/file.rs](src/file.rs)
- **问题**：缺少 >4KB 超长行截断，与 C++ `ReadFileDataLLineCut` 语义不一致
- **影响**：超长行处理不一致
- **修复方案**：新增 `read_line_cut()` 对齐 C++ `ReadFileDataLLineCut`：短行批量读取，长行按 4KB 临界值截断；保留 `read_line()` 作为单行读取 helper
- **测试**：已添加 `test_read_line_cut_batches_short_lines`、`test_read_line_defaults_to_long_line_cutoff`
- **状态**：✅ 已完成（2026-05-19）

---

### ✅ Task 4.5: 改进 `is_loaded()` 返回值

- **文件**：[src/file.rs](src/file.rs)、[src/lib.rs](src/lib.rs)
- **问题**：返回 `bool` 而非错误码，调用方无法区分"未加载"和"错误"
- **影响**：错误诊断不清晰
- **修复方案**：保留 `is_loaded()` 作为 bool 快捷查询，新增 `load_status() -> Result<bool>` 返回未加载、已加载或最近一次加载错误
- **测试**：已添加 `test_load_status_reports_unloaded_error_and_loaded`
- **状态**：✅ 已完成（2026-05-19）

---

## 📊 进度追踪

| 阶段     | 任务数 | 完成                     | 进度               |
| -------- | ------ | ------------------------ | ------------------ |
| 第一阶段 | 3      | 3                        | 100%               |
| 第二阶段 | 4      | 1（另 1 个部分完成）     | 25% + 部分完成     |
| 第三阶段 | 3      | 3                        | 100%               |
| 第四阶段 | 5      | 4                        | 80%                |
| **总计** | **15** | **11（另 1 个部分完成）** | **73% + 部分完成** |

---

## 🎯 建议执行策略

1. **快速赢**（第一周）：完成第一阶段 3 个任务，修复核心功能缺陷
2. **性能突破**（第二周）：完成第二阶段 4 个任务，实现分块索引和大文件优化
3. **完整性**（第三周）：完成第三阶段 3 个任务，添加 FFI 和扫描控制
4. **细节打磨**（第四周）：完成第四阶段 5 个任务，优化边界情况

---

## 📚 参考资源

- **功能覆盖报告**：[docs/功能覆盖深度检查报告.md](docs/功能覆盖深度检查报告.md)
- **C++ 参考实现**：[reference/fjiffyldg/Fjiffyldg/](reference/fjiffyldg/Fjiffyldg/)
- **Rust 源代码**：[src/](src/)
- **测试文件**：[tests/](tests/)（如果存在）

---

## 🔗 相关 Issue 和 PR

（待补充）

---

**最后更新**：2026-05-19（已同步第一阶段、百万行索引修复、UTF-8 offset、read_line_cut、UTF-16 offset 重扫、UTF-16BE/UTF-32 行扫描补测、加载状态 API、Condvar 扫描等待与 C FFI 核心绑定）
**维护者**：开发团队
