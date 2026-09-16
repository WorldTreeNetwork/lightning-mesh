# Work tracking

`mjolnir-mesh-ai0.5` owns this instrument change. Beads holds work and findings;
this is a pointer, not a duplicate task list. No checkboxes does not mean fold-ready.

- [x] Independent architecture advise (`reviews/2026-09-15-advise.md`, accept)
- [x] Pin language, on-disk layout, lock, v1 allowlist, bounds (`design.md` Implementation pins; `steer.md` 2026-09-15)
- [ ] Implementation-readiness advise of those pins (other-family reader; accept before code)
  - `reviews/2026-09-15-advise-pins.md`: **send-back** (fable-5.1-arch-review). Four pins owed before code: v1 allowlist = four UCI files only (meshd binary 10 MiB exceeds the 8 MiB snapshot cap; wpad swap is a package op with its rollback input outside the durable tree); boot-recovery executor mandatory + init ordering; deadline on boot_id + monotonic clock.
- [x] Re-pin A–D from that review in `design.md` Implementation pins (UCI-only v1; mandatory two-phase helper; boot_id+monotonic; derived `$STAGE/result`; inherited flock fd 9)
- [x] Re-run implementation-readiness advise after A–D re-pin
  - `reviews/2026-09-15-advise-pins-r2.md`: **accept** (fable-5.1-arch-review). A–D closed. Body notes for the adapter packet: helper must flock the inherited fd 9, not reopen the lock path; `steer.md:10` still says "optional" helper (stale log line, design.md pins mandatory); record busybox `flock` applet from a fleet node.
- [x] Rust crate `crates/mjolnir-apply`: journal/state-machine + red-first fault injection (laptop)
- [ ] OpenWrt adapter + shared flock + boot/service recovery-first (still this change; not live radio)
- [ ] Hardware qualification remains `mjolnir-mesh-lpv` / `mjolnir-mesh-z3th` (separate go-ahead)
