# Tasks

- [x] Add `house_ula_prefix`, `router_ula`, and `app_ula` next to the legacy helpers in `tun/link.rs`, using the byte layout in `design.md`
- [x] Test the fixture vectors in the spec delta, including the legacy address for `node-example`
- [x] Read `/etc/mjolnir/house-guild-id` at startup and on legacy-ULA reconcile; trim whitespace; accept either hex case; treat anything else as absent
- [x] Record the last usable house `/128` in `/var/lib/mjolnir/house-ula` and delete only that address across reconcile and restart
- [x] Treat an already-present usable `/128` as assigned even if add returns an error; do not publish or record a tentative, DAD-failed, or missing address
- [x] Publish `house_ula` only from that usable kernel address; restore it after `br-mesh` recreation
- [x] Cover parser cases, failed add, already-present `/128`, tentative and DAD-failed, interface loss, and restart with a changed or removed guild file with state tests plus one assignment integration check
- [x] Keep the legacy `/64`, the `fc00::/7` Babel deny, and the `10.254` Iroh pin unchanged
- [x] Fresh advise after preparation

## Owed from advise (2026-10-31, astra-arch-review)

- [x] R1: Pin canonical router-id/app-key hash bytes and subnet/IID byte order in the design and spec delta; add literal house/router/app vectors and a legacy-output regression vector. See `reviews/2026-10-31-advise.md`.
- [x] R2: Pin guild-file parsing and reload boundaries, restart/change/removal behavior, stale-address ownership, interface-recreation recovery, and the assignment evidence required to publish `house_ula`; add acceptance cases for missing/malformed file, failed add, existing `/128`, interface loss/recovery, and guild change/removal. Startup-only loading is acceptable; routing and fleet deployment remain out of scope.
- [x] R3: Replace absolute guild-prefix and node-address uniqueness promises in the modified requirement with deterministic derivation and explicit collision limits; make separation scenarios fixture-specific without adding collision-allocation machinery.

## Owed from re-advise (2026-10-31, astra-arch-review)

- [x] R2a: Define ownership evidence that survives daemon restart, or an explicit bounded reset procedure, for an old house `/128` when the guild file changes or disappears while the daemon is stopped; preserve unrelated IPv6 addresses. Add the restart case to the delta. See `reviews/2026-10-31-advise-r2.md`.
- [x] R2b: Explicitly define publication for tentative/DAD-failed addresses and for an already-present correct `/128` when an add reports an error; align design and delta with the chosen assignment-state semantics.
- [x] R2c: Add lifecycle acceptance cases and implementation verification tasks for parser behavior, failed add, existing correct `/128`, interface loss/recovery, and restart/change/removal, including R2b's address states; use focused state/publication checks and an assignment integration check.
