# ADR proposal: overlay canvases filled from allowlisted stdout

Status: proposed under activated `mjolnir-mesh-qap4`; independent review outstanding.

## Inputs and limits

Papyrus D-043 (`openspec/specs/canvas/spec.md` in the Papyrus repo,
`src-tauri/src/canvas.rs`, `static/canvas.html`) is the visual and windowing
reference: undecorated, transparent, no system shadow, cyan corner ticks,
45° octagon, frost (`Effect::HudWindow` on macOS 15; `NSGlassEffectView`
when the class exists), Control-drag move, Option-drag resize. Lightning
Admin at HEAD is one decorated Tauri window (`admin/src-tauri/tauri.conf.json`)
with `scan_link_local`, `apply_network_name`, and `sign_guild_op`. Tauri
features do not include `macos-private-api`. Transparent overlays in Papyrus
need that feature.

This change names the contract. Chrome lands in `add-admin-canvas`. The
ifconfig pipe lands in `add-host-ifconfig-view`. Do not fold this spec as
current behavior until those land.

## Decisions

1. **Copy, do not extract.** Admin vendors a copy of the canvas chrome and
   the overlay window builder. Papyrus remains a sibling product. A shared
   crate is a later extraction if both trees hurt.

2. **Overlay windows, not CSS panes in main.** Each stdout view is a
   runtime `WebviewWindow` labeled `canvas-*`, capability glob `canvas-*`.
   The decorated main window stays. A crystal grid of many overlays is
   `add-crystal-grid`; this ADR does not pick Three.js, web3d-space, or
   CSS grid inside main.

3. **Allowlisted argv, never a shell.** A source is a name plus a fixed
   argument vector. The process is `Command::new(path).args(args)` with
   `stdin` null. No `/bin/sh -c`. No operator-typed command line. First
   row: name `ifconfig`, argv `["/sbin/ifconfig"]`, host macOS. Missing
   binary is an error in the well, not a fallback to `if-addrs`. Non-macOS
   shows unsupported for that source and does not spawn.

4. **Transcript, not a second parser.** The well is a monospace dump of
   combined stdout and stderr. Non-zero exit is a failure state with the
   status visible. `scan_link_local` remains the neighbor list. Parsing
   ifconfig into NIC cards is a later view, not this source.

5. **One-shot child with a lifetime.** A fill is one process. Timeout and
   capture-byte cap are finite; `add-host-ifconfig-view` names the numbers
   and the Rust/IPC types. Truncation is visible. One child per canvas;
   a new fill or window destroy cancels and reaps the previous child.
   States: loading, success (including empty), failure, truncated. No
   poll or refresh UI in this landing. Stale output from a reaped child
   does not appear later.

6. **Resize anchors.** Option/Alt at pointer-down starts resize (wins over
   Control). Live Option/Alt during that resize: held = from center;
   released = top-left fixed. Shift live = aspect ratio from pointer-down.
   Control at pointer-down without Option starts a move and does not
   convert later. Copy of Papyrus `canvas.html` live `altKey` behavior.

7. **Host only.** The first source runs on the Admin machine. Overlay SSH
   to a mesh node is a later source row and needs its own change.

8. **One canvas here.** Simultaneous canvases and a grid wait on
   `add-crystal-grid`. This contract folds after one canvas plus ifconfig.

## Unverified hypotheses

- `canvas-chrome-port` — Papyrus owner-drawn canvas (`macos-private-api`,
  transparent, HudWindow frost, CSS octagon ticks) copies into Lightning
  Admin's current Tauri 2 crate — confidence: high — verification: empirically-tested
  — spike-required: false — spike-artifact: docs/spikes/canvas-chrome-port/finding.md.
  Validated by spike canvas-chrome-port on 2026-10-30. Keep Admin's tauri 2.11.5
  family lock; add `macos-private-api` only (unpinned 2.12.x runtime does not compile).

## Security

Allowlist is compile-time or a checked-in table, not a preference file of
argv. Output is not interpreted as HTML. Large dumps truncate with a
visible cap rather than growing without bound. Refresh is operator-driven
or a bounded poll; it does not spawn a login shell.
