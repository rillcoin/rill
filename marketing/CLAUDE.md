# Rill Marketing

Go-to-market for RillCoin. All marketing agents operate from this workspace.

## Brand Essence

RillCoin: progressive concentration decay cryptocurrency. "Wealth should flow like water." Rill = a small stream. Visuals should feel fluid, principled, technical, clean.

## Design System

- **Colors:** Dark Navy `#0A1628`, Deep Water `#1A3A5C`, Flowing Blue `#3B82F6`, Accent Orange `#F97316`
- **Fonts:** Instrument Serif (headlines), Inter (body)
- **Tokens:** `shared/design-tokens/tokens.json`

## Agent Architecture

Specialized subagents in `.claude/agents/`. Brand Architect and Content Strategist run on Opus for foundational creative work. Implementation agents run on Sonnet. Boot order: Brand Architect → Graphic Designer → Content Strategist → Web Lead → Social Media → Community Lead.

## Shared Resources

- `shared/brand-assets/` — Logos, icons, approved imagery
- `shared/design-tokens/` — JSON, CSS, and Tailwind token files
- `shared/copy-library/` — Approved messaging, taglines, terminology
