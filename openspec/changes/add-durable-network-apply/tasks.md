# Work tracking

`mjolnir-mesh-ai0.5` owns this instrument change. Beads holds work and findings;
this is a pointer, not a duplicate task list. No checkboxes does not mean fold-ready.

- [x] Independent architecture advise (`reviews/2026-09-15-advise.md`, accept)
- [x] Pin language, on-disk layout, lock, v1 allowlist, bounds (`design.md` Implementation pins; `steer.md` 2026-09-15)
- [ ] Implementation-readiness advise of those pins (other-family reader; accept before code)
- [ ] Rust crate `crates/mjolnir-apply`: journal/state-machine + red-first fault injection (laptop)
- [ ] OpenWrt adapter + shared flock + boot/service recovery-first (still this change; not live radio)
- [ ] Hardware qualification remains `mjolnir-mesh-lpv` / `mjolnir-mesh-z3th` (separate go-ahead)

