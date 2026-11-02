# steer remove-v4-backhaul-pin

**When.** 2026-11-01
**Depth.** standard

## Decided

- Assign `10.254`: stop (user, 2026-11-01). The address is not installed. pt9 does not run.
- Demux key: the peer's house router `/128` (user, 2026-11-01). The old `fe80::H` ← `10.254` derivation is not the key.

## Skipped

None.

## Feeds change

Do not assign `10.254`. Key the open Iroh connection by `router_ula` of the local guild and the authenticated peer endpoint id. Associate Babel's link-local `via` from a Hello or IHU on that connection. Do not derive that key from `fe80::H`, `backhaul_addr`, or babeld `router-id`. IPv4 packets stay IPv4 and use the same neighbor. Iroh binds the usable house `/128`, calls `clear_ip_transports`, and does not bind `0.0.0.0`. Babel on `br-mesh` supplies the underlay host route used for the outer dial. A `mjolnir0` route to the same peer does not carry that dial. Architecture review before any bind edit.
