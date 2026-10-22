# Tasks

`mjolnir-mesh-bf7.2` owns this architecture change.

- [x] Independent architecture advise r1 send-back (`reviews/opus-arch-review.md`)
- [x] Answer S1–S5 in design.md and the spec delta (2026-10-22)
- [x] Fresh advise after preparation (`reviews/opus-arch-review-r2.md`, accept)
- [x] Candidate-bound plan builder on a fake adapter. Refuse submission to `OpenWrtAdapter` until live probes exist (`crates/mjolnir-apply/src/uplink.rs`: `build_uplink_plan`, `submit_uplink_plan` gated on `ProvesUpstream`, 2026-10-22)
- [x] Root-only secret file snapshotted outside the four UCI revisions (plan names `/etc/mjolnir/secrets/uplink/<candidate-id>` 0600 plus `mjolnir.uplink.secret_generation`; the sidecar snapshot/restore is part of the `ProvesUpstream` contract and modeled in the fake adapter — the on-target sidecar is live-path follow-up)
- [x] Tests for wrong-candidate refusal, wrong credentials restoring the prior revision without requiring upstream evidence, receipt and revision not confirming a guessed passphrase, mesh BSSID refused as upstream, mesh-mode radio refused, captive lease installing no mesh default route, and no second default route beside an existing gateway export

Not owed here:

- Live fleet association or SSH.
- Hardware receiver qualification (`mjolnir-mesh-lpv`).
- Paid transit.
- Resident Wi-Fi (`mjolnir-mesh-ai0.7`).
