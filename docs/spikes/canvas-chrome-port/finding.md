---
spike_id: canvas-chrome-port
hypothesis_id: canvas-chrome-port
source_synthesis: openspec/changes/add-admin-stdout-views/design.md
run_at: 2026-10-30T12:00:00-07:00
time_spent_min: 25
verdict: go
version-pinned: tauri@2.11.5 (Admin Cargo.lock: tauri-runtime 2.11.3, tauri-runtime-wry 2.11.4, tauri-utils 2.9.3, wry 0.55.1, tao 0.35.3)
---

# Spike: Papyrus overlay chrome copies into Admin's Tauri 2.11.5 crate

## Claim Tested

Papyrus owner-drawn canvas (`macos-private-api`, transparent, HudWindow frost, CSS octagon ticks) copies into Lightning Admin's current Tauri 2 crate.

## Minimal Test

Isolated lib crate `docs/spikes/canvas-chrome-port/repro/` type-checks the Papyrus `open_with` builder chain (`decorations(false)`, `transparent(true)`, `shadow(false)`, `Effect::HudWindow` + `Acrylic`) against **the same crate versions Admin already locks**. Action: `cargo check` with `macos-private-api`, then the same sources without the feature. Assertion: go-path compiles; on macOS the negative path fails because `WebviewWindowBuilder::transparent` is cfg-gated. CSS ticks are HTML (`Papyrus/static/canvas.html`) and do not need a crate feature.

## Evidence

- `repro/src/lib.rs` — probe source (also `evidence.code`).
- `repro-command.sh` — go + negative compile.
- `evidence.log` — `cargo check` Finished; negative `error[E0599]: no method named transparent`.
- Registry quotes, tauri **2.11.5**:
  - `src/webview/webview_window.rs:1073-1081` — `transparent` is `#[cfg(any(not(target_os = "macos"), feature = "macos-private-api"))]`.
  - `src/lib.rs:31` — feature enables transparent windows; config flag is `app.macOSPrivateApi`.
  - `src/window/mod.rs:2422-2465` — `EffectsBuilder` + `Effect` (re-export of `WindowEffect`, includes `HudWindow`).
- Papyrus production: `src-tauri/src/canvas.rs:246-280` uses this chain; `tauri.conf.json` `macOSPrivateApi: true`; `capabilities/canvas.json` windows `canvas-*`.
- Admin HEAD: `tauri = { version = "2", features = [] }`, lock `tauri 2.11.5`, one decorated window, capabilities `["main"]` only.

Unpinned `tauri = "=2.11.5"` (no family pins) resolved `tauri-runtime` 2.12.x / `wry` 0.57 and **failed to compile tauri itself**. Production must keep Admin's lock and only add the feature.

## Verdict: go

The overlay APIs exist on Admin's pinned Tauri 2.11.5 family. `macos-private-api` is required on macOS for `.transparent(true)`. HudWindow/Acrylic `EffectsBuilder` compiles on that crate. Chrome CSS copies as static HTML. `add-admin-canvas` can enable the feature, set `app.macOSPrivateApi`, add a `canvas-*` capability, and copy Papyrus window builder + `canvas.html` chrome. Do not bump the tauri family past the Admin lock in that landing.

VERDICT: go
