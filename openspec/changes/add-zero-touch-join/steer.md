# steer add-zero-touch-join

**When.** 2026-09-16
**Depth.** lean

## Decided

- Plug-and-wait grants: Find + quarantine (user)
  Why: association is not membership; iroh does not cover babel.
- Open control plane: only after 661 + fyby.3 + fyby.2 (user)
  Why: the two documented open-backhaul gates plus quarantine; 661 is
  neighbor/session/freshness, not origin-only.
- Stranger authorization: beacon offer + QR still works (user)
  Why: zero-touch offer on the quarantine lane; met remains physical
  presence; K-of-N and revocation stay.

## Skipped

None.

## Feeds change

Discoverable ≠ trusted. Radio is a hostile underlay. Inventory `m4a` is
the trusted-fleet bootstrap only. Do not implement RF-neighbor peer
ingest. Capability `identity-peering` is ADDED; ARCHITECTURE.md will
note the three gates.
