# Fjiffyldg (Rust)

[中文文档](README.md)

A high-performance, cross-platform file processing library designed for efficient modern large-file loading.

> This is a Rust rewrite of the original C++ [Fjiffyldg project](https://github.com/ceepuka/fjiffyldg).

## Core Features

- Cross-platform: Windows / Linux / macOS / Android
- Huge files: theoretical limit below 8 EB, with practical support for very large inputs
- Smart line index: direct positioning for million-line files
- Memory mapping: efficient large-file access with window remapping
- Encoding support: UTF-8 / UTF-16 (LE/BE) / UTF-32 (LE/BE)
- C/C++ ABI: public header, C function declarations, handle alias, export macro, and lightweight C++ RAII wrapper
- Benchmarks: Criterion entry for large-file load, scan, query, and read paths
- Memory safety: Rust ownership and borrowing with low runtime overhead

## Installation

Add the crate to `Cargo.toml`:

```toml
[dependencies]
fjiffyldg = "0.1"
```

## Usage

```rust
use fjiffyldg::Fjiffyldg;

fn main() -> fjiffyldg::Result<()> {
    let fjiffyldg = Fjiffyldg::new();

    fjiffyldg.load_and_scan("large_file.txt")?;

    println!("file size: {} bytes", fjiffyldg.file_size());
    println!("line count: {}", fjiffyldg.line_count());

    if let Some(data) = fjiffyldg.read(0, 1024) {
        println!("first 1024 bytes: {:?}", data);
    }

    println!("line 5 starts at: {}", fjiffyldg.line_pos(5));
    println!("line 10 length: {}", fjiffyldg.line_length(10));
    println!("position 100000 belongs to line: {}", fjiffyldg.line_at_pos(100000));

    Ok(())
}
```

## C/C++ ABI

The public C/C++ header is [include/fjiffyldg.h](include/fjiffyldg.h). It is generated from the Rust FFI source with `cbindgen` and provides C declarations, the `fjiffyldg_ptr` handle alias, export macros, and a lightweight C++ RAII wrapper. See [docs/c_api_usage_en.md](docs/c_api_usage_en.md) for the full usage guide.

```powershell
cargo install cbindgen --locked
pwsh -File scripts/generate_c_header.ps1
pwsh -File scripts/check_c_abi.ps1
```

## Benchmarks

```bash
cargo bench --bench large_file
```

The benchmark creates an approximately 12 MB temporary file and covers mmap load/scan, random line queries, and random reads.

## License

This project uses multiple licenses:

- BSD-3-Clause, aligned with the original C++ project
- MIT, common in the Rust ecosystem
- Apache-2.0, optional patent-friendly licensing

## Project Layout

```text
fjiffyldg-rs/
├── src/                # Rust API, FFI, encoding, indexing, and file operations
├── include/            # cbindgen-generated C/C++ ABI header
├── examples/           # Example programs
├── benches/            # Criterion benchmarks
├── scripts/            # Release and validation scripts
├── tests/              # Integration and C/C++ ABI smoke inputs
├── docs/               # Coverage reports and project documentation
├── Cargo.toml
├── README.md           # Chinese README
└── README_EN.md        # English README
```

## Compared with the C++ Version

| Feature | C++ version | Rust version |
| ------- | ----------- | ------------ |
| License | BSD 3-Clause | BSD-3-Clause OR MIT OR Apache-2.0 |
| Platform dependencies | U++ framework | Rust standard library + lightweight crates |
| Memory management | Manual + RAII | Ownership + borrow checker |
| Cross-platform | Yes | Yes |
| Ecosystem | C++ | crates.io |

## Contributing

Issues and pull requests are welcome.

## Contact

- Project: https://github.com/fangfuzha/fjiffyldg-rs
- Author: fangfuzha
