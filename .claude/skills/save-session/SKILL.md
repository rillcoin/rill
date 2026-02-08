---
name: save-session
description: >
  End-of-session persistence routine. Use when finishing a work session
  to record progress, update changelogs, and ensure continuity for the
  next session.
---

# Save Session Procedure

1. Run `cargo check --workspace` to verify the build is clean.
2. Run `cargo test --workspace` and note any failures.
3. Update the changelog in `docs/CHANGELOG.md` with what was accomplished.
4. If any cross-agent blockers were discovered, note them in `docs/BLOCKERS.md`.
5. Stage and commit changes with a descriptive message.
6. Summarize: what was done, what's next, any blockers.
