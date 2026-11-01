# Design: add-crystal-grid

Human-gate. Architecture. Steer has not picked the layout engine.

## Pins

- Each well is the existing stdout canvas: owner-drawn, allowlisted
  argv, 5 second timeout, 65536 byte cap, own in-flight child.
- The data plane does not change. Sources stay `ifconfig` and
  `netstat`.
- Today's code has one Tauri window id, `canvas-well`. Two visible
  wells need two ids. That is a constraint of the current overlay, not
  an engine decision.

## Layout fork (steer)

1. **Several overlays (recommended for the first landing).** Each well
   is its own owner-drawn window, same chrome as `canvas-well`, placed
   in a grid. No new renderer.
2. **One window, many wells.** A single overlay whose interior is a
   grid of transcripts. One set of window chrome.
3. **3d / diorama.** A scene of the same wells. No engine is in the
   tree. This option waits on steer and on the spike below.

The spec requires the operator-visible outcome (two wells, focus one,
independent sources). It does not name option 3.

## Unverified hypotheses

- `crystal-grid-engine` — a 3d or diorama layout of stdout wells can
  reuse the Papyrus overlay chrome without a new renderer — unverified
  (medium). Verification: a spike that places two `canvas-*` windows
  and, separately, one scene containing two transcripts. Synthesis:
  `docs/spikes/crystal-grid-engine/finding.md`. Do not start it from
  this draft.
