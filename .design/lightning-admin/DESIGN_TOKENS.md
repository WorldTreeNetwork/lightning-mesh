# Design tokens: Plain

Philosophy: Dieter Rams, adapted to a desktop window. Less, but better. One accent, and only for action, selection, and focus. Structure comes from hairlines and the spacing scale. Cyberpunk is an easter egg: its palette is unchanged except hover aliases, so Plain's accent cannot leak into it through inheritance.

Source of truth: the Plain blocks in `admin/src/app.css`. The names already in the app (`--bg`, `--surface`, `--text`, `--accent`, `--ok`, `--warn`, `--bad`, `--radius`, `--shadow`) still work. This pass fills the gaps and retunes Plain's color.

## Choices

- Warm neutrals. The old Plain was a cool gray with an electric blue (`#4c8dff` / `#1d5fd0`). The field is now warm paper in light and warm charcoal in dark. Dark is not an inversion of light.
- One steel-blue accent. Light `#1f4e8a` on paper, dark `#9dc0f0` on charcoal, each with its own ink color. `--accent-hover` and `--accent-active` are separate steps, not a glow.
- Status stays off the accent. Green, amber, and red are only for heard, warning, and failure.
- Type stays the system UI face. A loaded Helvetica would be more Rams and less Plain. Addresses keep the system mono.
- Body size is 15px. Controls are 14px. The count on the home uses `--font-size-xl` (22px). There is no display size in this interface.
- Radius stays 6px (`--radius-md`). Smaller and larger steps exist for later. Plain does not use pills.
- Shadow is for overlays only. `--shadow-sm` is a hairline. `--shadow-md` and `--shadow-lg` are the same quiet lift. The menu already uses `--shadow`.
- Motion is short and has no bounce. The status row expands in `--duration-normal` (180ms) with `--easing-default`. `prefers-reduced-motion` already cuts transitions.
- Spacing is a 4px desk scale, not the 1px and 3px fractional steps. The home should use these variables instead of new magic numbers.

## Color

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `--bg` | `#f3f2ef` | `#141311` | Window |
| `--surface` | `#fbfaf8` | `#1c1b19` | Header, menus, pages |
| `--surface-2` | `#eeece7` | `#252320` | Wells, inputs |
| `--border` | `#ddd9d2` | `#34312c` | Hairlines |
| `--border-strong` | `#c4bfb6` | `#4c483f` | Emphasis, menu edge |
| `--text` | `#1c1b19` | `#e7e4de` | Primary text |
| `--muted` | `#5f5b56` | `#b0aaa0` | Secondary text, "Not heard" |
| `--accent` | `#1f4e8a` | `#9dc0f0` | Action, selection, focus |
| `--accent-ink` | `#f7f6f3` | `#14202e` | Text on a filled accent |
| `--accent-soft` | `#e6eef6` | `#243044` | Selected row wash |
| `--accent-hover` | `#183e6e` | `#b7d0f5` | Hover |
| `--accent-active` | `#14355f` | `#d3e3fa` | Pressed |
| `--ok` | `#0e6b3c` | `#6fce96` | Heard |
| `--warn` | `#8a5a10` | `#e2b86a` | Warning |
| `--bad` | `#a61b14` | `#ee8d88` | Failure |
| `--focus` | `#1f4e8a` | `#9dc0f0` | Focus ring |

`--primary-bg`, `--primary-fg`, and `--primary-border` match the accent pair. `--row-hover` and `--local-bg` are the table washes.

## Spacing

`--space-0` 0, `--space-1` 2px, `--space-2` 4px, `--space-3` 8px, `--space-4` 12px, `--space-5` 16px, `--space-6` 24px, `--space-7` 32px, `--space-8` 48px, `--space-9` 64px, `--space-10` 96px, `--space-11` 128px, `--space-12` 160px.

Use the small steps. Related controls sit on `--space-1` or `--space-2`. `--space-4` and up are only the gap between things that are clearly separate: the header, the left bar, and the page.

## Type

| Token | Size | Use on the home |
| --- | --- | --- |
| `--font-size-xs` | 11px | Rare meta |
| `--font-size-sm` | 12px | Menu labels |
| `--font-size-base` | 15px | Body, links |
| `--font-size-md` | 16px | Router buttons |
| `--font-size-lg` | 18px | Group name |
| `--font-size-xl` | 22px | The `3/5` count |
| `--font-size-2xl` | 28px | Unused here |
| `--font-size-3xl` | 34px | Unused here |
| `--font-size-4xl` | 40px | Unused here |

Weights are 400, 500, 600, and 700. The count uses `--font-weight-medium` and `--line-height-tight`. Body uses `--line-height-normal` (1.45). Labels stay normal case (`--case: none`, `--track: 0`).

## Layout, radius, shadow, motion

- Reading measure `--max-width-content` is 72ch. `--max-width-wide` is 960px. `--max-width-page` is 1200px. A tiled column near 847px is the narrow case, and it is still this layout.
- Radius: 4, 6, 10, and full. Controls use 6.
- Durations: 50, 120, 180, 240, 320ms. Easing settles. No bounce.
- Breakpoint tokens exist for the scale. This window does not grow a phone navigation at `--breakpoint-sm`.

## What the home may color

The group name, the count, and the router names are `--text`. The slash and the "Not heard" state are `--muted`. A router button's border is `--border`. Its hover is `--row-hover`. The primary action, the current group, and focus use `--accent`. Heard is `--ok` only when a row needs a state word, not as a tint behind every button.
