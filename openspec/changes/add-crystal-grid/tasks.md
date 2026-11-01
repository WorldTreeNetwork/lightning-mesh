# Tasks

- [x] Human activation (banner to ACTIVE BUILD) — Duke said "activate and run" 2026-11-01
- [x] Steer: several owner-drawn overlays, session-only placement, 3d spike skipped (`steer.md`)
- [x] Fresh advise after preparation
  (send-back 2026-11-01, `reviews/2026-11-01-advise.md`; accept r2 `reviews/2026-11-01-advise-r2.md`; spec `openspec/changes/add-crystal-grid/specs/admin-stdout-views/spec.md`)
- [x] Owed R1: per-well child isolation scenarios; well id is the window label, never an IPC argument
- [x] Owed R2: a second well mints a new `canvas-*` label; maximum is four, and the fifth is a visible refusal
- [x] Owed R3: new well starts one cell step to the right, wraps down, clamped on screen; Control-drag and Option-resize stay per well
- [x] Owed R4: one well's timeout or refusal leaves the other transcript
- [x] Owed R5: "One canvas in this landing" renamed to "Canvas count"
- [x] Implement several owner-drawn windows, with at least two wells visible
- [x] Each well has its own window id, source, and in-flight child
- [x] Focusing one well does not close the others
- [ ] Focused verify on macOS: two wells, `ifconfig` and `netstat -ib`

Implementation gate: `CARGO_TARGET_DIR=/tmp/lm-admin-target cargo test --manifest-path admin/src-tauri/Cargo.toml --lib canvas` — 10 passed, 0 failed (2026-11-01). Covers independent ids, per-well fill/refusal/timeout/close, child reaping, fifth-open refusal, concurrent opens, and initial placement. Window visibility and gestures still need the visual check above.

Do not add a source. Do not start a 3d spike. Do not persist positions.
