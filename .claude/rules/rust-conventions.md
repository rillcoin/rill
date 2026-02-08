# Rust Conventions

- Edition 2024, stable toolchain, MSRV 1.85
- `cargo fmt` with default settings
- `cargo clippy -- -D warnings` must pass with zero warnings
- All consensus math uses checked arithmetic (`checked_add`, `checked_mul`, etc.)
- Public types: `#[derive(Debug, Clone, PartialEq, Eq)]` minimum
- Error types: use `thiserror` for library crates, `anyhow` only in binaries
- Logging: `tracing` crate with structured fields
- Dependencies: workspace-level in root `Cargo.toml`, inherited in crate `Cargo.toml`
