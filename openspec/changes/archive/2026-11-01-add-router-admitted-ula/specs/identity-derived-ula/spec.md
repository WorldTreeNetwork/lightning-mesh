## MODIFIED Requirements

### Requirement: ULA sits beside 10.254, not in iroh candidates
<!-- base: 328ea142dcf5ddad26016697f94f6b120212df9f openspec/specs/identity-derived-ula/spec.md -->

The daemon SHALL assign the legacy derived ULA on `br-mesh` next to the derived `10.254` backhaul address, prefix length `/64`, so peer legacy ULAs are on-link. It SHALL read `/etc/mjolnir/house-guild-id` at startup and on the reconcile pass that restores the legacy ULA. The file is valid only when, after trimming ASCII whitespace, it is exactly 64 hex digits of either case. When the file is valid the daemon SHALL assign that router's house `/128` on `br-mesh` and SHALL NOT assign a covering prefix of the house `/48`. The only house address the daemon MAY delete is the one recorded in `/var/lib/mjolnir/house-ula` from the last time that address was usable. A missing state file SHALL mean the daemon deletes no IPv6 address. A restart after the guild file changes or disappears SHALL remove only that recorded address, including when the previous process has exited. An address is usable only when it is present at prefix length 128 and is neither tentative nor DAD-failed. An add that errors while that usable address is already present SHALL count as assigned. An add that errors and leaves no usable address SHALL NOT count as assigned, SHALL NOT publish `house_ula`, and SHALL NOT replace the state file. After `br-mesh` is recreated, the same reconcile pass SHALL restore the current house `/128` while the file is valid. It SHALL NOT assign either address on `mjolnir0` or `br-lan` in this capability. It SHALL NOT add either address to iroh's QUIC candidate / bind set. A house router `/128` is known only when it is usable on `br-mesh`: prefix length 128, not tentative, and not DAD-failed. Both Babel reconcilers SHALL pass that address into the renderer on startup and on each reconcile, and SHALL drop a previous house allow when the guild changes, the address becomes unusable, or a restart finds none. When a house router `/128` is known, Babel on `br-mesh` and on `mjolnir0` SHALL redistribute that `/128`, SHALL allow the house `/48` at prefix length 128 with `eq 128`, and SHALL deny every `fc00::/7` prefix that is neither that house `/48` nor a counted admitted `/48`. The new house lines SHALL NOT use `ge` or `le`. The existing gateway line `redistribute ip 0.0.0.0/0 le 0 metric 128` SHALL stay when the gateway is on and SHALL stay absent when it is off. The house allow SHALL appear before `in ip fc00::/7 deny` and `out ip fc00::/7 deny`. The daemon SHALL read `/var/lib/mjolnir/admitted-prefixes` on each Babel reconcile. That file is a projection the publisher supplies. The reader SHALL NOT read it as a guild log and SHALL NOT admit a prefix by itself. The publisher SHALL put only admitted foreign house `/48`s in the file and SHALL replace the file when the house changes. A missing file SHALL mean an empty list. Any other read error SHALL keep the last successful projection, and SHALL NOT render a removal when this process has never read one. A line SHALL count only when it names an `fd00::/8` network address whose last 80 bits are zero. The reader SHALL skip `ula_prefix()`, which for this build is `fd53:6213:4797::/48`, and SHALL skip the house `/48` parsed from a valid `/etc/mjolnir/house-guild-id` even when the house `/128` is not usable. A missing or malformed guild file SHALL exclude no house prefix. The reader SHALL NOT reject `fd12:3456:789a::/48` by a stock denylist; that prefix is forbidden publisher input. For each prefix that counts, Babel on `br-mesh` and on `mjolnir0` SHALL allow that `/48` at prefix length 128 with `eq 128` before both `fc00::/7` denies. Those admitted lines SHALL NOT use `ge` or `le`. The daemon SHALL NOT redistribute a foreign prefix as a local address and SHALL NOT assign a foreign prefix, or any longer prefix inside it, on `br-mesh`, `mjolnir0`, or `br-lan`. A projection change SHALL be applied only when Babel start or restart succeeds. Both reconcilers SHALL keep a failed start or restart pending and SHALL retry it on the next paced pass even when the rendered text is unchanged. When no house router `/128` is known and no admitted prefix counts, Babel SHALL deny `fc00::/7` in and out and SHALL NOT add a house allow. When no house router `/128` is known and an admitted prefix counts, Babel SHALL allow that admitted `/48` at prefix length 128 and SHALL NOT add a house allow. On `mjolnir0`, an IPv6 `/128` route whose next hop is `fe80::H` in the overlay link-local shape SHALL select the already-open Iroh connection for the `10.254` peer that derives `H`. Withdrawing that route SHALL remove the selection on the next poll. A destination with no such route SHALL be dropped. The IPv4 route mirror SHALL keep reading only `ip -4 route`. The legacy `/64` SHALL NOT be exported. Stock OpenWrt `ula_prefix` SHALL stay filtered. Assigning the house `/128` SHALL NOT make any other address in that prefix on-link. Another house `/128` SHALL be reachable only as a Babel host route. IPv4 Babel next hops SHALL stay link-local. `10.254` SHALL remain the Iroh bind.

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

#### Scenario: An admitted host route selects the open peer

- GIVEN `mjolnir0` has `fd8f:78a9:2caa::2/128 via fe80::2` and `10.254.0.2` is an open connection
- WHEN a packet to `fd8f:78a9:2caa::2` is read from the overlay
- THEN that connection is selected and no `fd` address is bound for Iroh
