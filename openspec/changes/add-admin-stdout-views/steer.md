# steer add-admin-stdout-views

**When.** 2026-10-30
**Depth.** lean (`--go` from “activate and run”)

## Decided
- Spike: spike before act (auto)
  Why: Papyrus chrome is proven in Papyrus; Admin's Tauri crate has no
  `macos-private-api` yet. Do not skip the line. `act` on chrome waits for
  `docs/spikes/canvas-chrome-port/finding.md` go or an explicit skip.
- Window host: copy Papyrus overlay canvases into Lightning Admin (auto)
  Why: Duke asked for that funky-border window. Extra Tauri windows,
  `decorations: false`, transparent, D-043 chrome. Not CSS panes in the
  main window. Not a Papyrus crate extract.
- Stdout: raw `/sbin/ifconfig` bytes in an allowlisted well (auto)
  Why: the Mac utility is the view. No shell string. Discovery stays
  `scan_link_local`. Windows `ipconfig` and remote stdout wait.
- Grid: later node `add-crystal-grid` (auto)
  Why: first landing is one pane; spatial set is a second architecture
  landing once swap exists.

## Skipped
None remaining on this node. Child architecture (`add-crystal-grid`) still
needs its own steer when unblocked.

## Feeds change
Lightning Admin grows overlay canvases copied from Papyrus `static/canvas.html`
and `canvas.rs`, not a shared crate. A well is stdout of an allowlisted argv
vector. First vector is macOS `/sbin/ifconfig` with no arguments. Main window
stays the discovery / network-name / guild surface.
