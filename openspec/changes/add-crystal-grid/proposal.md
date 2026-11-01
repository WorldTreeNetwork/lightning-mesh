# add-crystal-grid

> **PENDING**

**Rigor:** architecture

## Why

One stdout canvas can switch `ifconfig` and `netstat -ib`. The operator
cannot see two transcripts at once. Swap landed (`382c4ad`, folded
`eaa7d30`). This change is the grid of those same wells.

Preparation (2026-11-01): living spec
`openspec/specs/admin-stdout-views/spec.md` still caps the product at
one canvas until this change. Code still uses one window id,
`canvas-well`, in `admin/src-tauri/src/canvas.rs`. Outcome: still valid.
Authority: Duke said `change` on the pinned DAG. This node is a
human-gate. Banner stays PENDING. Engine choice waits on steer.

## What

- At least two stdout wells are visible at once.
- Each well is an owner-drawn canvas with its own allowlisted source.
  Focusing one does not close the others. A source swap stays inside
  that well.
- The layout is a grid of those wells. It is not a new data plane and
  not a new command allowlist.
- A 3d or diorama engine is not chosen here.

## Impact

- Capabilities: MODIFIED `admin-stdout-views` (lift the one-canvas cap).
  ADDED a grid requirement on that spec.
- ADRs: none until steer picks an engine.

## User journey & surfaces

The operator is in Lightning Admin on macOS, on the same canvas overlay
family as today's `canvas-well`.

- Working: they open a second well. Two transcripts are on screen. One
  shows `ifconfig`. They set the other to `netstat`. They focus either
  well. Both remain.
- Empty: a well whose source has not returned yet shows the same empty
  or in-progress state as a single canvas.
- Failed: a timed-out or refused fill marks that well. The other well
  keeps its transcript.
- Off: on a non-macOS host each well shows unsupported and does not
  spawn. A third source name is refused.

## Out of scope

- A new allowlisted command, a shell string, or remote SSH stdout.
- Parsing transcripts into NIC cards.
- Guild signing, the client SSID, or `scan_link_local`.
- Choosing or shipping a 3d / diorama engine before steer.
- Replacing the single-canvas swap behavior inside one well.
