## ADDED Requirements

### Requirement: Backhaul connects with no guild and across guilds

The daemon SHALL bind Iroh to the legacy ULA on `br-mesh` when that address is present at prefix length 64 or 128 and is neither tentative nor DAD-failed. A missing guild file, a malformed guild file, and a guild id that is not the peer's guild SHALL NOT keep the endpoint closed and SHALL NOT change the bind address. A dial SHALL target `ula_addr` of the authenticated peer endpoint id and SHALL NOT read the local guild file to compute that address. The outer transport table SHALL contain the legacy `/64` on `br-mesh`. When relays are disabled, the dial SHALL proceed if that prefix or a Babel host route on `br-mesh` covers the peer, and SHALL wait otherwise. `10.254` SHALL NOT be assigned. The status lamp SHALL treat a missing legacy ULA while mesh L2 is up as overlay-sick, and SHALL NOT treat a missing `10.254` as overlay-sick.

#### Scenario: No guild still dials

- GIVEN `br-mesh` has this node's legacy ULA and `/etc/mjolnir/house-guild-id` is missing
- WHEN the daemon dials a peer endpoint id
- THEN the local socket is the legacy ULA and the destination is `ula_addr` of that endpoint id

#### Scenario: Another guild still dials

- GIVEN this node and the peer have different valid guild ids
- WHEN the daemon dials that peer
- THEN the destination is `ula_addr` of the peer endpoint id and is not `router_ula` of the local guild
