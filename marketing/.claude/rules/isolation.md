# Marketing Isolation

This is the **Rill Marketing** workspace. Strict boundaries apply.

## Forbidden

- Never run `cargo`, `rustup`, `rustc`, or any Rust toolchain commands.
- Never modify files in the dev workspace (`../crates/`, `../src/`).
- Never reference Subtone, Renewly, or their identifiers.
- Never run `wrangler`, `vercel`, `polar`, `supabase`, `aws`.

## Environment

Verify isolation: `echo $RILL_CONTEXT` should return `marketing`.
