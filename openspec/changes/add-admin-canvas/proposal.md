# add-admin-canvas

> **ACTIVE BUILD**

**Rigor:** change

## Why

The accepted `admin-stdout-views` contract names a Papyrus-style overlay
canvas in Lightning Admin. Admin still has one decorated Tauri window and
empty `tauri` features. This change copies D-043 chrome into Admin so a
stdout well has a window to live in.

## What

- Enable `macos-private-api` and runtime `canvas-*` windows (transparent,
  no decorations, no system shadow, capability glob `canvas-*`).
- Copy Papyrus overlay chrome: cyan ticks, 45° octagon, frost, close,
  Escape, Control-drag move, Option-drag resize as pinned in
  `add-admin-stdout-views`.
- Keep the decorated main window. One canvas this landing.
- Do not fill the well from ifconfig here (`add-host-ifconfig-view`).

## Impact

- Capability: MODIFIED `admin-stdout-views` (implements overlay chrome;
  parent contract still unfolder).
- Parent: `add-admin-stdout-views` / `mjolnir-mesh-qap4` (closed, astra accept).
- Work node: `mjolnir-mesh-cz6v`. Grant: `mjolnir-mesh-70wk` activate-and-run.
- Spike `canvas-chrome-port` is open: act waits for
  `docs/spikes/canvas-chrome-port/finding.md` go or an explicit skip.

## User journey & surfaces

Operator runs Lightning Admin. A canvas overlay can be opened with Papyrus
chrome. The well may be empty. Discovery / network name / guild stay on
the main window. Close or Escape destroys the canvas.

## Out of scope

- ifconfig / stdout fill: `add-host-ifconfig-view` (`mjolnir-mesh-bn7z`).
- Source swap: `add-view-swap`.
- Multiple canvases / grid: `add-crystal-grid`.
- Papyrus crate extract.
