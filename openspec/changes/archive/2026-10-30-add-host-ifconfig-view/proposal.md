# add-host-ifconfig-view

> **ACTIVE BUILD**

**Rigor:** change

## Why

The canvas overlay is on screen (`add-admin-canvas`, a4053ca) with an empty
well. The accepted stdout-views contract says opening that canvas on macOS
fills from host `/sbin/ifconfig` as raw combined stdout/stderr. This change
names the timeout, cap, and IPC and lands the pipe.

## What

- Allowlist one source: name `ifconfig`, argv `["/sbin/ifconfig"]`, macOS only.
- Spawn with `Command::new` (no shell). Combined stdout+stderr. stdin null.
- Timeout **5 seconds**. Capture cap **65536 bytes**. Truncation is visible.
- One child per canvas; new fill or window destroy cancels and reaps.
- States in the well: loading, success (including empty), failure, truncated,
  unsupported (non-macOS, no spawn).
- Opening the canvas (button or `LIGHTNING_ADMIN_OPEN_CANVAS`) fills ifconfig
  by default. No poll, no refresh control in this landing.
- `scan_link_local` stays the neighbor list.

## Impact

- Capability: MODIFIED `admin-stdout-views` (names timeout/cap; parent still
  unfolder until this lands).
- Parent: `add-admin-stdout-views` / `mjolnir-mesh-qap4`.
- Chrome: `add-admin-canvas` / `mjolnir-mesh-cz6v` (closed, a4053ca).
- Work node: `mjolnir-mesh-bn7z`. Grant: `mjolnir-mesh-70wk` activate-and-run.

## User journey & surfaces

Operator opens Lightning Admin on a Mac, clicks Canvas (or the env hook
opens it). The overlay well shows the current `/sbin/ifconfig` transcript in
monospace. Non-zero exit, missing binary, timeout, or spawn failure appear as
failure in the well. Discovery Scan on the main window is unchanged.

## Out of scope

- Source swap: `add-view-swap` (`mjolnir-mesh-6sro`).
- Multiple canvases / grid: `add-crystal-grid` (`mjolnir-mesh-oqun`).
- Windows `ipconfig` or overlay SSH stdout.
- Parsing ifconfig into NIC cards.
- Refresh / poll UI.
