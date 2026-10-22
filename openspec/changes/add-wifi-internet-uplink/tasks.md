# Tasks

`mjolnir-mesh-bf7.2` owns this architecture change.

- [x] Independent architecture advise r1 send-back (`reviews/opus-arch-review.md`)
- [x] Answer S1–S5 in design.md and the spec delta (2026-10-22)
- [x] Fresh advise after preparation (`reviews/opus-arch-review-r2.md`, accept)
- [ ] Candidate-bound plan builder on a fake adapter. Refuse submission to `OpenWrtAdapter` until live probes exist
- [ ] Root-only secret file snapshotted outside the four UCI revisions
- [ ] Tests for wrong-candidate refusal, wrong credentials restoring the prior revision without requiring upstream evidence, receipt and revision not confirming a guessed passphrase, mesh BSSID refused as upstream, mesh-mode radio refused, captive lease installing no mesh default route, and no second default route beside an existing gateway export

Not owed here:

- Live fleet association or SSH.
- Hardware receiver qualification (`mjolnir-mesh-lpv`).
- Paid transit.
- Resident Wi-Fi (`mjolnir-mesh-ai0.7`).
