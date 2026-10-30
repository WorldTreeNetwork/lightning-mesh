# add-view-swap

> **ACTIVE BUILD**

**Rigor:** change

## Why

The stdout canvas opens on macOS `ifconfig` and stays there. The operator
needs a second allowlisted source in that same well. `add-host-ifconfig-view`
landed the pipe (`225c3f7`, folded `6d1dc9a`). This change names the second
source and the switch.

Preparation (2026-10-30): upstream ifconfig fill is in
`openspec/specs/admin-stdout-views/spec.md` (5s / 65536, argv
`["/sbin/ifconfig"]`). Code is `admin/src-tauri/src/stdout.rs` `lookup` and
`admin/static/canvas.html` `fillDefault`. Outcome: still valid. Authority:
campaign grant on `mjolnir-mesh-70wk` (“activate and run”). Second source
named here: `netstat` → `["/usr/sbin/netstat", "-ib"]`.

## What

- Add allowlisted source `netstat` with argv `["/usr/sbin/netstat", "-ib"]`.
  No shell. Same timeout, cap, cancel, and reap as `ifconfig`.
- On the one canvas (`admin/static/canvas.html`), show each allowlisted name.
  Choosing one cancels the in-flight child and fills that source.
- Default open stays `ifconfig`.
- Capability: MODIFIED `admin-stdout-views` (one canvas may switch sources).
  ADDED source-swap requirement.

## Impact

- Capabilities: MODIFIED `admin-stdout-views`; ADDED requirement on that spec.
- ADRs: none. `ARCHITECTURE.md` already points at `add-view-swap`.

## User journey & surfaces

The operator is on the Lightning Admin canvas overlay (`canvas-well`,
`admin/static/canvas.html`), not the discovery window.

- Working: the canvas opens on `ifconfig`. They activate `netstat`. The
  status and transcript change to that command's combined stdout/stderr.
  Activating `ifconfig` again restores that transcript. One window.
- Empty: a source that exits 0 with no bytes shows success and an empty
  transcript, same as today.
- Failed: timeout, missing binary, or non-zero exit shows failure. The
  previous transcript does not remain as if it were the new source.
- Off / unsupported: a name not on the allowlist is refused and does not
  spawn. On a non-macOS host both sources show unsupported and do not spawn.

## Out of scope

- A second canvas, grid, diorama, or 3d layout (`add-crystal-grid`,
  `mjolnir-mesh-oqun`, human-gate).
- Windows `ipconfig`, remote SSH stdout, a shell string, or a free-text command.
- Parsing either transcript into NIC cards.
- A refresh control or polling.
- Replacing `scan_link_local`.
