# Design: add-zero-touch-join

Cross-cutting trust boundary. Security-sensitive. Steer 2026-09-16.

## Pins

- **Discoverable ≠ trusted.** Plug-and-wait means find + quarantine.
  Running Lightning Mesh firmware is not membership.
- **Three gates** before the control plane may ride open 802.11s:
  1. quarantine (`fyby.2`)
  2. authenticated babel neighbor/session/freshness (`661`)
  3. identity-authorized CRDT writes (`fyby.3`)
- **`m4a` is inventory-only.** `install-node.sh` / `update-fleet.sh`
  render full peer sets from `fleet-nodes.conf`. 802.11s neighbors are
  never appended to `list peer`.
- **Beacon is a locator.** Signatures on a broadcast IE are replayable.
  Freshness is the nonce-bound iroh handshake after association.
- **Enrollment.** e5u untrusted-compatible → offer on the quarantine
  lane; `met` QR remains operator/physical-presence. K-of-N and
  revocation are not skipped for UX.
- **Degrade, don't hard-reject.** Incompatible/untrusted → isolated
  L3/NAT. That path must not expose `10.254` SSH or unrestricted relay.
- **Residual RF risk** (DoS, airtime theft, traffic analysis, parser
  surface) is a named product choice, not "solved by iroh".
  **HWMP:** `mesh_fwding=1` makes an associated station an L2 forwarder
  for the segment. A quarantined stranger can relay or black-hole
  frames. That is accepted **DoS-only** exposure: iroh stays E2E and
  babel is authenticated per identity after `661`.
- **CRDT authz is subject signatures**, not a hop allow-list. iroh
  authenticates the delivering neighbor; gossip is epidemic. Merge
  verifies Ed25519 by the record subject plus lane grant (`leased_name`
  already does this). Coordinate-lane stamper signs and holds stamp grant.
- **Babel keys are per node identity**, not one fleet HMAC (RFC 8967
  shared key would recreate SAE insider-complete). Prefixes bind to
  the announcer's authorized claims; default needs a gateway grant.
- **Grant is per accepting node** (`membership-enrollment.md` local
  K / allow / block), not a mesh-wide instant.
- **Releases and tombstones** are production mutations: subject-signed
  over canonical bytes that include the lane key and HLC, same as
  records. An enrolled member cannot drop another identity's claim.
- **Babel prefix bind is router-id origin** (transit-safe). Jump nodes
  re-announce. Enrolled-insider forgery of another member's router-id
  is accepted residual until R5 revocation; this change does not owe
  cryptographic route-origin proof.
- **Migration.** Signed records land as new gossip enum variants; old
  variants freeze. Persisted claim/addr books MUST load across the
  schema change (not empty-on-unknown, which would renumber `pt9`
  backhaul claims).
- **Quarantine handshake addressing.** The joiner uses IPv6 link-local
  (or an ungated ephemeral), not a claimed `10.254`, so the enrollment
  lane does not collide with overlay derivation.

## Why not the operator's first instinct

iroh authenticates the gossip/tunnel **hop**, not the record **author**.
babel hellos, 802.11s payloads, and dropbear on the mesh/LAN zone do
not ride that ALPN. Sol (2026-09-15, caution): origin-only babel
validation still leaves replay, forged withdrawal, metric games, and
exhaustion. Fable (2026-09-16, send-back): hop allow-list and
fleet-wide babel HMAC are both insufficient.

## Slice order (act, after advise)

1. `m4a` — trusted inventory full peer sets (no new trust model).
2. `e5u` — beacon + handshake (find).
3. `fyby.2` — quarantine policy (what plug-and-wait actually grants).
4. `661` then `fyby.3` — the two open-backhaul gates.
5. `met` offer wiring onto the quarantine lane (QR already designed).

## Related docs (not living specs)

- `docs/network-coordination/identity-peering-requirements.md`
- `docs/network-coordination/island-formation.md`
- `docs/network-coordination/membership-enrollment.md`
