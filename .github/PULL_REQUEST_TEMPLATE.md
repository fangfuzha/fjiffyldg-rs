## Summary

-

## Change type

- [ ] Bug fix
- [ ] Feature
- [ ] Performance improvement
- [ ] Documentation
- [ ] C ABI / C++ wrapper
- [ ] Release engineering

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-targets --all-features`
- [ ] `cargo doc --all-features --no-deps`
- [ ] `pwsh -File scripts/check_c_abi.ps1`
- [ ] `cargo publish --dry-run` when release metadata changed

## C ABI impact

- [ ] No C ABI changes
- [ ] Header regenerated with `pwsh -File scripts/generate_c_header.ps1`
- [ ] C API docs updated in both languages
- [ ] Breaking ABI impact documented

## Documentation

- [ ] Chinese and English docs updated together when applicable
- [ ] CHANGELOG updated when user-visible behavior changed
