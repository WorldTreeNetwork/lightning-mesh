# connect-backhaul-without-guild

> **ACTIVE BUILD**

**Rigor:** brief

## Why

A node with no house guild, or a guild that is not its neighbor's, never opened the Iroh endpoint. `binding_witness` waited for a usable house `/128`, and `HouseDialGate` rejected every dial when the guild file was missing. 802.11s and physical Babel were already up. The user requires the backhaul to connect in both cases.

## What

- Iroh binds the legacy identity ULA (`ula_addr`) once it is present on `br-mesh` and not tentative or DAD-failed.
- A dial targets `ula_addr` of the peer endpoint id. The local guild file is not an input.
- The outer transport table gets the identity `/64` on `br-mesh`, so the dial is on-link. `10.254` is not assigned. No per-peer static host route.
- The status lamp treats a missing identity ULA as overlay-sick. A missing `10.254` does not.

## Impact

- Capability: MODIFIED `identity-derived-ula` (edited in the living spec with this change)

## User journey & surfaces

No new UI because the router joins the backhaul by itself. The visible check is the status lamp and `mjolnir-meshd` listening on the identity ULA.

## Out of scope

- Assigning `10.254`
- Keying inner house forwarding by a foreign guild
- Rolling the other bench radios (`mjolnir-mesh-z7gl`)
