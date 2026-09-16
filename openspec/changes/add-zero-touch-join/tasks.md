# Tasks

Authoritative owed work is epic `mjolnir-mesh-fyby` and children.
These boxes mirror what this change owes once activated.

- [x] Human activation (banner to ACTIVE BUILD) — Duke said "activate all" 2026-09-16
- [ ] Advise by an independent non-Grok reader (Fable 5.1); accept before act
- [ ] `m4a`: `install-node.sh` / `update-fleet.sh` render full peer sets from `fleet-nodes.conf` onto the target; never from `iw station dump`
- [ ] `e5u`: signed capability beacon (protocol generation, node id, channel plan, bootstrap); nonce-bound iroh handshake; replay of a captured beacon does not authorize
- [ ] `fyby.2`: unknown id after association has no babel adjacency, no production CRDT writes, no subnet claim merge, no overlay SSH
- [ ] `661`: babel neighbor/session authenticated with freshness; forged default, replayed update, and forged withdrawal fail closed (property-red)
- [ ] `fyby.3`: unauthorized identities cannot merge subnet claims or production CRDT lane writes
- [ ] `met`: enrollment offer on the quarantine lane; existing QR path still enrolls; K-of-N / revocation unchanged
- [ ] Amend `ARCHITECTURE.md`: discoverable ≠ trusted; three gates; radio as hostile underlay
- [ ] Docs: `docs/join/node/03-join-the-mesh.md` trust paragraph matches this change

Not owed here (bullets, not boxes):

- `9u5y` local-EVM transit
- `3kd` / `190` client-L2 islands
- `rp9` user IdentiKey
- Fold (after act lands)
- Flipping live fleet `MESH_KEY`

## Owed from advise 2026-09-16 (fable-5.1-arch-review, send-back)

- [x] S1 `fyby.3` spec: subject-signed canonical records; merge verifies signature and grant; enrolled-forge scenario; coordinate-lane stamper carve-out (pinned 2026-09-16)
- [x] S2 `661` spec: per-identity neighbor keys (no fleet HMAC); prefixes bound to authorized claims; default needs gateway grant; enrolled-unowned-prefix scenario (pinned 2026-09-16)
- [x] S3 rewrite: live empty `MESH_KEY` is a fact, not owed; join doc SHALL say the gate is absent until the three gates land (pinned 2026-09-16)
- [x] N3 design.md: `mesh_fwding=1` HWMP by quarantined stations named as accepted DoS-only (pinned 2026-09-16)
