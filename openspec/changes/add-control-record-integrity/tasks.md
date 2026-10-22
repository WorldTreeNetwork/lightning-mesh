# Tasks

`mjolnir-mesh-ai0.9` owns this instrument change.

- [ ] Independent architecture advise
- [ ] Extend the mesh-admin-capabilities encoder with a record-type domain tag if it cannot carry one, without a second canonicalizer
- [ ] Consumer verifier: owner-signed checks, node-vouched rejection, replay, withdrawn issuer, forked head, unreadable freshness
- [ ] Tests that a non-ingesting consumer rejects a forged or replayed record and accepts a valid owner-signed record

Not owed here:

- HTTPS alias resolution records (`add-https-aliases`).
- Owner-claim ceremony (`add-household-owner-claim`).
- Live network writes.
