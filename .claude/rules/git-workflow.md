# Git Workflow

- Branch naming: `<agent>/<description>` (e.g., `core/implement-transaction-type`)
- Commit messages: `<crate>: <description>` (e.g., `rill-core: implement Transaction struct`)
- Always run `cargo test --workspace` before committing
- Pre-commit hook blocks commits containing Subtone/Renewly identifiers
- Never force-push to main
