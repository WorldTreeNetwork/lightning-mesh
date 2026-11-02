# remove-v4-backhaul-pin

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

Routers still bind and dial Iroh on `10.254`, and the living spec forbids `fd` in that set. `iroh-ula-dial` is go (`8857cb6`). The user named the replacement key: the peer's house router `/128`. That key is `router_ula` of the local guild and the authenticated peer endpoint id. `10.254` is not assigned, and `fe80::H` is not the connection key.

## What

- Iroh binds and dials the usable house router `/128`. The endpoint does not bind `0.0.0.0` and does not advertise `10.254.0.0/16`.
- `10.254` is not assigned. The birthday-collision heal for that prefix (pt9) does not run.
- The open Iroh connection is keyed by `router_ula` of the local guild and the authenticated peer endpoint id. A Hello or IHU on that connection associates Babel's overlay link-local `via`. The destination address is not the key. `fe80::H` is not reversed into `10.254`. IPv4 packets stay IPv4 and use that same neighbor.
- Babel on `br-mesh` learns the peer `/128` underlay host route before the first dial. Overlay mode runs that Babel beside Babel on `mjolnir0`. Outer Iroh to a peer `/128` uses the `br-mesh` route only. A `mjolnir0` route to the same destination does not carry it. Relays, when enabled, stay off `mjolnir0`. No static peer route.
- The endpoint waits for a usable house `/128`, calls `clear_ip_transports`, and does not fall back to `0.0.0.0` or every interface. Losing the address closes the endpoint.

## Impact

- Capabilities: MODIFIED identity-derived-ula
- ADRs: will amend ARCHITECTURE.md when the pin actually moves

## User journey & surfaces

No new UI because a router reboot comes up with the house `/128` bound for Iroh and no `10.254` address, and an existing peer still completes a dial. The cold-start check is an isolated fixture with relays disabled, no hand-installed peer route, and no existing Iroh connection: Babel on `br-mesh` installs the peer `/128`, the dial completes, then IPv4 and IPv6 forwarding. A later `mjolnir0` route to that `/128` does not carry outer Iroh, and removing the `br-mesh` route does not move it there. It is not a fleet errand. The operator-visible reference is `add-ula-node-reference`. The walked fleet column stays `10.254`.

## Out of scope

- Acting this node before `docs/spikes/iroh-ula-dial/finding.md` says go
- Keying a direct foreign-house Iroh peer by a foreign guild
- House host filters, admitted prefixes, Admin TUN, and rewriting the fleet inventory column
- A mesh errand or a fleet deploy from this scaffold

## Unverified hypotheses

- `iroh-ula-dial`. Claim: current Iroh binds and dials the house ULA without a duplicate-address failure. Confidence: high. Verification: empirically tested. Synthesis path: none. Spike artifact: `docs/spikes/iroh-ula-dial/finding.md` (`8857cb6`, verdict go). `bind_addr` on the `/128` still leaves the preset IPv4 socket on `0.0.0.0`.

## Preparation

- Upstream: `add-ula-host-forwarding` folded at `634f296`. Living spec still says `10.254` remains the Iroh bind and no candidate starts with `fd`. `overlay_link_local` derives `fe80::H` from the last 16 bits of a `10.254.a.b` address (`crates/mjolnir-mesh/src/tun/iface.rs`).
- Outcome: revised. Spike go at `8857cb6`. Key confirmed by the user: peer house router `/128`, computed as `router_ula(local guild, authenticated endpoint id)`. Round 2 send-back R4 answered: Babel on `br-mesh` is the underlay route source, including in overlay mode beside `mjolnir0`; outer dials use that route only; no static peer route; a missing `br-mesh` route fails closed when relays are off. Fresh advise stays open.
- Authority: 2026-11-01 "activate and run" for `mjolnir-mesh-bny6`, plus the user's "yeah use the /128". Bounds: no fleet-column rewrite, no mesh errand, no deploy. Architecture review before act.
