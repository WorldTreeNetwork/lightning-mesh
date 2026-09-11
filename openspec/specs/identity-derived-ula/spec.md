# identity-derived-ula

A node's Unique Local Address is a hash of identity, assigned `/64` on
`br-mesh` beside `10.254`. It is not an iroh QUIC candidate and is not
exported by babel. Folded from `add-identity-derived-ula` (2026-09-11).

Code: `crates/mjolnir-mesh/src/tun/link.rs` (`ula_addr`, `ULA_ID_INPUT`),
`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs` (`assign_ula_addr`,
`reconcile_backhaul_addr`, `DirectoryNode.ula`, `check_reachability`),
`crates/mjolnir-mesh/src/babel/config.rs` (`fc00::/7` deny),
`crates/mjolnir-hello` (`GET /api/node`).

## Requirements

### Requirement: Node ULA is a hash of identity

A node's Unique Local Address SHALL be a pure function of mesh
identity and node id in RFC 4193 ULA space (`fd00::/8`). It SHALL
be deterministic across reboots. Distinct node ids SHALL produce
distinct addresses. The address SHALL NOT be allocated by DHCP, RA,
or a CRDT claim.

#### Scenario: Same secret, same address

- GIVEN a node id and mesh id
- WHEN the daemon computes the ULA on two boots
- THEN both results are identical and in `fd00::/8`

#### Scenario: No clerk

- GIVEN two nodes that have never gossiped
- WHEN each assigns its ULA
- THEN neither consulted DHCP, a lease table, or a subnet claim

### Requirement: ULA sits beside 10.254, not in iroh candidates

The daemon SHALL assign the derived ULA on `br-mesh` next to the
derived `10.254` backhaul address, prefix length `/64`, so peer ULAs
are on-link. It SHALL NOT assign that ULA on `mjolnir0` or `br-lan`
in this capability. It SHALL NOT add that ULA to iroh's QUIC
candidate / bind set. Babel SHALL deny `fc00::/7` in and out until
a later change opts in. Stock OpenWrt `ula_prefix` SHALL stay
filtered.

#### Scenario: Overlay IPv4 still the underlay

- GIVEN the ULA is assigned
- WHEN iroh lists connection candidates (status or `/api/node`)
- THEN private IPv4 `10.254.x` is present and no candidate starts
  with `fd`

#### Scenario: On-link SSH

- GIVEN two nodes have assigned ULAs on `br-mesh`
- WHEN an operator on one node SSHes to the peer's ULA
- THEN the packet is on-link on `br-mesh` and does not require babel
  IPv6 or `mjolnir0`

#### Scenario: Directory publishes it

- GIVEN the ULA is assigned
- WHEN a client `GET`s `/api/node`
- THEN the node object includes the ULA, omitted when not yet
  assigned

#### Scenario: Not exported by babel; stock ula_prefix stays filtered

- GIVEN babeld config rendered for br-mesh and for `mjolnir0`
- WHEN the config is inspected
- THEN it contains `in ip fc00::/7 deny` and `out ip fc00::/7 deny`,
  and OpenWrt's random `ula_prefix` is not redistributed
