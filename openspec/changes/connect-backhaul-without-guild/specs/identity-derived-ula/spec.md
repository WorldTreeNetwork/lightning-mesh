## ADDED Requirements

### Requirement: Backhaul connects with no guild and across guilds

The daemon SHALL bind Iroh to the legacy ULA on `br-mesh` when that address is present at prefix length 64 or 128 and is neither tentative nor DAD-failed. A missing guild file, a malformed guild file, and a guild id that is not the peer's guild SHALL NOT keep the endpoint closed and SHALL NOT remove the legacy ULA from the bind set. When the guild file is valid the daemon SHALL also bind that node's house `/128` on the same port. A dial SHALL try `ula_addr` of the authenticated peer endpoint id, and SHALL also try `router_ula` of that peer for each guild id this node has learned, including a guild that is not the peer's and a guild learned before this node is a member. A missing guild file SHALL still dial `ula_addr`. An announced direct address SHALL be tried. The outer transport table SHALL contain the legacy `/64` on `br-mesh`. When relays are disabled, the dial SHALL proceed if that prefix or a Babel host route on `br-mesh` covers any attempted address, and SHALL wait otherwise. `10.254` SHALL NOT be assigned. The status lamp SHALL treat a missing legacy ULA while mesh L2 is up as overlay-sick, and SHALL NOT treat a missing `10.254` as overlay-sick. The one-shot commands `id`, `listen`, `connect`, `tun-listen`, and `tun-connect` SHALL bind that same legacy ULA when it is usable and SHALL NOT exit because a house `/128` is absent. When the house `/128` is usable those commands SHALL bind it as well. An explicit `--bind` SHALL replace the legacy ULA. A peer listening on only one of those addresses SHALL be able to complete the handshake. The daemon SHALL NOT require the peer to run the same build.

#### Scenario: One-shot commands welcome a guild-less node

- GIVEN `br-mesh` has this node's legacy ULA and `/etc/mjolnir/house-guild-id` is missing
- WHEN the operator runs `id`, `listen`, `connect`, `tun-listen`, or `tun-connect` without `--bind`
- THEN the process binds the legacy ULA and does not exit for lack of a house `/128`
- AND when that house `/128` is usable the same process also binds it

#### Scenario: No guild still dials

- GIVEN `br-mesh` has this node's legacy ULA and `/etc/mjolnir/house-guild-id` is missing
- WHEN the daemon dials a peer endpoint id
- THEN the local socket includes the legacy ULA and the attempt list includes `ula_addr` of that endpoint id

#### Scenario: Another guild still dials

- GIVEN this node has learned a guild id and the peer's listening address is the house `/128` for that guild
- WHEN the daemon dials that peer
- THEN the attempt list includes `ula_addr` of the peer endpoint id and `router_ula` of the learned guild and that peer, and the handshake can complete on the address the peer bound
