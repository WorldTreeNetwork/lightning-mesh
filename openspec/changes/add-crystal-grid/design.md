# Design: add-crystal-grid

Architecture. Steer 2026-11-01 picked several overlays. Spike
`crystal-grid-engine` is skipped.

## Pins

- Each well is the existing stdout canvas: owner-drawn, allowlisted
  argv, 5 second timeout, 65536 byte cap, own in-flight child.
- The data plane does not change. Sources stay `ifconfig` and
  `netstat`.
- Today's code has one Tauri window id, `canvas-well`. Two visible
  wells need two ids. That is a constraint of the current overlay, not
  an engine decision.

## Send-back 2026-11-01

The well id is the Tauri window label. Fill and cancel read that label.
The IPC payload is only the source name. Opening a well mints a new
`canvas-*` label. The maximum is four. A fifth open is a visible
refusal. A new well starts one cell step to the right of the newest
well, wraps down at the screen edge, and is clamped on screen.
Control-drag and Option-resize stay per well and are not snapped back.

## Layout

Steer picked several owner-drawn windows, same chrome as
`canvas-well`, placed in a grid. Positions last for the Admin session
only. The 3d spike `crystal-grid-engine` is skipped.
