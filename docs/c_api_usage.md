# C API 使用指南

[English](c_api_usage_en.md)

本文说明如何从 C 或 C++ 项目使用 Fjiffyldg 的 C ABI。公共头文件由 `cbindgen` 从 Rust FFI 源码生成，不应手动编辑。

## 1. 生成头文件

安装 `cbindgen`：

```powershell
cargo install cbindgen --locked
```

生成公共头文件：

```powershell
pwsh -File scripts/generate_c_header.ps1
```

生成结果位于 [include/fjiffyldg.h](../include/fjiffyldg.h)。发布或提交前建议验证头文件未过期：

```powershell
pwsh -File scripts/generate_c_header.ps1 -Verify
```

[cbindgen.toml](../cbindgen.toml) 控制导出宏、include guard、C++ 兼容包装和函数声明格式。

## 2. 构建 Rust 库

生成动态库或静态库：

```powershell
cargo build --release
```

`Cargo.toml` 中的 crate 类型包含 `cdylib` 和 `staticlib`，因此 release 目录会生成平台对应的动态库和静态库。Windows 上通常会生成 `fjiffyldg.dll`、`fjiffyldg.dll.lib` 或 `libfjiffyldg.a`；Linux/macOS 上通常会生成 `libfjiffyldg.so`、`libfjiffyldg.dylib` 或 `libfjiffyldg.a`。

## 3. 编译调用方

C 调用方需要包含生成的头文件目录，并链接 release 构建产物。示例：

```bash
cc -std=c11 -I include app.c -L target/release -lfjiffyldg -o app
```

C++ 调用方也可以直接包含同一个头文件：

```bash
g++ -std=c++17 -I include app.cpp -L target/release -lfjiffyldg -o app
```

仓库内置的 smoke 检查会先验证头文件由 `cbindgen` 生成且未过期，再分别用 C 和 C++ 编译器编译 smoke 输入；随后构建 release 动态库，链接 C/C++ smoke 可执行文件，并运行最小调用闭环：

```powershell
pwsh -File scripts/check_c_abi.ps1
```

该脚本覆盖加载、扫描、行查询、读取、huge mmap、编码工具和 C++ RAII wrapper 的基本可用性。Windows 上会把 `target/release` 临时加入 `PATH`，以便 smoke 可执行文件能加载 `fjiffyldg.dll`。

## 4. 句柄生命周期

C API 使用不透明句柄 `fjiffyldg_ptr`。调用方只持有指针，不访问内部字段。

```c
#include "fjiffyldg.h"

int main(void) {
    fjiffyldg_ptr fm = fjiffyldg_create();
    if (fm == 0) {
        return 1;
    }

    fjiffyldg_clear(fm);
    return 0;
}
```

生命周期规则：

- 使用 `fjiffyldg_create()` 创建句柄。
- 每个成功创建的句柄必须调用一次 `fjiffyldg_clear()` 释放。
- 传入空句柄时，多数查询会返回错误码或空指针。
- 不要复制、释放或写入 API 返回的内部缓冲区指针。

## 5. 加载、扫描和行查询

```c
#include "fjiffyldg.h"
#include <stdint.h>
#include <stdio.h>

int main(void) {
    fjiffyldg_ptr fm = fjiffyldg_create();
    if (fm == 0) {
        return 1;
    }

    int code = LoadAndScanFile(fm, "large_file.txt");
    if (code != 0) {
        fjiffyldg_clear(fm);
        return code;
    }

    WaitFileScanTaskFinished(fm);

    long long lines = GetFileLineCount(fm);
    long long pos = GetFileLinePos(fm, 0);
    long long len64 = GetFileLineLength(fm, 0);
    printf("lines=%lld first_pos=%lld first_len=%lld\n", lines, pos, len64);

    fjiffyldg_clear(fm);
    return 0;
}
```

常用查询：

| 函数                       | 作用                           |
| -------------------------- | ------------------------------ |
| `LoadAndScanFile`          | 加载文件并启动后台行扫描       |
| `LoadFileOnly`             | 只加载文件，不启动行扫描       |
| `WaitFileScanTaskFinished` | 等待后台扫描结束               |
| `BackstageRequestStop`     | 请求停止扫描并清空当前行索引   |
| `GetFileLineCount`         | 获取总行数                     |
| `GetFileLinePos`           | 获取指定行起始字节偏移         |
| `GetFileLineLength`        | 获取指定行内容长度，不含换行符 |
| `GetFileLineIndex`         | 根据字节位置查找所在行         |

## 6. 读取数据

`ReadFileData` 的 `len` 是输入输出参数：调用前表示希望读取的最大字节数，返回后表示实际读取字节数。

当 `pos` 超出文件有效范围时，行为会对齐 C++ 参考实现：负偏移会被钳到文件起点，位于文件末尾或超过文件末尾时会被钳到 EOF，并返回空缓冲区，`len` 写回为 `0`。

```c
unsigned int len = 1024;
const char *data = ReadFileData(fm, 0, &len);
if (data != 0) {
    fwrite(data, 1, len, stdout);
}
```

缓冲区规则：

- 返回指针由 `fjiffyldg_ptr` 内部持有。
- 下一次读取可能覆盖上一次读取结果。
- 调用方如需长期保存内容，应立即复制到自己的缓冲区。
- 调用方不得 `free()` 返回指针。

`ReadFileDataLLineCut` 按 C++ 参考实现语义读取短行批量数据，并对超长行执行 4KB 截断；如果行查找失败，则返回空指针、将 `len` 写回为 `0`，并保留调用方原有的 `index`、`bpos`、`epos`。`ReadFileDataEndOfLine` 可从指定行内位置读取到当前行末尾；如果 `pos` 恰好位于当前行尾，则返回空缓冲区并将 `len` 写回为 `0`。

## 7. Huge mmap 指针

`GetFileMappedHuge` 返回由句柄持有的真实文件 mmap 指针，并通过 `bufferSize` 返回映射大小。

```c
long long size = 0;
const char *mapped = GetFileMappedHuge(fm, "large_file.txt", &size);
if (mapped != 0) {
    /* mapped[0..size) is valid until ClearHugeBuffer or fjiffyldg_clear. */
    ClearHugeBuffer(fm);
}
```

注意事项：

- 指针只读，调用方不要写入。
- 指针有效期到 `ClearHugeBuffer(fm)`、下一次 `GetFileMappedHuge` 或 `fjiffyldg_clear(fm)`；下一次 `GetFileMappedHuge` 即使失败，也会让上一次返回的指针失效。
- 空文件、打开失败或映射失败会返回空指针，并将 `bufferSize` 置为 0。

## 8. 编码和文件工具函数

| 函数                   | 作用                            |
| ---------------------- | ------------------------------- |
| `CheckTextASCII`       | 检查字节片段是否全为 ASCII      |
| `CheckWholeTextUtf8`   | 检查完整字节片段是否为 UTF-8    |
| `CheckExtractTextUtf8` | 抽样检查文本片段是否为 UTF-8    |
| `GetUtf8TextCharCount` | 统计 UTF-8 字符数并推进输入指针 |
| `GetFileSizeByteCount` | 查询文件大小                    |
| `ToCloneFile`          | 复制文件                        |
| `ToSaveFile`           | 保存缓冲区到文件                |
| `ToAppendFile`         | 追加缓冲区到文件                |
| `ToConcatenateFile`    | 将第二个文件追加到第一个文件    |

## 9. 完整接口参考

本节覆盖 [include/fjiffyldg.h](../include/fjiffyldg.h) / [src/ffi.rs](../src/ffi.rs) 当前导出的全部接口。签名以生成头文件为准；修改 FFI 后必须重新生成头文件并同步本节。

### 9.1 句柄管理

| 接口               | 签名                                     | 参数                     | 返回值                             | 生命周期与注意事项                                             |
| ------------------ | ---------------------------------------- | ------------------------ | ---------------------------------- | -------------------------------------------------------------- |
| `fjiffyldg_create` | `fjiffyldg_ptr fjiffyldg_create(void)`   | 无                       | 成功返回不透明句柄；失败返回空指针 | 返回的句柄由调用方持有，必须传给 `fjiffyldg_clear` 释放        |
| `fjiffyldg_clear`  | `void fjiffyldg_clear(fjiffyldg_ptr fm)` | `fm`：待释放句柄，可为空 | 无                                 | 释放文件模型、读取缓冲区和 huge mmap；调用后不得继续使用该句柄 |

### 9.2 加载、扫描和扫描控制

| 接口                       | 签名                                                                                  | 参数                                                                                       | 返回值                                            | 生命周期与注意事项                                                               |
| -------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------- | -------------------------------------------------------------------------------- |
| `LoadAndScanFile`          | `int LoadAndScanFile(fjiffyldg_ptr fm, const char *name)`                             | `fm`：句柄；`name`：以 NUL 结尾的路径字符串                                                | `0` 成功；非 `0` 为错误码                         | 加载文件并启动后台行扫描；行查询前可调用 `WaitFileScanTaskFinished` 等待完整索引 |
| `LoadFileOnly`             | `int LoadFileOnly(fjiffyldg_ptr fm, const char *name)`                                | `fm`：句柄；`name`：路径字符串                                                             | `0` 成功；非 `0` 为错误码                         | 只加载文件，不启动行扫描；适合只做按字节读取的场景                               |
| `GetFileIsLoaded`          | `int GetFileIsLoaded(fjiffyldg_ptr fm)`                                               | `fm`：句柄                                                                                 | `0` 表示已加载且可用；非 `0` 表示未加载或句柄无效 | 该接口沿用错误码语义，不是布尔 true/false                                        |
| `RestartScanFile`          | `void RestartScanFile(fjiffyldg_ptr fm, const char *name, long long offset, int utf)` | `fm`：句柄；`name`：可用于重新加载/扫描的路径；`offset`：扫描起始字节；`utf`：UTF 模式编号 | 无                                                | 会停止当前扫描并从指定偏移重建索引；无效句柄或路径会被忽略                       |
| `WaitFileScanTaskFinished` | `void WaitFileScanTaskFinished(fjiffyldg_ptr fm)`                                     | `fm`：句柄                                                                                 | 无                                                | 阻塞到后台扫描结束；空句柄直接返回                                               |
| `BackstageRequestStop`     | `void BackstageRequestStop(fjiffyldg_ptr fm)`                                         | `fm`：句柄                                                                                 | 无                                                | 请求停止后台扫描并清空当前行索引；适合重新加载或退出前调用                       |

### 9.3 行索引查询

| 接口                | 签名                                                             | 参数                            | 返回值                                                                   | 生命周期与注意事项                                                                                                |
| ------------------- | ---------------------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| `GetFileLineCount`  | `long long GetFileLineCount(fjiffyldg_ptr fm)`                   | `fm`：句柄                      | `>0` 表示扫描完成后的总行数；`0` 表示尚未开始扫描；`<0` 表示扫描中或失败 | `LoadFileOnly` 后未构建行索引时返回 `0`；后台扫描进行中通常返回负数；空文件仅在扫描路径下按 C++ 语义视为 1 行     |
| `GetFileLinePos`    | `long long GetFileLinePos(fjiffyldg_ptr fm, long long index)`    | `fm`：句柄；`index`：0 起始行号 | 行起始字节偏移；失败或越界返回 `-1`                                      | 偏移基于原始文件字节，不是字符数；空文件的第 0 行起始位置为 `0`；后台扫描进行中时，已建立的前缀行位置可以提前可见 |
| `GetFileLineLength` | `long long GetFileLineLength(fjiffyldg_ptr fm, long long index)` | `fm`：句柄；`index`：0 起始行号 | 行内容长度；失败或越界返回 `-1`                                          | 长度不包含 `\n` 或 `\r\n` 行尾；空文件的第 0 行长度为 `0`                                                         |
| `GetFileLineIndex`  | `long long GetFileLineIndex(fjiffyldg_ptr fm, long long pos)`    | `fm`：句柄；`pos`：字节偏移     | 所在行号；失败或越界返回 `-1`                                            | 位置查询使用字节偏移；UTF 文本也不按字符计数；空文件中 `pos = 0` 返回 `0`                                         |

### 9.4 读取和 mmap 缓冲区

| 接口                    | 签名                                                                                                                        | 参数                                                                           | 返回值                                             | 生命周期与注意事项                                                                                                                      |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `ReadFileData`          | `const char *ReadFileData(fjiffyldg_ptr fm, long long pos, unsigned int *len)`                                              | `fm`：句柄；`pos`：起始字节；`len`：输入最大读取长度、输出实际长度             | 成功返回内部缓冲区指针；失败返回空指针             | 指针由句柄持有，下一次读取可能覆盖；调用方需要长期保存时必须复制；负偏移会钳到文件起点，`pos` 到达或越过 EOF 时返回空缓冲区且 `len = 0` |
| `ReadFileDataLLineCut`  | `const char *ReadFileDataLLineCut(fjiffyldg_ptr fm, long long *index, long long *bpos, long long *epos, unsigned int *len)` | `index`：输入/输出行号；`bpos`/`epos`：输入/输出读取边界；`len`：输入/输出长度 | 成功返回内部缓冲区指针；失败返回空指针             | 对短行批量读取，对超长行按 4KB 语义截断；成功时推进输出参数，失败时仅将 `len` 置为 `0` 并保留原有 `index`/`bpos`/`epos`                 |
| `ReadFileDataEndOfLine` | `const char *ReadFileDataEndOfLine(fjiffyldg_ptr fm, long long index, long long pos, unsigned int *len)`                    | `index`：行号；`pos`：行内或文件字节位置；`len`：输入/输出长度                 | 成功返回内部缓冲区指针；失败返回空指针             | 从指定位置读取到当前行末尾；若 `pos` 已在该行末尾，则返回空缓冲区且 `len = 0`                                                           |
| `GetFileMappedHuge`     | `const char *GetFileMappedHuge(fjiffyldg_ptr fm, const char *fileName, long long *bufferSize)`                              | `fm`：句柄；`fileName`：路径；`bufferSize`：输出映射大小                       | 成功返回只读 mmap 指针；失败返回空指针并将大小置 0 | 指针有效期到 `ClearHugeBuffer`、下一次 `GetFileMappedHuge` 或 `fjiffyldg_clear`；新的 `GetFileMappedHuge` 调用即使失败也会使旧指针失效  |
| `ClearHugeBuffer`       | `void ClearHugeBuffer(fjiffyldg_ptr fm)`                                                                                    | `fm`：句柄                                                                     | 无                                                 | 释放句柄持有的 huge mmap；此前返回的 mmap 指针立即失效                                                                                  |

### 9.5 编码检查

| 接口                   | 签名                                                                     | 参数                                               | 返回值                                                     | 生命周期与注意事项                                                       |
| ---------------------- | ------------------------------------------------------------------------ | -------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------ |
| `CheckTextASCII`       | `unsigned int CheckTextASCII(const char *text, unsigned int len)`        | `text`：字节指针；`len`：长度                      | `0` 表示全部为 ASCII；非 `0` 表示发现非 ASCII 的位置或距离 | `text` 为空且 `len > 0` 时返回错误值；不要把返回值当作布尔 true 表示成功 |
| `CheckWholeTextUtf8`   | `unsigned int CheckWholeTextUtf8(const char *text, unsigned int len)`    | `text`：完整字节片段；`len`：长度                  | `0` 表示有效 UTF-8；非 `0` 表示无效位置或距离              | 适合完整校验输入缓冲区                                                   |
| `CheckExtractTextUtf8` | `unsigned int CheckExtractTextUtf8(const char *text, unsigned int len)`  | `text`：字节片段；`len`：长度                      | `0` 表示抽样范围有效；非 `0` 表示抽样发现无效 UTF-8        | 用于快速抽样判断，不能替代需要严格保证时的完整校验                       |
| `GetUtf8TextCharCount` | `unsigned int GetUtf8TextCharCount(const char **text, unsigned int len)` | `text`：输入/输出字节指针地址；`len`：最大检查长度 | 返回 UTF-8 字符数，并把 `*text` 推进到已消费位置           | `text` 或 `*text` 为空时返回 0；调用方可用指针差计算已消费字节数         |

### 9.6 文件工具

| 接口                   | 签名                                                                         | 参数                                                            | 返回值                                             | 生命周期与注意事项                                         |
| ---------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------- | ---------------------------------------------------------- |
| `GetFileSizeByteCount` | `long long GetFileSizeByteCount(const char *name)`                           | `name`：路径字符串                                              | 成功返回文件大小；失败返回错误码对应的负值或错误值 | 返回单位是字节                                             |
| `ToCloneFile`          | `int ToCloneFile(const char *oldFileName, const char *newFileName)`          | `oldFileName`：源路径；`newFileName`：目标路径                  | `0` 成功；非 `0` 为错误码                          | 目标文件会被创建或覆盖，具体行为遵循 Rust 文件操作实现     |
| `ToSaveFile`           | `int ToSaveFile(const char *fileName, const char *buffer, long long len)`    | `fileName`：目标路径；`buffer`：待写入字节；`len`：字节数       | `0` 成功；非 `0` 为错误码                          | 保存 `buffer[0..len)` 到文件；`len < 0` 或空指针会返回错误 |
| `ToAppendFile`         | `int ToAppendFile(const char *fileName, const char *buffer, long long len)`  | `fileName`：目标路径；`buffer`：待追加字节；`len`：字节数       | `0` 成功；非 `0` 为错误码                          | 文件不存在时会创建；调用方保留缓冲区所有权                 |
| `ToConcatenateFile`    | `int ToConcatenateFile(const char *catFileName, const char *appendFileName)` | `catFileName`：被追加的目标文件；`appendFileName`：读取来源文件 | `0` 成功；非 `0` 为错误码                          | 将第二个文件内容追加到第一个文件末尾                       |

## 10. 错误码约定

多数返回 `int` 的函数使用 0 表示成功，非 0 表示错误。常见错误码与 Rust 内部错误类型保持映射，例如：

| 错误码 | 含义                 |
| ------ | -------------------- |
| `0`    | 成功                 |
| `-1`   | 文件未加载或句柄无效 |
| `1`    | 文件不可访问         |
| `2`    | 流错误               |
| `3`    | 内存映射错误         |

返回指针的函数通常用空指针表示失败，并通过输出参数返回 0 或保留错误上下文。

## 11. 维护生成文件

修改 [src/ffi.rs](../src/ffi.rs) 中的 `#[no_mangle] extern "C"` 函数签名或 C API 注释后，应同步执行：

```powershell
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

如果 `-Verify` 失败，说明 [include/fjiffyldg.h](../include/fjiffyldg.h) 与 Rust FFI 源码或 [cbindgen.toml](../cbindgen.toml) 不一致，需要重新生成并提交。
