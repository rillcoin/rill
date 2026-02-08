---
name: brand-identity
description: >
  RillCoin brand identity specifications. Load when creating visual assets,
  enforcing brand consistency, or referencing design tokens. Contains color
  values, typography rules, spacing system, and usage guidelines.
---

# RillCoin Brand Identity

## Color Palette

| Name          | Hex       | Usage                                    |
|---------------|-----------|------------------------------------------|
| Dark Navy     | `#0A1628` | Backgrounds, deep surfaces               |
| Deep Water    | `#1A3A5C` | Secondary surfaces, cards, overlays       |
| Flowing Blue  | `#3B82F6` | Primary actions, links, interactive       |
| Accent Orange | `#F97316` | Highlights, CTAs, emphasis, alerts        |
| White         | `#FFFFFF` | Body text on dark backgrounds             |
| Light Gray    | `#94A3B8` | Secondary text, captions, metadata        |

## Typography

| Role        | Family           | Weight     | Size Range   |
|-------------|------------------|------------|--------------|
| H1          | Instrument Serif | Regular    | 48-64px      |
| H2          | Instrument Serif | Regular    | 36-48px      |
| H3          | Inter            | SemiBold   | 24-30px      |
| Body        | Inter            | Regular    | 16-18px      |
| Caption     | Inter            | Regular    | 12-14px      |
| Code        | JetBrains Mono   | Regular    | 14-16px      |

## Spacing System

Base unit: 4px. Scale: 4, 8, 12, 16, 24, 32, 48, 64, 96, 128.

## Logo Usage

- Minimum clear space: 1x logo height on all sides
- Minimum size: 32px height for icon mark, 120px width for wordmark
- Always use SVG source files, never upscale rasters
- Approved backgrounds: Dark Navy, Deep Water, transparent
- Never place on Flowing Blue or Accent Orange backgrounds

## Design Token Files

- `shared/design-tokens/tokens.json` — Canonical source
- `shared/design-tokens/tokens.css` — CSS custom properties
- `shared/design-tokens/tailwind.config.js` — Tailwind theme extension
