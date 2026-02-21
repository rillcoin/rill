# Contributing to Rill

Thank you for your interest in contributing to RillCoin! This document provides guidelines to make the contribution process smooth for everyone.

## Getting Started

1. Fork the repository and clone your fork
2. Install Rust 1.85+ (`rustup update stable`)
3. Build the workspace: `cargo build --workspace`
4. Run tests: `cargo test --workspace`

## Development Workflow

### Branch Naming

```
<area>/<description>
```

Examples: `core/fix-utxo-lookup`, `decay/optimize-sigmoid`, `wallet/add-export`

### Commit Messages

```
<crate>: <description>
```

Examples: `rill-core: fix checked arithmetic overflow in fee calc`, `rill-decay: add proptest for cluster boundaries`

### Before Submitting a PR

Every PR must pass these checks:

```bash
# All tests pass
cargo test --workspace

# No clippy warnings
cargo clippy --workspace -- -D warnings

# Formatted correctly
cargo fmt --check
```

## Code Standards

- **Rust Edition**: 2024, MSRV 1.85
- **Arithmetic**: All consensus math uses `checked_add`, `checked_mul`, etc. No panicking arithmetic in consensus paths.
- **No floats in consensus**: All consensus calculations use `u64` fixed-point with 10^8 precision.
- **Error handling**: `thiserror` for library crates, `anyhow` only in binaries.
- **Logging**: Use `tracing` with structured fields.
- **Public APIs**: Must have doc comments.
- **Derives**: Public types need at minimum `#[derive(Debug, Clone, PartialEq, Eq)]`.

## What to Contribute

### Good First Issues

Look for issues labeled `good first issue` — these are scoped, well-defined tasks suitable for newcomers.

### Areas Where Help is Welcome

- **Testing**: Property-based tests (proptest), fuzzing, edge cases
- **Documentation**: Improving doc comments, adding examples
- **Performance**: Benchmarks, optimizations (with before/after numbers)
- **Tooling**: Developer experience improvements

### Consensus-Critical Code

Changes to `rill-core`, `rill-decay`, or `rill-consensus` require extra scrutiny:

- Must include comprehensive tests (unit + property-based)
- Must preserve all existing invariants
- Will receive thorough code review
- Should include rationale in the PR description

## Reporting Bugs

Please open an issue with:

1. What you expected to happen
2. What actually happened
3. Steps to reproduce
4. Rust version (`rustc --version`) and OS

For security vulnerabilities, see [SECURITY.md](SECURITY.md) instead.

## Pull Request Process

1. Create a focused PR that addresses one concern
2. Fill out the PR template
3. Ensure CI passes
4. Respond to review feedback
5. A maintainer will merge once approved

## Community

- [Discord](https://discord.com/invite/F3dRVaP8) — join #dev-discussion for technical chat
- [GitHub Issues](https://github.com/rillcoin/rill/issues) — bug reports and feature requests

## License

By contributing, you agree that your contributions will be licensed under the same dual license as the project: MIT OR Apache-2.0.
