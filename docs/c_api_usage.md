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

仓库内置的头文件 smoke 检查会先验证头文件由 `cbindgen` 生成且未过期，再分别用 C 和 C++ 编译器编译 smoke 输入：

```powershell
pwsh -File scripts/check_c_abi.ps1
```

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

| 函数 | 作用 |
| ---- | ---- |
| `LoadAndScanFile` | 加载文件并启动后台行扫描 |
| `LoadFileOnly` | 只加载文件，不启动行扫描 |
| `WaitFileScanTaskFinished` | 等待后台扫描结束 |
| `BackstageRequestStop` | 请求停止扫描并清空当前行索引 |
| `GetFileLineCount` | 获取总行数 |
| `GetFileLinePos` | 获取指定行起始字节偏移 |
| `GetFileLineLength` | 获取指定行内容长度，不含换行符 |
| `GetFileLineIndex` | 根据字节位置查找所在行 |

## 6. 读取数据

`ReadFileData` 的 `len` 是输入输出参数：调用前表示希望读取的最大字节数，返回后表示实际读取字节数。

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

`ReadFileDataLLineCut` 按 C++ 参考实现语义读取短行批量数据，并对超长行执行 4KB 截断。`ReadFileDataEndOfLine` 可从指定行内位置读取到当前行末尾。

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
- 指针有效期到 `ClearHugeBuffer(fm)`、下一次 `GetFileMappedHuge` 或 `fjiffyldg_clear(fm)`。
- 空文件、打开失败或映射失败会返回空指针，并将 `bufferSize` 置为 0。

## 8. 编码和文件工具函数

| 函数 | 作用 |
| ---- | ---- |
| `CheckTextASCII` | 检查字节片段是否全为 ASCII |
| `CheckWholeTextUtf8` | 检查完整字节片段是否为 UTF-8 |
| `CheckExtractTextUtf8` | 抽样检查文本片段是否为 UTF-8 |
| `GetUtf8TextCharCount` | 统计 UTF-8 字符数并推进输入指针 |
| `GetFileSizeByteCount` | 查询文件大小 |
| `ToCloneFile` | 复制文件 |
| `ToSaveFile` | 保存缓冲区到文件 |
| `ToAppendFile` | 追加缓冲区到文件 |
| `ToConcatenateFile` | 将第二个文件追加到第一个文件 |

## 9. 错误码约定

多数返回 `int` 的函数使用 0 表示成功，非 0 表示错误。常见错误码与 Rust 内部错误类型保持映射，例如：

| 错误码 | 含义 |
| ------ | ---- |
| `0` | 成功 |
| `-1` | 文件未加载或句柄无效 |
| `1` | 文件不可访问 |
| `2` | 流错误 |
| `3` | 内存映射错误 |

返回指针的函数通常用空指针表示失败，并通过输出参数返回 0 或保留错误上下文。

## 10. 维护生成文件

修改 [src/ffi.rs](../src/ffi.rs) 中的 `#[no_mangle] extern "C"` 函数签名或 C API 注释后，应同步执行：

```powershell
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

如果 `-Verify` 失败，说明 [include/fjiffyldg.h](../include/fjiffyldg.h) 与 Rust FFI 源码或 [cbindgen.toml](../cbindgen.toml) 不一致，需要重新生成并提交。
