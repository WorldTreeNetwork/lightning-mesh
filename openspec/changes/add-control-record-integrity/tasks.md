# Tasks

`mjolnir-mesh-ai0.9` owns this instrument change.

- [x] Independent architecture advise. r1 **send-back** (`reviews/2026-10-22-advise.md`, READER opus-arch-review)
- [x] Answer B1–B4 and N1–N3 in design.md and the spec delta (2026-10-22)
- [ ] Fresh advise after preparation
- [ ] Add closed record schemas on the existing canonical-CBOR profile (`lightning-admin/record/<type>`), not a second encoder
- [ ] Consumer verifier: owner-signed checks, node-vouched rejection, freshness advanced only by owner-signed authority-state records, withdrawals accepted at any age, higher-epoch fork successor, unreadable freshness, and `VerifiedAuthority` constructable only from this verifier
- [ ] Tests that a non-ingesting consumer rejects a forged record, ignores re-gossip, and does not restore privilege when the newest record is replayed past the stale maximum

Not owed here:

- HTTPS alias resolution records (`add-https-aliases`).
- Owner-claim ceremony (`add-household-owner-claim`).
- Live network writes.
