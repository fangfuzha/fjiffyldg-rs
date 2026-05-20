# Security Policy

[中文文档](SECURITY.md)

## Supported Versions

The project is currently in the `0.x` phase. Security fixes are prioritized for the latest `main` branch and the latest crates.io release. Backports to older versions depend on impact and maintenance cost.

## Reporting a Vulnerability

If you find a security issue, please do not disclose it publicly first. Use GitHub private vulnerability reporting or a private maintainer contact listed in the repository metadata when available. Include as much of the following information as possible:

- Affected version or commit;
- Operating system and target platform;
- Reproduction steps or a minimal input;
- Potential impact, such as crash, out-of-bounds read, denial of service, or unsafe FFI usage;
- Whether you have tested the latest `main` branch.

## Response Expectations

Maintainers will try to confirm the impact as soon as possible and coordinate disclosure after a fix is available. Fixes may include Rust crate updates, C ABI documentation updates, regenerated headers, and GitHub Release notes.

## Security Boundary

Fjiffyldg primarily handles local file reading, line indexing, and C ABI calls. C API callers must follow the documented pointer, length, buffer lifetime, and read-only mmap constraints. Violating these constraints may cause undefined behavior or process crashes.
