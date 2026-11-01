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

## Layout

Steer picked several owner-drawn windows, same chrome as
`canvas-well`, placed in a grid. Positions last for the Admin session
only. The 3d spike `crystal-grid-engine` is skipped.
