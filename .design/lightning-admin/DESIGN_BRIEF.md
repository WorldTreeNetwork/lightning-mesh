# Design Brief: Lightning Admin home

## Problem

The person who runs a house mesh opens Lightning Admin and gets one window doing four jobs at once: a scan toolbar, a Wi-Fi rename field, the whole guild panel, and a table of every machine on the link. They cannot tell, at a glance, which house they are looking at, how many of its routers answered, or which part of the app they are in.

## Solution

The window is a compact operator console for the house they have selected. A header names that house and switches saved groups. Under the header, a left bar lists the sections, each with an icon, and collapses to icons only. Home shows how many of the group's routers were heard on the last scan (`3/5`). Expanding that line lays the heard routers out as named buttons, each with an icon. Group holds the guild name, the Wi-Fi, and the other local-network settings. Map, Nearby, and Services are their own sections. You holds personal settings, including how to become the admin.

## Experience Principles

1. Dense until it is a different thing -- Related controls sit close together. A margin appears when the header, the left bar, and the page are separate objects, not between every row.
2. Recognizable at a glance -- Every section has an icon. A router button has an icon and its name. The count is active over total. The guild name and the Wi-Fi name stay different words.
3. The bar is how you move -- The left bar reaches every section from any page. A page that is not a section (a router, Become the admin) still has a way back.

## Aesthetic Direction

- **Philosophy**: Functional admin console, in the manner of Ubiquiti's Network application. Plain's warm neutrals and steel-blue accent stay. Color is status and selection. Chrome is a header, a left bar, and a tight page.
- **Tone**: Direct and operational. A tool you leave open.
- **Reference points**: Ubiquiti's controller: full-width header, icon section bar underneath it, dense lists, little decoration. Router names you can scan. Addresses stay readable on Nearby.
- **Anti-references**: Generous marketing margins. A Rams product-shot with space around every control. Cyberpunk as the only look. A single scrolling page that stacks the guild editor, the neighbor table, and the Wi-Fi field.

## Existing Patterns

This pass extends the Admin window. It does not replace Plain's tokens or the discovery and guild behavior underneath.

- Typography: Plain uses `--font: system-ui, -apple-system, 'Segoe UI', sans-serif` and `--mono` for addresses. Body is 15px. Cyberpunk uses IBM Plex Mono / JetBrains Mono and stays an easter egg.
- Colors: Semantic tokens in `admin/src/app.css`. Plain and Cyberpunk, each with System, Light, and Dark. Default is Plain + System. Appearance stays a menu in the header.
- Spacing: `--space-0` through `--space-12` in `admin/src/app.css`. Use `--space-1` and `--space-2` inside a region. Use `--space-4` and up only between the header, the bar, and the page. The window is often a half-width niri column about 847px wide.
- Components: shadcn-svelte Badge. Appearance uses bits-ui DropdownMenu. New controls that shadcn-svelte already offers should come from there; otherwise Tailwind on the existing tokens. Icons come from Lucide (named in `admin/components.json`, not yet a dependency). Role colors already exist for admin, member, absent, and no-seed.

Routes today are `/` (the stacked window) and `/become` (claim admin). SvelteKit with `ssr = false`.

## Component Inventory

| Component | Status | Notes |
| --------- | ------ | ----- |
| App shell | New | Header across the top. Left section bar directly under it. Page fills the rest. |
| Section bar | New | Home, Group, Map, Nearby, Services, and You. Icon and label. Collapses to icons. Current section is marked. |
| Home | New | Status line and router buttons inside the shell. |
| Group switcher | New | In the header. Each saved group shows `active/total` from the last scan. |
| Status line | New | `3/5` for the selected group. Expands in place. |
| Router button row | New | Heard routers only, left to right. Icon plus router name. |
| Map | New | Network map of the selected group. Stub allowed. |
| Router | New | One router, opened from a button. Stub allowed. Not a section. |
| Group | New | Guild name, Wi-Fi name, other local-network settings, roster. Quiet rows for routers this scan missed. Absorbs today's network-name field. |
| Nearby | Modify | Today's neighbor table moves here. |
| Services | New | The service mesh. Stub allowed. |
| You | New | Personal settings. Holds Become the admin. |
| Become the admin | Modify | The flow on `/become` moves under You. |
| Appearance menu | Exists | Header. Plain / Cyberpunk, System / Light / Dark. |
| Guild panel | Modify | Membership and rename move onto the group page. The panel leaves the home. |

## Key Interactions

- Opening the app shows the shell on Home, with the last selected group in the header. The status line reads `3/5`: routers heard on the last scan, over routers assigned to that group.
- The left bar shows an icon and a label for Home, Group, Map, Nearby, Services, and You. The current one is marked. Collapsing the bar leaves the icons and the mark. The choice persists.
- Opening the switcher lists saved groups, each with its own `active/total`. Choosing one replaces the house, the count, and the router buttons.
- Home always shows this group's routers. Heard ones are present and open the router page. Routers this scan missed stay on the page, quiet, and read "Not heard". A group with no roster shows whoever the scan heard.
- Activating a router button opens that router's page. The section bar stays. A back control returns to Home.
- Activating a section opens that page inside the same shell. Group shows the guild name, the Wi-Fi name, the local-network settings, and the roster. Heard routers look present. Missed routers look quiet and read "Not heard". Editing the Wi-Fi name is the current "Network name" apply. Editing the guild name does not change the Wi-Fi name.
- Nearby lists every machine this computer can hear. A Lightning router is one of those machines. Services is the service mesh.
- You shows this person's settings. Become the admin contains the claim instructions that live on `/become` today. When this person is not yet an admin, the status line can name that as the next action and open You.

Counts and the heard-router names should come from the last real scan when the app has one. Saved groups with no roster yet may show a placeholder count. Map, Router, and Services may be stubs. The section bar must work even when the page body is a stub.

## Responsive Behavior

Lightning Admin is a desktop window, not a phone app. The narrow case is a tiled column, about 847px wide.

- At that width the section bar is collapsed to icons unless the person has expanded it. Labels return when the window is wide or when they expand it.
- The router button row stays one horizontal row and scrolls sideways when the names do not fit.
- The switcher is a menu, usable at that column width.
- Group, Nearby, and You scroll vertically inside the page. The header and the section bar stay put.
- Appearance, type size, and contrast follow the active theme. No phone navigation.

## Accessibility Requirements

- Text on surfaces meets WCAG AA: 4.5:1 for body text, 3:1 for large text. Use `--text`, `--muted`, and `--accent`.
- Every section, router button, switcher row, collapse control, and back control is a real button or link with a visible focus ring (`--focus`).
- A collapsed section still has an accessible name (the label), not only an icon.
- The status count is text, not only a graphic. The expanded row is a named group ("Routers heard").
- Guild name and Wi-Fi name are separate labeled fields.
- Quiet roster rows stay in the accessibility tree, marked as not heard on the last scan.
- `prefers-reduced-motion` keeps the bar collapse and the status expansion instant.

## Out of Scope

- Redesigning Cyberpunk. It remains selectable and will be updated later.
- A new color or type system. Plain's tokens stand. Density is how those spacing tokens are used.
- Wiring every stub to live data. Map, Router, and Services may be placeholder pages.
- The public user guide, the hello.mesh front desk, and fleet install.
- Changing what a guild membership means, or writing the Wi-Fi name from a guild rename.
- Radio bring-up, deploy, and mesh-errand behavior.
- A phone layout of this window.
