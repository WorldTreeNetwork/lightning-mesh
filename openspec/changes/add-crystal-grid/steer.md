# steer add-crystal-grid

**When.** 2026-11-01
**Depth.** explicit

## Decided
- crystal-grid-engine spike: skipped (auto)
  Why: the menu was declined. The recommended landing does not use a 3d renderer, so the spike does not gate the first grid.
- Layout: several owner-drawn overlays in a grid (auto)
  Why: same chrome as the current canvas, no new renderer. Two wells need two window ids.
- Placement memory: session only (auto)
  Why: positions last for this run of Admin. No layout file and no schema.
- [AUTO] Each well keeps its own allowlisted source and its own child. A swap stays inside that well.

## Skipped
- (none)

## Feeds change
Update the draft so the first landing is several owner-drawn windows, same chrome as `canvas-well`, placed in a grid. Do not add a 3d engine and do not persist positions. Mark the crystal-grid-engine hypothesis skipped. The banner stays pending until a human activates it.
