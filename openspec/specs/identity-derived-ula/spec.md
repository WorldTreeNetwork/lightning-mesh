# identity-derived-ula

A node's legacy Unique Local Address is a hash of the compiled mesh tag
and the node id, assigned `/64` on `br-mesh`. When a house guild id is
configured, the daemon also assigns that router's house `/128` (subnet 0
of a guild-derived `/48`) on `br-mesh`. An app address uses subnet 1 of
the same `/48`. `10.254` is not assigned. Iroh binds the usable house
router `/128` and does not advertise `10.254.0.0/16`. The open connection
is keyed by `router_ula` of the local guild and the authenticated peer
endpoint id. Overlay link-local on `mjolnir0` is the blake3 interface id
of the node id, not the last 16 bits of `10.254`. When that router `/128`
is usable on `br-mesh`, Babel redistributes it and allows the house `/48`
at prefix length 128, then denies the rest of `fc00::/7`. The walked
fleet column and `DirectoryNode.backhaul_addr` stay `10.254`. Folded from
`add-identity-derived-ula` (2026-09-11), `add-house-ula-prefix`
(2026-11-01), `add-ula-host-forwarding` (2026-11-01),
`add-ula-node-reference` (2026-11-01), and `remove-v4-backhaul-pin`
(2026-11-02).

Code: `crates/mjolnir-mesh/src/tun/link.rs` (`ula_addr`, `house_ula_prefix`,
`router_ula`, `app_ula`), `crates/mjolnir-mesh/src/tun/neighbor.rs`
(`overlay_link_local`), `crates/mjolnir-mesh/src/tun/underlay.rs`
(outer route off `mjolnir0`), `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`
(`reconcile_house_ula`, `DirectoryNode.ula`, `DirectoryNode.house_ula`),
`crates/mjolnir-mesh/src/babel/config.rs` (house `/128` allow, then
`fc00::/7` deny), `crates/mjolnir-hello` (`GET /api/node`).

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

The daemon SHALL assign the legacy derived ULA on `br-mesh`, prefix length `/64`, so peer legacy ULAs are on-link. It SHALL read `/etc/mjolnir/house-guild-id` at startup and on the reconcile pass that restores the legacy ULA. The file is valid only when, after trimming ASCII whitespace, it is exactly 64 hex digits of either case. When the file is valid the daemon SHALL assign that router's house `/128` on `br-mesh` and SHALL NOT assign a covering prefix of the house `/48`. The only house address the daemon MAY delete is the one recorded in `/var/lib/mjolnir/house-ula` from the last time that address was usable. A missing state file SHALL mean the daemon deletes no IPv6 address. A restart after the guild file changes or disappears SHALL remove only that recorded address, including when the previous process has exited. An address is usable only when it is present at prefix length 128 and is neither tentative nor DAD-failed. An add that errors while that usable address is already present SHALL count as assigned. An add that errors and leaves no usable address SHALL NOT count as assigned, SHALL NOT publish `house_ula`, and SHALL NOT replace the state file. After `br-mesh` is recreated, the same reconcile pass SHALL restore the current house `/128` while the file is valid. It SHALL NOT assign either address on `mjolnir0` or `br-lan` in this capability. It SHALL NOT assign `10.254` on `br-mesh`, `mjolnir0`, or `br-lan`. Birthday-collision healing of `10.254` SHALL NOT run. The daemon SHALL NOT create the Iroh endpoint until the house router `/128` is usable on `br-mesh`. A missing guild file, a malformed guild file, a tentative address, a DAD-failed address, or a failed bind SHALL leave no endpoint, SHALL NOT bind `0.0.0.0`, and SHALL NOT fall back to every interface. The next reconcile pass SHALL retry. When that usable address is lost, replaced by a different house `/128`, or `br-mesh` is recreated, the daemon SHALL close the endpoint and SHALL drop its connection keys. It SHALL bind again only after the replacement address is usable. The builder SHALL call `clear_ip_transports` and then bind only that `/128`. The bound socket set SHALL NOT include `0.0.0.0`, and advertised candidates SHALL NOT include an address in `10.254.0.0/16`. An explicit `--bind` SHALL replace the house bind. A `--bind` of `0.0.0.0` SHALL be rejected. A failed explicit bind SHALL NOT fall back. LAN mode, overlay mode, and internet mode SHALL use this rule once the `/128` is usable. Overlay mode SHALL NOT assign the house `/128` on `mjolnir0`. A house router `/128` is known only when it is usable on `br-mesh`: prefix length 128, not tentative, and not DAD-failed. Both Babel reconcilers SHALL pass that address into the renderer on startup and on each reconcile, and SHALL drop a previous house allow when the guild changes, the address becomes unusable, or a restart finds none. When a house router `/128` is known, Babel on `br-mesh` and on `mjolnir0` SHALL redistribute that `/128`, SHALL allow the house `/48` at prefix length 128 with `eq 128`, and SHALL deny every `fc00::/7` prefix that is neither that house `/48` nor a counted admitted `/48`. The new house lines SHALL NOT use `ge` or `le`. The existing gateway line `redistribute ip 0.0.0.0/0 le 0 metric 128` SHALL stay when the gateway is on and SHALL stay absent when it is off. The house allow SHALL appear before `in ip fc00::/7 deny` and `out ip fc00::/7 deny`. The daemon SHALL read `/var/lib/mjolnir/admitted-prefixes` on each Babel reconcile. That file is a projection the publisher supplies. The reader SHALL NOT read it as a guild log and SHALL NOT admit a prefix by itself. The publisher SHALL put only admitted foreign house `/48`s in the file and SHALL replace the file when the house changes. A missing file SHALL mean an empty list. Any other read error SHALL keep the last successful projection, and SHALL NOT render a removal when this process has never read one. A line SHALL count only when it names an `fd00::/8` network address whose last 80 bits are zero. The reader SHALL skip `ula_prefix()`, which for this build is `fd53:6213:4797::/48`, and SHALL skip the house `/48` parsed from a valid `/etc/mjolnir/house-guild-id` even when the house `/128` is not usable. A missing or malformed guild file SHALL exclude no house prefix. The reader SHALL NOT reject `fd12:3456:789a::/48` by a stock denylist; that prefix is forbidden publisher input. For each prefix that counts, Babel on `br-mesh` and on `mjolnir0` SHALL allow that `/48` at prefix length 128 with `eq 128` before both `fc00::/7` denies. Those admitted lines SHALL NOT use `ge` or `le`. The daemon SHALL NOT redistribute a foreign prefix as a local address and SHALL NOT assign a foreign prefix, or any longer prefix inside it, on `br-mesh`, `mjolnir0`, or `br-lan`. A projection change SHALL be applied only when Babel start or restart succeeds. Both reconcilers SHALL keep a failed start or restart pending and SHALL retry it on the next paced pass even when the rendered text is unchanged. When no house router `/128` is known and no admitted prefix counts, Babel SHALL deny `fc00::/7` in and out and SHALL NOT add a house allow. When no house router `/128` is known and an admitted prefix counts, Babel SHALL allow that admitted `/48` at prefix length 128 and SHALL NOT add a house allow. The open Iroh connection SHALL be keyed by `router_ula` of the local house guild id and the authenticated peer endpoint id. That endpoint id is the peer node id string. The daemon SHALL NOT read babeld's `router-id` as that key. babeld 1.13 `router-id` is a modified EUI-64 or a MAC-48, not an IPv6 address, and Babel SHALL NOT be required to carry the house `/128`. A missing or malformed guild file SHALL yield no key, and those routes SHALL select nothing. The daemon SHALL assign on `mjolnir0` one link-local whose prefix is `fe80::/64` and whose interface id is the first 8 bytes of `blake3` of the node id UTF-8, with a zero id clamped to 1, the same interface id as that node's router `/128`. It SHALL NOT derive that link-local from `10.254`. That link-local is a scoped Babel protocol address and SHALL NOT be reversed into a connection key. IPv4 and IPv6 Babel next hops SHALL stay that link-local. They SHALL NOT be represented as a `10.254` gateway. A Babel Hello or IHU, UDP port 6696, whose IPv6 source is link-local and which arrives on one authenticated Iroh connection, SHALL associate that source with that connection's key. The association is scoped to that connection. A Hello heard only on `br-mesh` SHALL NOT by itself key an Iroh connection. A second distinct link-local source on that same connection SHALL make the association ambiguous. The daemon SHALL NOT overwrite one peer with the other. An ambiguous association SHALL select nothing. When two live connections are associated with the same link-local, a route via that link-local SHALL select nothing. Disconnect SHALL drop that connection's key and its link-local association on the next poll. A Babel route whose `via` is one associated link-local SHALL select the connection stored under that key. The destination address SHALL NOT be the key. An admitted foreign host SHALL be selected by the same-house transit peer that announced it, using the local guild and that peer's endpoint id. It SHALL NOT be selected by `router_ula` of a foreign guild. This change SHALL NOT key a direct foreign-house Iroh peer by a foreign guild. An IPv4 packet SHALL stay IPv4, its payload bytes SHALL be unchanged, and it SHALL select the connection of the Babel neighbor that announced its prefix. The daemon SHALL NOT use a `10.254` gateway from `ip -4 route` as the connection key. Before a destination host route exists, unicast to one associated link-local SHALL select the connection that last received a Hello or IHU from that source, and an ambiguous or unknown source SHALL select nothing. Multicast to `ff02::1:6` UDP 6696 SHALL be sent on every established connection. Outer Iroh UDP SHALL NOT leave on a route whose device is `mjolnir0`. Babel on `br-mesh` SHALL supply the underlay route to a peer house `/128`. It SHALL redistribute the local house `/128` and SHALL accept a peer house `/128` only as a host route whose device is `br-mesh`. The next hop of that route is the neighbor's link-local on `br-mesh`. That link-local is not the overlay `fe80` on `mjolnir0` and is not the connection key. The daemon SHALL NOT install a static host route for the peer. In overlay mode this `br-mesh` Babel runs in addition to Babel on `mjolnir0`, and it SHALL be running before the first dial. It SHALL NOT wait for an Iroh connection. The endpoint MAY exist before any peer route exists. A dial to a peer `/128` SHALL wait until the `br-mesh` host route is present, or until a relay is enabled and the peer is off-link. Outer Iroh datagrams sourced from the house `/128` and destined to a peer house `/128` SHALL use that `br-mesh` route only. A route to the same destination whose device is `mjolnir0` SHALL NOT be selected for those datagrams, whatever its metric. When the `br-mesh` route is absent and relays are disabled, the dial SHALL wait and retry, and outer datagrams to that peer SHALL NOT be sent. When relays are enabled and the `br-mesh` route is absent, the dial MAY use the relay. Relay datagrams SHALL use an underlay path whose device is not `mjolnir0`. They SHALL NOT use `mjolnir0`. Losing the `br-mesh` route SHALL NOT move an existing connection's outer UDP onto `mjolnir0`. Inner packets to a house host `/128` SHALL still follow Babel on `mjolnir0`. Withdrawing a route SHALL remove that destination's selection on the next poll and SHALL NOT by itself drop the connection. A destination with no associated neighbor SHALL be dropped. The walked fleet column SHALL stay `10.254` in this change. This requirement SHALL NOT rewrite that column. The legacy `/64` SHALL NOT be exported. Stock OpenWrt `ula_prefix` SHALL stay filtered. Assigning the house `/128` SHALL NOT make any other address in that prefix on-link. Another house `/128` SHALL be reachable only as a Babel host route: on `br-mesh` for the outer dial, and on `mjolnir0` for inner forwarding.

#### Scenario: Reboot dials the house /128

- GIVEN the house router `/128` is usable on `br-mesh`
- WHEN the daemon restarts and Iroh lists connection candidates
- THEN the house router `/128` is bound and dialed, `10.254` is not assigned, and no candidate is in `10.254.0.0/16` or bound on `0.0.0.0`

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

- GIVEN guild `11` repeated, an authenticated peer endpoint id `peer-example`, and a Babel route for `fd81:2b22:8aca::2/128` via `fe80::403d:2741:12c1:e5c0` after one Hello from that source arrived on that peer's connection
- WHEN a packet to `fd81:2b22:8aca::2` is read from the overlay
- THEN the connection keyed by `fd81:2b22:8aca:0:403d:2741:12c1:e5c0` is selected
- AND that key is `router_ula` of the local guild and `peer-example`
- AND the destination `fd81:2b22:8aca::2` is not the connection key
- AND reversing `fe80::H` into `10.254` selects nothing

#### Scenario: Withdrawal drops the selection

- GIVEN the route in the previous scenario was selected
- WHEN the route is deleted and the mirror polls again
- THEN a packet to `fd81:2b22:8aca::2` selects no connection
- AND the peer connection stays open

#### Scenario: An unknown destination is dropped

- GIVEN the open peer above and no route for `fd12:3456:789a::1` or `fd81:2b22:8aca::99`
- WHEN packets to those addresses are read
- THEN neither selects a connection

#### Scenario: The return host uses the same rule

- GIVEN a Babel route for this router's house `/128` `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df` via `fe80::403d:2741:12c1:e5c0`, associated with endpoint id `peer-example`
- WHEN a packet to that local house `/128` is read
- THEN the connection keyed by `fd81:2b22:8aca:0:403d:2741:12c1:e5c0` is selected

#### Scenario: Two claims on one link-local select nothing

- GIVEN two authenticated connections are both associated with `fe80::403d:2741:12c1:e5c0`
- WHEN a route via that link-local is polled
- THEN the route selects nothing

#### Scenario: Two link-locals on one connection select nothing

- GIVEN one authenticated connection has received Hellos sourced from `fe80::403d:2741:12c1:e5c0` and from `fe80::ddcd:86a6:64e9:f2df`
- WHEN a route via either link-local is polled
- THEN the route selects nothing

#### Scenario: An IPv4 prefix uses the announcing neighbor

- GIVEN the neighbor keyed by `fd81:2b22:8aca:0:403d:2741:12c1:e5c0` announced `10.42.1.0/24` via `fe80::403d:2741:12c1:e5c0`
- WHEN a packet to `10.42.1.50` is read from the overlay
- THEN that connection is selected and the packet bytes stay IPv4

#### Scenario: Disconnect drops the key

- GIVEN the peer connection keyed by `fd81:2b22:8aca:0:403d:2741:12c1:e5c0`
- WHEN that connection disconnects and the mirror polls again
- THEN its key and its link-local association select nothing

#### Scenario: Cold start with relays disabled learns the underlay route

- GIVEN relays are disabled, no Iroh connection exists, no peer host route was installed by hand, the local house `/128` is usable on `br-mesh`, and Babel on `br-mesh` is exporting that `/128`
- WHEN Babel on `br-mesh` installs `fd81:2b22:8aca:0:403d:2741:12c1:e5c0/128` with device `br-mesh`
- THEN the daemon dials that address on `br-mesh`
- AND one Hello from `fe80::403d:2741:12c1:e5c0` on that connection associates the overlay link-local
- AND a packet to `10.42.1.50` and a packet to `fd8f:78a9:2caa::2` select that connection

#### Scenario: A peer /128 route on mjolnir0 does not carry outer Iroh

- GIVEN the underlay route for `fd81:2b22:8aca:0:403d:2741:12c1:e5c0/128` uses device `br-mesh`, and the same prefix is also installed with device `mjolnir0`
- WHEN outer Iroh UDP to that peer is sent
- THEN that UDP uses device `br-mesh` and does not use `mjolnir0`
- AND an inner packet to a house host still follows Babel on `mjolnir0`

#### Scenario: Losing the underlay route does not fall onto mjolnir0

- GIVEN relays are disabled and the peer `/128` has routes on both `br-mesh` and `mjolnir0`
- WHEN the `br-mesh` route is removed
- THEN outer UDP to that peer is not sent
- AND it is not sent on `mjolnir0`

#### Scenario: No usable address leaves no endpoint

- GIVEN the guild file is missing, or the house `/128` is tentative or DAD-failed
- WHEN reconcile runs
- THEN no Iroh endpoint exists, `0.0.0.0` is not bound, and the next reconcile retries

#### Scenario: Losing the house address closes the endpoint

- GIVEN an endpoint is bound to the usable house `/128`
- WHEN that address is removed or replaced and reconcile runs
- THEN the endpoint is closed, its connection keys are dropped, and a new endpoint is bound only after the replacement address is usable

#### Scenario: Explicit bind rejects the IPv4 wildcard

- GIVEN `--bind` is `0.0.0.0`
- WHEN the endpoint would be built
- THEN the bind is rejected and the daemon does not fall back to every interface

#### Scenario: House address is a host route, not a covering prefix

- GIVEN a valid house guild id
- WHEN the daemon assigns addresses on `br-mesh`
- THEN the legacy prefix length is `/64`, the house address prefix length is `/128`, and no house `/64` or `/48` is installed

#### Scenario: An admitted foreign /48 is a host allow

- GIVEN a house router `/128` in `fd81:2b22:8aca::/48` and the admitted projection contains `fd8f:78a9:2caa::/48`
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each render redistributes the house `/128`, allows `fd81:2b22:8aca::/48 eq 128` and `fd8f:78a9:2caa::/48 eq 128` before both `fc00::/7` denies, and does not redistribute the foreign prefix as a local address
- AND neither render contains `fd12:3456:789a`
- AND the admitted lines contain neither `ge` nor `le`

#### Scenario: The compiled legacy prefix and this house do not count

- GIVEN a valid guild file for guild `11` repeated and the projection contains `fd53:6213:4797::/48`, `fd81:2b22:8aca::/48`, and `fd8f:78a9:2caa::/48`
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each render allows `fd8f:78a9:2caa::/48 eq 128` and does not allow `fd53:6213:4797::/48` or `fd81:2b22:8aca::/48` a second time

#### Scenario: Own-house exclusion does not need a usable address

- GIVEN guild `11` repeated is configured, the house `/128` is not usable, and the projection contains `fd81:2b22:8aca::/48` and `fd8f:78a9:2caa::/48`
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each render allows only `fd8f:78a9:2caa::/48 eq 128` and redistributes no local house host

#### Scenario: No guild file excludes no house prefix

- GIVEN the guild file is missing and the projection contains `fd81:2b22:8aca::/48` and `fd8f:78a9:2caa::/48`
- WHEN Babel config is rendered
- THEN both prefixes are allowed at `eq 128` and no local house host is redistributed

#### Scenario: A guild change excludes the new house

- GIVEN the projection still contains `fd81:2b22:8aca::/48` and `fd8f:78a9:2caa::/48` and the guild file changes from guild `11` repeated to guild `22` repeated
- WHEN the next reconcile renders Babel
- THEN `fd8f:78a9:2caa::/48` is excluded and `fd81:2b22:8aca::/48` is allowed

#### Scenario: A supplied stock prefix is not a reader denylist

- GIVEN the renderer is given `fd12:3456:789a::/48` directly and no guild file
- WHEN Babel config is rendered
- THEN `fd12:3456:789a::/48 eq 128` is present
- AND a render whose projection omits that prefix does not contain `fd12:3456:789a`

#### Scenario: A failed restart retries the removal

- GIVEN `fd8f:78a9:2caa::/48` was allowed and the projection then drops it
- WHEN the config is written, Babel restart fails, and the next pass sees the same inputs
- THEN that pass retries the restart and the running config denies that prefix

#### Scenario: An unreadable projection keeps the last good list

- GIVEN the last successful projection allowed `fd8f:78a9:2caa::/48`
- WHEN the next read fails with an error other than a missing file
- THEN that prefix stays allowed and the render is not cleared

#### Scenario: A missing projection changes nothing

- GIVEN a house router `/128` is known and `/var/lib/mjolnir/admitted-prefixes` is missing
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each render matches the house-only allow and still denies the rest of `fc00::/7`

#### Scenario: The foreign prefix is not on-link

- GIVEN the admitted projection contains `fd8f:78a9:2caa::/48`
- WHEN the daemon reconciles addresses
- THEN no address in `fd8f:78a9:2caa::/48` is assigned on `br-mesh`, `mjolnir0`, or `br-lan`

#### Scenario: An admitted host route selects the same-house transit peer

- GIVEN guild `11` repeated, the peer endpoint id `peer-example` is associated with `fe80::403d:2741:12c1:e5c0`, and Babel has `fd8f:78a9:2caa::2/128` via that link-local
- WHEN a packet to `fd8f:78a9:2caa::2` is read from the overlay
- THEN the connection keyed by `fd81:2b22:8aca:0:403d:2741:12c1:e5c0` is selected
- AND the key is not a router address in `fd8f:78a9:2caa::/48`
- AND no address in `fd8f:78a9:2caa::/48` is bound, and `10.254` is not bound

### Requirement: The house /128 is the operator backhaul reference

The directory, Lightning Admin's address list, and the fleet inventory SHALL present a node's house router `/128` as the backhaul reference. The string SHALL be `router_ula` in its Rust `Display` form. `10.254` SHALL stay visible on each of those surfaces as the transport pin. The directory field `backhaul_addr` and the fleet column that rollout walks SHALL stay `10.254`. A directory neighbor's `house_ula` SHALL be `router_ula` of this node's house guild id and that neighbor's node id when `/etc/mjolnir/house-guild-id` is valid, and SHALL be omitted when the file is missing or malformed. Admin SHALL show the probed `house_ula` as the reference and the probed `backhaul_addr` as the transport pin. The fleet printer SHALL use `router_ula` and SHALL NOT implement a second hash. Pending inventory lines with no node id SHALL NOT gain a `/128`. This requirement SHALL NOT change the Iroh bind, SHALL NOT assign an address, and SHALL NOT open a live radio.

#### Scenario: A neighbor shows the fixture router address

- GIVEN this node's guild file is `11` repeated to 32 bytes and a directory neighbor has node id `node-example`
- WHEN the directory is built
- THEN that neighbor's `house_ula` is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`
- AND the neighbor's transport address is still its `10.254` pin

#### Scenario: No guild file omits the reference

- GIVEN the guild file is missing
- WHEN the directory is built and Admin renders a probed node that has `backhaul_addr` `10.254.0.2` and no `house_ula`
- THEN the neighbor omits `house_ula` and Admin still shows `10.254.0.2` as the transport pin

#### Scenario: The fleet printer uses the same function

- GIVEN a member line whose node id is `node-example`, whose transport column is `10.254.0.2`, and whose guild id is `11` repeated to 32 bytes
- WHEN the fleet inventory is printed
- THEN the line shows `10.254.0.2` and `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`
- AND the walked inventory column is still the transport address

#### Scenario: An operator reads the other site

- GIVEN Admin's address list has probed a node of this house whose `house_ula` is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df` and whose `backhaul_addr` is `10.254.0.2`
- WHEN an operator reads that row
- THEN the reference they copy is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df` and `10.254.0.2` is still labeled as the transport pin
