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

## Why not the operator's first instinct

iroh authenticates the gossip/tunnel ALPN. babel hellos, 802.11s
payloads, and dropbear on the mesh/LAN zone do not ride that ALPN.
Sol (2026-09-15, caution): origin-only babel validation still leaves
replay, forged withdrawal, metric games, and exhaustion.

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
