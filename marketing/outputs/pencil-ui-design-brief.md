# RillCoin — Website UI Design Brief
### For Pencil / UI Design System

> "Wealth should flow like water."

This document is the single source of truth for designing the RillCoin website UI.
Apply every specification here exactly. Do not introduce colours, fonts, or effects
not listed below.

---

## 1. Design Direction

**Aesthetic:** Ultra-modern deep-tech. Dark, precise, minimal. Feels like a serious
protocol, not a meme coin. Inspired by cutting-edge infrastructure products
(Vercel, Linear, Stripe Terminal, Uniswap v4 interface). High contrast, glowing
accents, clean grid discipline.

**Emotional target:** Trustworthy. Principled. Inevitable.

**Visual metaphors:** Moving water, flowing streams, fluid velocity. Not explosions,
not rockets — clean, directional energy.

---

## 2. Colour System

### Background Stack (layered, dark-to-dark)

| Token               | Hex        | Usage                                        |
|---------------------|------------|----------------------------------------------|
| `bg-void`           | `#05080F`  | Page background — the deepest layer          |
| `bg-base`           | `#0A1628`  | Primary surface (cards, panels)              |
| `bg-raised`         | `#0F1E38`  | Elevated surface (modals, popovers)          |
| `bg-overlay`        | `#162440`  | Hover states, active panel backgrounds       |
| `bg-glass`          | `rgba(15, 30, 56, 0.6)` | Glassmorphism — backdrop-blur panels |

### Brand Blues (primary identity, carried from logo)

| Token               | Hex        | Usage                                        |
|---------------------|------------|----------------------------------------------|
| `blue-500`          | `#3B82F6`  | Primary interactive — buttons, links, focus  |
| `blue-400`          | `#60A5FA`  | Hover state for blue-500                     |
| `blue-600`          | `#2563EB`  | Pressed state, strong emphasis               |
| `blue-glow`         | `#3B82F6` at 25% opacity | Glow halos behind CTAs, icons   |

### Cyan (from logo right-bar — use sparingly as accent)

| Token               | Hex        | Usage                                        |
|---------------------|------------|----------------------------------------------|
| `cyan-400`          | `#22D3EE`  | Highlight text, live data indicators         |
| `cyan-500`          | `#06B6D4`  | Gradient partner to blue, data viz           |
| `cyan-glow`         | `#06B6D4` at 20% opacity | Subtle glow on stat numbers         |

### Accent (use sparingly — CTAs and critical emphasis only)

| Token               | Hex        | Usage                                        |
|---------------------|------------|----------------------------------------------|
| `orange-500`        | `#F97316`  | Primary CTA button, badge highlights         |
| `orange-400`        | `#FB923C`  | CTA hover                                    |
| `orange-glow`       | `#F97316` at 20% opacity | Glow halo on CTA button            |

### Text

| Token               | Hex        | Usage                                        |
|---------------------|------------|----------------------------------------------|
| `text-primary`      | `#F1F5F9`  | Headlines, primary body text                 |
| `text-secondary`    | `#94A3B8`  | Subheadings, captions, metadata              |
| `text-muted`        | `#475569`  | Disabled, placeholder, very quiet labels     |
| `text-inverse`      | `#0A1628`  | Text on light/orange surfaces                |

### Borders & Dividers

| Token               | Hex / Value                  | Usage                         |
|---------------------|------------------------------|-------------------------------|
| `border-subtle`     | `rgba(148, 163, 184, 0.08)`  | Card edges, dividers          |
| `border-default`    | `rgba(148, 163, 184, 0.15)`  | Input borders, table lines    |
| `border-strong`     | `rgba(59, 130, 246, 0.30)`   | Focus rings, selected states  |
| `border-glow`       | `rgba(59, 130, 246, 0.50)`   | Glowing card borders on hover |

### Semantic

| Token               | Hex        | Usage                              |
|---------------------|------------|------------------------------------|
| `status-success`    | `#10B981`  | Confirmed transactions, live nodes |
| `status-warning`    | `#F59E0B`  | Pending, caution                   |
| `status-error`      | `#EF4444`  | Failures, invalid input            |
| `status-info`       | `#3B82F6`  | Informational, neutral notice      |

---

## 3. Typography

### Font Families

| Role       | Family            | Source              |
|------------|-------------------|---------------------|
| Display    | `Instrument Serif`| Google Fonts        |
| UI / Body  | `Inter`           | Google Fonts        |
| Code / Data| `JetBrains Mono`  | Google Fonts        |

### Type Scale

| Level       | Family            | Weight   | Size  | Line Height | Letter Spacing | Usage                         |
|-------------|-------------------|----------|-------|-------------|----------------|-------------------------------|
| Display XL  | Instrument Serif  | 400      | 72px  | 1.05        | -0.02em        | Hero headline (desktop)       |
| Display L   | Instrument Serif  | 400      | 56px  | 1.1         | -0.02em        | Section headers               |
| H1          | Instrument Serif  | 400      | 48px  | 1.15        | -0.01em        | Page titles                   |
| H2          | Instrument Serif  | 400      | 36px  | 1.2         | -0.01em        | Sub-section titles            |
| H3          | Inter             | 600      | 24px  | 1.3         | 0              | Card headings, feature titles |
| H4          | Inter             | 600      | 18px  | 1.4         | 0              | Labels, group headings        |
| Body L      | Inter             | 400      | 18px  | 1.65        | 0              | Lead paragraph text           |
| Body M      | Inter             | 400      | 16px  | 1.6         | 0              | Standard body                 |
| Body S      | Inter             | 400      | 14px  | 1.55        | 0              | Captions, footnotes           |
| Label       | Inter             | 500      | 12px  | 1.4         | 0.06em         | All-caps UI labels, tags      |
| Code M      | JetBrains Mono    | 400      | 14px  | 1.6         | 0              | Inline code, addresses        |
| Code S      | JetBrains Mono    | 400      | 12px  | 1.5         | 0              | Hash values, small data       |

**Rule:** Instrument Serif is for editorial headlines only. Never use it for UI
labels, buttons, or navigation. Inter handles all functional text.

---

## 4. Spacing & Layout

**Base unit:** 4px. All spacing must be a multiple of 4.

### Spacing Scale

| Token  | Value  |
|--------|--------|
| `xs`   | 4px    |
| `sm`   | 8px    |
| `md`   | 16px   |
| `lg`   | 24px   |
| `xl`   | 32px   |
| `2xl`  | 48px   |
| `3xl`  | 64px   |
| `4xl`  | 96px   |
| `5xl`  | 128px  |
| `6xl`  | 192px  |

### Page Layout

| Property          | Value              |
|-------------------|--------------------|
| Max content width | 1200px             |
| Content padding   | 24px (mobile) / 80px (desktop) |
| Section padding   | 96px vertical (desktop) / 64px (mobile) |
| Grid columns      | 12-column, 24px gutter |
| Sidebar width     | 240px (if applicable) |

---

## 5. Border Radius

| Token    | Value   | Usage                                |
|----------|---------|--------------------------------------|
| `xs`     | 4px     | Tags, badges, small chips            |
| `sm`     | 6px     | Buttons, inputs                      |
| `md`     | 10px    | Cards, panels                        |
| `lg`     | 16px    | Feature cards, modals                |
| `xl`     | 24px    | Hero sections, large containers      |
| `full`   | 9999px  | Pills, avatars, lozenges             |

---

## 6. Elevation & Shadow

| Level    | Shadow Value                                                       | Usage                      |
|----------|--------------------------------------------------------------------|----------------------------|
| `sm`     | `0 1px 3px rgba(0,0,0,0.5)`                                       | Inputs, small elements     |
| `md`     | `0 4px 16px rgba(0,0,0,0.4)`                                      | Cards, dropdowns           |
| `lg`     | `0 8px 40px rgba(0,0,0,0.5)`                                      | Modals, popovers           |
| `glow-blue` | `0 0 32px rgba(59,130,246,0.25), 0 0 64px rgba(59,130,246,0.10)` | Active cards, CTA focus  |
| `glow-cyan` | `0 0 24px rgba(6,182,212,0.20)`                                  | Data highlights            |
| `glow-orange` | `0 0 32px rgba(249,115,22,0.30)`                               | Primary CTA button         |

---

## 7. Visual Effects

### Glassmorphism (glass panels)
```
background: rgba(15, 30, 56, 0.60)
backdrop-filter: blur(20px) saturate(1.5)
border: 1px solid rgba(148, 163, 184, 0.10)
border-radius: 16px
```

### Gradient — Brand (logo-grade, blue-to-cyan)
```
linear-gradient(135deg, #4A8AF4 0%, #5DE0F2 100%)
```
Use for: icon backgrounds, gradient text, decorative line accents.

### Gradient — Surface (page depth)
```
linear-gradient(180deg, #05080F 0%, #0A1628 50%, #060C1A 100%)
```
Use as the page background for hero sections.

### Gradient — CTA
```
linear-gradient(135deg, #F97316 0%, #FB923C 100%)
```
Use for primary CTA buttons.

### Grid Background Texture
Subtle dot grid or line grid overlaid on deep background.
```
background-image: radial-gradient(rgba(59,130,246,0.08) 1px, transparent 1px)
background-size: 24px 24px
```
Apply at 40% opacity on hero sections only.

### Glow Halo (behind key icons / logos)
```
width: 200px, height: 200px
background: radial-gradient(circle, rgba(59,130,246,0.20) 0%, transparent 70%)
```
Place behind the Rill coin icon on hero.

---

## 8. Component Specifications

### Primary Button (CTA)
```
background:    linear-gradient(135deg, #F97316, #FB923C)
text:          #0A1628  (text-inverse)
font:          Inter 600, 14px, letter-spacing 0.02em
padding:       12px 24px
border-radius: 6px
shadow:        glow-orange
hover:         brightness(1.05), scale(1.02)
active:        brightness(0.95)
```

### Secondary Button
```
background:    transparent
border:        1px solid rgba(59,130,246,0.40)
text:          #60A5FA  (blue-400)
font:          Inter 500, 14px
padding:       12px 24px
border-radius: 6px
hover:         border-color rgba(59,130,246,0.80), bg rgba(59,130,246,0.06)
```

### Ghost Button
```
background:    transparent
border:        1px solid rgba(148,163,184,0.15)
text:          #94A3B8
padding:       10px 20px
border-radius: 6px
hover:         border rgba(148,163,184,0.30), text #F1F5F9
```

### Input Field
```
background:    #0F1E38  (bg-raised)
border:        1px solid rgba(148,163,184,0.15)  (border-default)
text:          #F1F5F9
placeholder:   #475569  (text-muted)
font:          Inter 400, 16px
padding:       12px 16px
border-radius: 6px
focus-border:  rgba(59,130,246,0.60)  (border-strong)
focus-shadow:  0 0 0 3px rgba(59,130,246,0.12)
```

### Card — Standard
```
background:    #0A1628  (bg-base)
border:        1px solid rgba(148,163,184,0.08)  (border-subtle)
border-radius: 10px
padding:       24px
shadow:        md
hover:         border-color rgba(59,130,246,0.30), shadow glow-blue
```

### Card — Glass
```
background:    rgba(15,30,56,0.60)
backdrop-filter: blur(20px)
border:        1px solid rgba(148,163,184,0.10)
border-radius: 16px
padding:       32px
```
Use glass cards for hero feature callouts and stat blocks.

### Badge / Tag
```
background:    rgba(59,130,246,0.12)
border:        1px solid rgba(59,130,246,0.25)
text:          #60A5FA
font:          Inter 500, 11px, letter-spacing 0.08em, uppercase
padding:       4px 10px
border-radius: 4px
```

### Code Block
```
background:    #05080F  (bg-void)
border:        1px solid rgba(148,163,184,0.10)
border-radius: 10px
padding:       20px 24px
font:          JetBrains Mono 400, 13px
color:         #94A3B8
line-height:   1.7
```
Syntax highlight accent: cyan-400 `#22D3EE` for keywords, blue-400 for strings.

### Navigation Bar
```
background:    rgba(5,8,15,0.80)
backdrop-filter: blur(16px)
border-bottom: 1px solid rgba(148,163,184,0.08)
height:        64px
padding:       0 80px
position:      sticky, top: 0
z-index:       100
```

### Stat / Number Display
```
value:         Instrument Serif 400, 48px, text-primary
label:         Inter 500, 12px, letter-spacing 0.08em, uppercase, text-secondary
accent-line:   2px, gradient blue-to-cyan, width 32px, below value
```

### Divider
```
border:        none
height:        1px
background:    rgba(148,163,184,0.08)
```

---

## 9. Iconography

- Style: Outline icons, 1.5px stroke weight, rounded caps and joins.
- Recommended set: **Lucide Icons** (MIT licensed).
- Size: 16px (inline), 20px (standard UI), 24px (feature sections).
- Colour: Inherit from text context — use `text-secondary` for decorative,
  `blue-400` for interactive, `cyan-400` for live/data indicators.
- Never fill icons. Outline only.

---

## 10. Motion & Animation

| Property          | Value                              |
|-------------------|------------------------------------|
| Default duration  | 150ms                              |
| Emphasis duration | 250ms                              |
| Easing (UI)       | `cubic-bezier(0.16, 1, 0.3, 1)`   |
| Easing (fade)     | `ease-out`                         |
| Hover scale       | `1.02` on cards, `1.01` on buttons|
| Page transitions  | Fade + 8px upward translate, 200ms |

**Reduce-motion:** All animations must respect `prefers-reduced-motion: reduce`.

---

## 11. Logo Usage on Web

- Use `rill-icon.svg` (coin mark) or `rill-wordmark-dark.svg` (coin + RILL text).
- On dark backgrounds (all website backgrounds): white wordmark or icon on transparent.
- Nav: coin icon at 32px height, left of "RILL" wordmark in Inter 700.
- Hero: coin icon at 64–80px with glow halo behind it.
- Footer: icon at 24px, muted opacity (70%).
- Minimum clear space: equal to the icon's height on all sides.
- Never place logo on Flowing Blue `#3B82F6` or Accent Orange `#F97316`.

---

## 12. Page Structure — Website Sections

Build these sections in order. Each section snaps to the grid.

### 12.1 Navigation
- Logo left, nav links centre, CTA button right.
- Links: Inter 500, 14px, `text-secondary`, hover `text-primary`.
- Active link: `blue-400`.
- CTA: "Get Testnet RILL" — Secondary Button style.
- Sticky with glassmorphism background.

### 12.2 Hero
- Full-width, min-height 90vh.
- Background: Surface gradient + dot grid texture at 40% opacity.
- Glow halo centred behind coin icon.
- Layout: centred, single column.
- Badge above headline: "Testnet Live" — Badge component, cyan variant.
- Headline: Display XL, Instrument Serif — "Wealth should flow like water."
- Sub-headline: Body L, Inter 400, `text-secondary` — elevator pitch text.
- CTA row: Primary CTA + Secondary button, 16px gap.
- Below fold: animated scroll indicator (subtle downward chevron).

### 12.3 Concept / How It Works
- 3-column feature grid (glass cards).
- Each card: icon (24px, blue-400) + H3 heading + Body M text.
- Section title: H2, Instrument Serif, centred.
- Section label above title: Label style, "HOW IT WORKS", `text-secondary`.

### 12.4 Stats Bar
- Full-width strip, `bg-raised` background.
- 4 stats across: stat component (number + label).
- Numbers in brand gradient (blue-to-cyan) using gradient-text technique.
- Dividers between each stat: subtle vertical border.

### 12.5 Decay Mechanism Explainer
- Split layout: text left (50%), visual right (50%).
- Text: H2 + Body L paragraphs + Secondary Button "Read the whitepaper".
- Visual: animated curve diagram or decay visualiser embed.
- Background: slightly elevated panel, `bg-raised`.

### 12.6 Code / Technical Proof
- Dark full-width section, `bg-void` background.
- Code block showing decay calculation or CLI interaction.
- Short label + H3 above, Body S caption below.
- Optional: tabbed code examples (Rust / CLI).

### 12.7 Testnet CTA Strip
- Centred, high contrast section.
- Glow halo behind the section.
- H2 headline + Body M + Primary CTA button.
- Background: deep surface with blue glow radial at centre.

### 12.8 Footer
- 4-column grid: Logo + tagline / Product links / Community links / Legal links.
- Logo at 24px, muted.
- Inter 400, 14px, `text-muted` for links, hover `text-secondary`.
- Copyright line: Body S, `text-muted`.
- Divider above footer: 1px, `border-subtle`.

---

## 13. Responsive Breakpoints

| Breakpoint | Width    | Behaviour                             |
|------------|----------|---------------------------------------|
| Mobile     | < 640px  | Single column, 24px side padding      |
| Tablet     | 640–1024px | 2-column grid, 40px side padding   |
| Desktop    | > 1024px | 12-column grid, 80px side padding    |
| Wide       | > 1440px | Content capped at 1200px, centred    |

On mobile: nav collapses to hamburger menu. Hero becomes 100vw with 48px headlines.
3-column grids become single column. Stats bar becomes 2x2 grid.

---

## 14. Tone Reminders for UI Copy

- No banned words: moon, lambo, pump, dump, gem, ape, degen, HODL, wagmi.
- No price predictions or financial promises.
- Use water/flow metaphors: "circulation", "flow", "stream", "distribute".
- Use approved terms: "concentration decay" (not "burn"), "decay pool" (not "tax pool").
- Voice: confident, technical, principled.

---

*Document version: 1.0 — Feb 2026*
*Source tokens: `shared/design-tokens/tokens.json`*
*Logo source: `shared/brand-assets/rill-icon.svg`*
