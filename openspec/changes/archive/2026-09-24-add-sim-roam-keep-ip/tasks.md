# Tasks

- [x] Human activation — Duke `activate all` 2026-09-18
- [x] Advise by an independent non-Grok reader; accept before act — r1 **send-back** (`reviews/2026-09-18-advise.md`); r2 **accept** (`reviews/2026-09-18-advise-r2.md`, READER sol-arch-review)
- [x] S1 ordered pre-hop baseline + green conditions (assoc transition, IP, `/32` B-only, LWW lease, sequenced egress via B) — design.md + spec 2026-09-18
- [x] S2 property-red: no transition / A keeps `/32` / lease diverges / no resume via B → harness non-zero — spec scenarios 2026-09-18
- [x] S3 hop mechanism + evidence schema in the log (station/neigh, seq/loss, egress node) — design.md 2026-09-18
- [x] STA guest hops A→B; harness prints IP, proto 158, LeaseBook, egress gap (`deploy/sim/roam-keep-ip.sh`)
- [x] Harness does not close `5wc` / `wvg` / `sz9.1` (2026-09-24 run: RED A still has proto 158; IP kept; assoc A→B)

Not owed: iPhone DNA script (`sim.9`); call-survive.
