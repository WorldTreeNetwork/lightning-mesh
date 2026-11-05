# connect-backhaul-without-guild

> **ACTIVE BUILD**

**Rigor:** brief

Authority: the user said “activate connect-backhaul-without-guild and z7gl” on 2026-11-04. The banner was already ACTIVE BUILD. The grant covers this change and `mjolnir-mesh-z7gl`. It is not a run. Bounds from the same conversation: 802.11s stays as it is; a peer on another build is still dialed; the attempt list is the identity ULA plus every house `/128` whose guild id this node has learned; `10.254` stays unassigned; guild entry and the new-node passage are not this change.

## Why

A node with no house guild, or a guild that is not its neighbor's, never opened the Iroh endpoint. `binding_witness` waited for a usable house `/128`, and `HouseDialGate` rejected every dial when the guild file was missing. 802.11s and physical Babel were already up. The landed dial then used only `ula_addr`, so a peer still listening on its house `/128` stayed silent until it ran the same binary. That single address is not the compatibility rule. The endpoint tries every Lightning address it can form.

## What

- Iroh binds the legacy identity ULA (`ula_addr`) once it is present on `br-mesh` and not tentative or DAD-failed. When `/etc/mjolnir/house-guild-id` is valid it also binds that node's house `/128` on the same port, so a peer that still dials the house formula can complete.
- A dial tries `ula_addr` of the peer endpoint id, and `router_ula` of that peer for each guild id this node has learned. The local guild file is one source of a guild id. A missing file still dials `ula_addr`. A direct address the peer announced is tried too.
- The outer transport table gets the identity `/64` on `br-mesh`, so the identity dial is on-link. `10.254` is not assigned. No per-peer static host route.
- The status lamp treats a missing identity ULA as overlay-sick. A missing `10.254` does not.
- `mjolnir-mesh-z7gl`: tr3000, wr3000s-bench, and ap3000-outdoor-b answer tr3000-b. The install is `install-node.sh` of a binary that tries this address set, names only, no `--wireless`. tr3000-b's cable is `10.42.149.1`.

## Impact

- Capability: MODIFIED `identity-derived-ula` (edited in the living spec with this change)

## User journey & surfaces

No new UI because the router joins the backhaul by itself. The visible check is the status lamp and `mjolnir-meshd` listening on the identity ULA.

## Out of scope

- Assigning `10.254`
- Keying inner house forwarding by a foreign guild
- Signing a guild op, writing `/etc/mjolnir/house-guild-id`, or the new-node passage (`add-zero-touch-join`, `met`)
- Changing the 802.11s mesh id or the client SSID
