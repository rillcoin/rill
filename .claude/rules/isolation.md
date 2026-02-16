# Project Isolation

This is the **Rill** project (RillCoin cryptocurrency). Strict isolation from other projects.

## Forbidden References

Never reference, import, or use identifiers from these projects:
- **Subtone/SubtoneFM**: No Supabase URLs, Cloudflare Workers, Wrangler config, R2 buckets
- **Renewly**: No Stripe, Resend, Foundry Labs, Vercel tokens

## Forbidden Commands

Do not run: `wrangler`, `vercel`, `stripe`, `supabase`, `aws`

## Environment

When `RILL_CONTEXT` is set, you are in the Rill workspace. Verify with `echo $RILL_CONTEXT`.
