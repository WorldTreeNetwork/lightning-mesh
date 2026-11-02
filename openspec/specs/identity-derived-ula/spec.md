# identity-derived-ula

A node's legacy Unique Local Address is a hash of the compiled mesh tag
and the node id, assigned `/64` on `br-mesh` beside `10.254`. When a
house guild id is configured, the daemon also assigns that router's
house `/128` (subnet 0 of a guild-derived `/48`) on `br-mesh`. An app
address uses subnet 1 of the same `/48`. Neither address is an iroh
QUIC candidate. When that router `/128` is usable on `br-mesh`, Babel
redistributes it and allows the house `/48` at prefix length 128, then
denies the rest of `fc00::/7`. On `mjolnir0`, a `/128` via overlay
`fe80::H` selects the already-open `10.254` peer. Folded from
`add-identity-derived-ula` (2026-09-11), `add-house-ula-prefix`
(2026-11-01), and `add-ula-host-forwarding` (2026-11-01).

Code: `crates/mjolnir-mesh/src/tun/link.rs` (`ula_addr`, `house_ula_prefix`,
`router_ula`, `app_ula`), `crates/mjolnir-mesh/src/tun/ula_host.rs`
(`parse_ula_host_routes`), `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`
(`assign_ula_addr`, `reconcile_house_ula`, `DirectoryNode.ula`,
`DirectoryNode.house_ula`), `crates/mjolnir-mesh/src/babel/config.rs`
(house `/128` allow, then `fc00::/7` deny), `crates/mjolnir-hello`
(`GET /api/node`).

## Requirements

### Requirement: Node ULA is a hash of identity

A node's legacy Unique Local Address SHALL remain a pure function of the compiled mesh tag `mjolnir/mesh/ula/v0` and the node id's UTF-8 bytes, in RFC 4193 ULA space (`fd00::/8`). It SHALL be deterministic across reboots. The address SHALL NOT be allocated by DHCP, RA, or a CRDT claim. The 64-bit interface id can collide; this capability does not detect or reallocate that collision.

When a house guild id is configured, the daemon SHALL also compute a house `/48` as `fd` plus the first five bytes of `blake3`, in network order. The hash input SHALL be the 20 ASCII bytes `mjolnir/house/ula/v1` concatenated with the raw 32-byte guild id and no separator. The router address SHALL place subnet id 0 in the next two bytes, network order, and the interface id in the last eight. That interface id SHALL be the first eight bytes of `blake3` over the node id's UTF-8 bytes, the same string the legacy helper hashes, clamped from 0 to 1. An app address SHALL use subnet id 1 and the first eight bytes of `blake3` over the raw 32-byte Ed25519 app public key, clamped from 0 to 1. A 40-bit prefix and a 64-bit interface id are deterministic and can collide; this capability does not detect or reallocate those collisions. Changing the CRDT gossip topic SHALL NOT change the house `/48`.

#### Scenario: Same secret, same address

- GIVEN a node id and the compiled mesh tag
- WHEN the daemon computes the legacy ULA on two boots
- THEN both results are identical and in `fd00::/8`

#### Scenario: No clerk

- GIVEN two nodes that have never gossiped
- WHEN each assigns its legacy ULA
- THEN neither consulted DHCP, a lease table, or a subnet claim

#### Scenario: Two fixture houses, two prefixes

- GIVEN guild id `11` repeated to 32 bytes, guild id `22` repeated to 32 bytes, and node id `node-example`
- WHEN each router address is computed
- THEN the first is `fd81:2b22:8aca::ddcd:86a6:64e9:f2df`, the second is `fd8f:78a9:2caa::ddcd:86a6:64e9:f2df`, and repeating either computation returns the same address

#### Scenario: App host is not in the router subnet

- GIVEN guild id `11` repeated to 32 bytes, node id `node-example`, and app key `ab` repeated to 32 bytes
- WHEN the router address and the app address are computed
- THEN the router address is `fd81:2b22:8aca::ddcd:86a6:64e9:f2df` and the app address is `fd81:2b22:8aca:1:bd5e:fa7a:5460:697`

#### Scenario: Legacy vector stays put

- GIVEN node id `node-example`
- WHEN the legacy ULA is computed
- THEN it is `fd53:6213:4797::ddcd:86a6:64e9:f2df`

### Requirement: ULA sits beside 10.254, not in iroh candidates

The daemon SHALL assign the legacy derived ULA on `br-mesh` next to the derived `10.254` backhaul address, prefix length `/64`, so peer legacy ULAs are on-link. It SHALL read `/etc/mjolnir/house-guild-id` at startup and on the reconcile pass that restores the legacy ULA. The file is valid only when, after trimming ASCII whitespace, it is exactly 64 hex digits of either case. When the file is valid the daemon SHALL assign that router's house `/128` on `br-mesh` and SHALL NOT assign a covering prefix of the house `/48`. The only house address the daemon MAY delete is the one recorded in `/var/lib/mjolnir/house-ula` from the last time that address was usable. A missing state file SHALL mean the daemon deletes no IPv6 address. A restart after the guild file changes or disappears SHALL remove only that recorded address, including when the previous process has exited. An address is usable only when it is present at prefix length 128 and is neither tentative nor DAD-failed. An add that errors while that usable address is already present SHALL count as assigned. An add that errors and leaves no usable address SHALL NOT count as assigned, SHALL NOT publish `house_ula`, and SHALL NOT replace the state file. After `br-mesh` is recreated, the same reconcile pass SHALL restore the current house `/128` while the file is valid. It SHALL NOT assign either address on `mjolnir0` or `br-lan` in this capability. It SHALL NOT add either address to iroh's QUIC candidate / bind set. A house router `/128` is known only when it is usable on `br-mesh`: prefix length 128, not tentative, and not DAD-failed. Both Babel reconcilers SHALL pass that address into the renderer on startup and on each reconcile, and SHALL drop a previous house allow when the guild changes, the address becomes unusable, or a restart finds none. When a house router `/128` is known, Babel on `br-mesh` and on `mjolnir0` SHALL redistribute that `/128`, SHALL allow the house `/48` at prefix length 128 with `eq 128`, and SHALL deny every other `fc00::/7` prefix. The new house lines SHALL NOT use `ge` or `le`. The existing gateway line `redistribute ip 0.0.0.0/0 le 0 metric 128` SHALL stay when the gateway is on and SHALL stay absent when it is off. The house allow SHALL appear before `in ip fc00::/7 deny` and `out ip fc00::/7 deny`. When no house router `/128` is known, Babel SHALL deny `fc00::/7` in and out and SHALL NOT add a house allow. On `mjolnir0`, an IPv6 `/128` route whose next hop is `fe80::H` in the overlay link-local shape SHALL select the already-open Iroh connection for the `10.254` peer that derives `H`. Withdrawing that route SHALL remove the selection on the next poll. A destination with no such route SHALL be dropped. The IPv4 route mirror SHALL keep reading only `ip -4 route`. The legacy `/64` SHALL NOT be exported. Stock OpenWrt `ula_prefix` SHALL stay filtered. Assigning the house `/128` SHALL NOT make any other address in that prefix on-link. Another house `/128` SHALL be reachable only as a Babel host route. IPv4 Babel next hops SHALL stay link-local. `10.254` SHALL remain the Iroh bind.

#### Scenario: Overlay IPv4 still the underlay

- GIVEN the legacy ULA is assigned and, when configured, the house `/128` is assigned
- WHEN iroh lists connection candidates (status or `/api/node`)
- THEN private IPv4 `10.254.x` is present and no candidate starts with `fd`

#### Scenario: On-link SSH

- GIVEN two nodes have assigned legacy ULAs on `br-mesh`
- WHEN an operator on one node SSHes to the peer's legacy ULA
- THEN the packet is on-link on `br-mesh` and does not require babel IPv6 or `mjolnir0`

#### Scenario: Directory publishes it

- GIVEN the legacy ULA is assigned
- WHEN a client `GET`s `/api/node`
- THEN the node object includes the legacy ULA, omitted when not yet assigned, and includes `house_ula` only when that house `/128` is usable on `br-mesh`

#### Scenario: Tentative or DAD-failed address is not published

- GIVEN the desired house `/128` is present but tentative or DAD-failed
- WHEN the node object is built
- THEN `house_ula` is omitted and the state file is not updated to that address

#### Scenario: Already-present address survives an add error

- GIVEN `br-mesh` already has the desired house `/128` usable at prefix length 128
- WHEN the add returns an error
- THEN the address counts as assigned, `house_ula` is that address, and the state file records it

#### Scenario: Failed add publishes nothing

- GIVEN a valid guild file and no usable house `/128` on `br-mesh`
- WHEN the add fails and the address is still absent
- THEN `house_ula` is omitted and the state file is left unchanged

#### Scenario: Bad guild file publishes nothing new

- GIVEN the guild file is missing, empty, or not 64 hex digits
- WHEN the node object is built
- THEN `house_ula` is omitted and no house `/128` is added

#### Scenario: Guild change drops the old host address

- GIVEN the state file records a house `/128` and that address is on `br-mesh`
- WHEN the guild file becomes a different valid guild id, or becomes missing or malformed, and reconcile runs
- THEN only the recorded address is removed, any new valid `/128` is assigned, and other IPv6 addresses on the interface stay

#### Scenario: Restart after the guild file changes

- GIVEN the daemon assigned a house `/128`, recorded it, and then exited
- WHEN the guild file is replaced or removed and a new process starts
- THEN the new process removes only the recorded address and does not remove the legacy `/64` or any other IPv6 address

#### Scenario: Interface loss restores the current host address

- GIVEN a valid guild file and a recorded house `/128`
- WHEN `br-mesh` is recreated without that address and reconcile runs
- THEN the desired `/128` is added again and `house_ula` returns once it is usable

#### Scenario: House host routes are allowed and the rest of fc00 stays denied

- GIVEN a house router `/128` in `fd81:2b22:8aca::/48` and Babel config rendered for `br-mesh` and for `mjolnir0`
- WHEN the config is inspected
- THEN each render redistributes that `/128`, allows `fd81:2b22:8aca::/48 eq 128` before `in ip fc00::/7 deny` and `out ip fc00::/7 deny`, and the new house lines contain neither `ge` nor `le`
- AND a stock OpenWrt `ula_prefix` is not redistributed

#### Scenario: No house guild keeps the blanket deny

- GIVEN no house router `/128`
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each contains `in ip fc00::/7 deny` and `out ip fc00::/7 deny` and no house `/48` allow

#### Scenario: Gateway on keeps its default route

- GIVEN the node is a gateway and a house router `/128` is known
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each still contains `redistribute ip 0.0.0.0/0 le 0 metric 128`

#### Scenario: Gateway off does not gain that line

- GIVEN the node is not a gateway and a house router `/128` is known
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN neither render contains `le 0`

#### Scenario: A house host route selects the open peer

- GIVEN `mjolnir0` has `fd81:2b22:8aca::2/128 via fe80::2` and `10.254.0.2` is an open connection
- WHEN a packet to `fd81:2b22:8aca::2` is read from the overlay
- THEN that connection is selected and `10.254.0.3` is not

#### Scenario: Withdrawal drops the selection

- GIVEN the route in the previous scenario was selected
- WHEN the route is deleted and the mirror polls again
- THEN a packet to `fd81:2b22:8aca::2` selects no connection

#### Scenario: An unknown destination is dropped

- GIVEN the open peer above and no route for `fd12:3456:789a::1` or `fd81:2b22:8aca::99`
- WHEN packets to those addresses are read
- THEN neither selects a connection

#### Scenario: The return host uses the same rule

- GIVEN `mjolnir0` also has this router's house `/128` via the peer's `fe80`
- WHEN a packet to that local house `/128` is read
- THEN the peer connection for that next hop is selected

#### Scenario: House address is a host route, not a covering prefix

- GIVEN a valid house guild id
- WHEN the daemon assigns addresses on `br-mesh`
- THEN the legacy prefix length is `/64`, the house address prefix length is `/128`, and no house `/64` or `/48` is installed
