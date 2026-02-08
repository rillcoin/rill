# RillCoin Brand Guidelines

> "Wealth should flow like water."

---

## Concept: Parallel Streams

The Rill mark is two vertical bars — one blue, one cyan — standing parallel inside a dark coin circle. They represent two streams flowing side by side: wealth moving through the network. The right bar is taller and carries a subtle 2-degree tilt, giving the mark asymmetry and forward motion. The bars also read as the "ll" in Rill.

A thin luminous ring borders the coin, shifting from blue to violet — the surface tension of the stream.

---

## Logo Suite

### Raster masters (APIFrame/Midjourney)
| File | Description |
|---|---|
| `rill-icon-hires.png` | Upscaled coin icon — source of truth |
| `rill-favicon-hires.png` | Upscaled favicon — dark rounded square |
| `concepts/coin-3.png` | Original approved concept |

### Production SVGs
| File | Type | Use |
|---|---|---|
| `rill-icon.svg` | Coin icon | Dark circle on dark bg, 1024x1024 |
| `rill-icon-light.svg` | Coin icon | White circle variant, 1024x1024 |
| `rill-favicon.svg` | Favicon | Dark rounded rect, 32x32 |
| `rill-wordmark.svg` | Wordmark | Coin + RILL, light backgrounds |
| `rill-wordmark-dark.svg` | Wordmark | Coin + white RILL, dark backgrounds |

---

## Colors

| Token | Hex | Role |
|---|---|---|
| bg | `#0C1420` | Coin fill |
| outer | `#080E18` | Deep background |
| blue-light | `#4A8AF4` | Left bar top |
| blue-deep | `#1E4FBB` | Left bar bottom |
| cyan-light | `#5DE0F2` | Right bar top |
| cyan-deep | `#1EA8D4` | Right bar bottom |
| ring | `#4A88E8` | Coin border glow |

**Left bar**: `linear-gradient(180deg, #4A8AF4, #1E4FBB)`
**Right bar**: `linear-gradient(180deg, #5DE0F2, #1EA8D4)`

---

## Typography

Inter, weight 700, letter-spacing `0.06em`.

---

## Geometry

- Coin circle: center-aligned, radius ~66% of container
- Left bar: centered-left, full-radius corners (pill shape), height ~33% of coin diameter
- Right bar: centered-right, ~12% taller than left bar, 2-degree clockwise tilt
- Gap between bars: ~70% of bar width
- Border ring: 2.5px stroke, blue-to-violet gradient, 70% opacity

---

## Rules

1. Do not rotate, stretch, or add effects.
2. Do not alter the bar gradients, tilt, or height ratio.
3. Use dark wordmark on light backgrounds, white wordmark on dark.
4. Do not place on busy or photographic backgrounds.
5. Coin always sits left of text in the lockup.
6. Raster PNGs are the visual source of truth.

---

## Files

```
shared/brand-assets/
  rill-icon.svg
  rill-icon-light.svg
  rill-favicon.svg
  rill-wordmark.svg
  rill-wordmark-dark.svg
  rill-icon-hires.png
  rill-favicon-hires.png
  concepts/
    coin-3.png

shared/design-tokens/
  tokens.json
```
