# link-local-mgmt

This node's unicast IPv6 link-local on `br-lan` and `br-mesh` is first-contact
management. Overlay `10.254` remains mesh-wide SSH. Folded from
`add-link-local-mgmt` (2026-09-11).

Code: `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs` (`DirectoryNode`,
`stamp_node_link_locals`, `parse_if_inet6_link_local`),
`crates/mjolnir-hello` (`GET /api/node`, `GET /api/directory`).

## Requirements

### Requirement: Node publishes link-local on client and backhaul bridges

`mjolnir-meshd` SHALL include this node's unicast IPv6 link-local
addresses (`fe80::/10`) for `br-lan` and `br-mesh` in the
`directory.json` `node` object. Fields SHALL be omitted when the
interface has no such address. The daemon SHALL NOT publish the
`mjolnir0` overlay TUN link-local (`overlay_link_local`) as a
management address.

#### Scenario: hello names this box's LAN link-local

- GIVEN the node has a unicast `fe80` on `br-lan`
- WHEN a client on that L2 `GET`s `/api/node` (or `/api/directory`)
- THEN the `node` object includes that address, and does not require
  overlay `10.254`, a WAN lease, or a DHCP lease on the client

#### Scenario: Missing iface omits the field

- GIVEN `br-mesh` is down or has no link-local
- WHEN the directory is projected
- THEN the `br-mesh` link-local field is absent, not `null`

#### Scenario: Overlay TUN LL is not the management address

- GIVEN `mjolnir0` has a derived `fe80::<host16>` for babel
- WHEN the directory `node` object is written
- THEN that address is not advertised as `br-lan` or `br-mesh`
  link-local

### Requirement: Link-local is first-contact SSH

An operator on-link with `br-lan` or `br-mesh` SHALL be able to SSH to
this node at its link-local (`root@fe80::…%iface`) with an authorized
key, without a client IPv4 lease or overlay reachability. Overlay
`10.254` SHALL remain the mesh-wide management address once the
operator is on the mesh.

#### Scenario: Recovery SSH with no IPv4 lease

- GIVEN the operator is associated to the client SSID and has this
  node's `br-lan` link-local and zone
- WHEN they `ssh -o BatchMode=yes root@<ll>%<iface>`
- THEN dropbear accepts the authorized key even if DHCP has not
  handed out `10.42.x`

#### Scenario: Off-link still uses overlay

- GIVEN the operator is not on the same L2 as `br-lan` / `br-mesh`
- WHEN they need SSH
- THEN they use overlay `10.254` (or WAN-LAN admin), not link-local
