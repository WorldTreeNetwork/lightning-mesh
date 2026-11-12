---
name: admin-screenshot
description: >
  Capture the running Lightning Admin Tauri window on this niri desktop and
  look at the image. Use when verifying Admin UI, taking a screenshot of the
  desktop app, or when the user says screenshot, capture the window, or
  /admin-screenshot. The window app id is lightning-admin. A Chrome shot of
  http://localhost:1420 is the Vite page, not the Tauri window.
---

# Admin screenshot

Capture the real Tauri window with niri, then read the PNG. A saved path
is not a look.

## Find the window

```bash
niri msg -j windows
```

Match `app_id` `lightning-admin` and use that `id`. If several match, pick
the `title` for the view you need. If none match, Admin is not open. Start
it only when the user asked to run or verify the app:

```bash
cd admin && bun run tauri dev
```

Wait until `niri msg -j windows` lists `lightning-admin`. Do not kill
another process on port 1420. Papyrus uses that port.

## Capture

`--path` must be absolute. Passing `--id` does not focus the window.

```bash
niri msg action screenshot-window --id <ID> \
  --path /tmp/lightning-admin.png
```

When a design review asked for shots, save under
`.design/<feature>/screenshots/` with a name that says the view. Otherwise
use `/tmp/lightning-admin-<view>.png`.

Read the PNG and say what is on screen.

## Full screen

Use `grim /tmp/niri-screen.png` when the surrounding layout matters. Prefer
the window capture.

## Vite page

`http://localhost:1420` is the Svelte dev server. Screenshot it with Chrome
DevTools only for a DOM or layout check of that page. It is not the Tauri
window, and port 1420 may belong to Papyrus.
