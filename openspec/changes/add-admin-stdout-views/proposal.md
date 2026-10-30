# add-admin-stdout-views

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

Lightning Admin already scans Unique Local and link-local addresses through
`if-addrs` and `scan_link_local`. That is a structured neighbor list, not the
Mac operator's first look at the machine's own NICs. On macOS that look is
`ifconfig`. The product move is a Papyrus-style owner-drawn canvas (funky
octagon chrome) whose well is stdout from an allowlisted host command, with
room later for several such wells in a spatial grid.

## What

- Add capability `admin-stdout-views`: a canvas pane whose interior is stdout
  from a named allowlisted argv, not a shell string.
- Copy Papyrus D-043 overlay window chrome into Lightning Admin (cyan ticks,
  45° octagon, frost, Control-drag / Option-drag). No Papyrus crate dependency.
- Name host `ifconfig` (`/sbin/ifconfig` on macOS, no extra args) as the first
  source. The well shows the bytes.
- Keep the existing decorated main window for discovery, network name, and guild.
- One stdout-view canvas in this landing. Many wells and a crystal-grid /
  diorama layout wait on `add-crystal-grid`; this change does not pick a
  3D engine.

## Impact

- Capability: ADDED `admin-stdout-views` (contract; chrome and ifconfig land in
  child nodes `add-admin-canvas` / `add-host-ifconfig-view`).
- ADR: `design.md` records overlay-copy, allowlist, and raw-transcript decisions.
- Work node: `mjolnir-mesh-qap4`; parent intention `mjolnir-mesh-70wk`.
- Activation: Duke 2026-10-30 “activate and run” on `mjolnir-mesh-70wk`.
- Independent architecture advise required before child implementation consumes
  this contract. Do not fold until canvas + ifconfig have landed.
- ARCHITECTURE.md will note Admin canvas overlays when this folds.

## User journey & surfaces

An operator on a Mac runs Lightning Admin. The discovery window still lists
ULA/LL neighbors. One undecorated canvas appears with Papyrus chrome. Its
well shows the current `/sbin/ifconfig` transcript (combined stdout/stderr,
finite one-shot). Close or Escape destroys that canvas and reaps the
process. Later they swap the well's source or open more canvases into a
grid; those journeys are child changes.

## Out of scope

- Canvas window implementation: `add-admin-canvas` (`mjolnir-mesh-cz6v`).
- ifconfig pipe implementation: `add-host-ifconfig-view` (`mjolnir-mesh-bn7z`).
- Source swap UI: `add-view-swap` (`mjolnir-mesh-6sro`).
- Crystal grid / diorama / 3d layout: `add-crystal-grid` (`mjolnir-mesh-oqun`).
- Replacing `scan_link_local` / if-addrs discovery.
- Guild ops and client SSID apply.
- Arbitrary shell, user-typed argv, or remote node SSH stdout.
- Extracting a shared canvas crate from Papyrus.
- Windows `ipconfig` as a named source (later row on the same allowlist).
