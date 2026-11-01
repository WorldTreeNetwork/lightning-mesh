## MODIFIED Requirements

### Requirement: ULA sits beside 10.254, not in iroh candidates
<!-- base: 5f001b0a9674e5f073bb0de84744b3adf04f865c openspec/specs/identity-derived-ula/spec.md -->

The daemon SHALL assign the legacy derived ULA on `br-mesh` next to the derived `10.254` backhaul address, prefix length `/64`, so peer legacy ULAs are on-link. It SHALL read `/etc/mjolnir/house-guild-id` at startup and on the reconcile pass that restores the legacy ULA. The file is valid only when, after trimming ASCII whitespace, it is exactly 64 hex digits of either case. When the file is valid the daemon SHALL assign that router's house `/128` on `br-mesh` and SHALL NOT assign a covering prefix of the house `/48`. The only house address the daemon MAY delete is the one recorded in `/var/lib/mjolnir/house-ula` from the last time that address was usable. A missing state file SHALL mean the daemon deletes no IPv6 address. A restart after the guild file changes or disappears SHALL remove only that recorded address, including when the previous process has exited. An address is usable only when it is present at prefix length 128 and is neither tentative nor DAD-failed. An add that errors while that usable address is already present SHALL count as assigned. An add that errors and leaves no usable address SHALL NOT count as assigned, SHALL NOT publish `house_ula`, and SHALL NOT replace the state file. After `br-mesh` is recreated, the same reconcile pass SHALL restore the current house `/128` while the file is valid. It SHALL NOT assign either address on `mjolnir0` or `br-lan` in this capability. It SHALL NOT add either address to iroh's QUIC candidate / bind set. When a house router `/128` is known, Babel on `br-mesh` and on `mjolnir0` SHALL redistribute that `/128`, SHALL allow the house `/48` at prefix length 128 with `eq 128` and without `ge` or `le`, and SHALL deny every other `fc00::/7` prefix. The house allow SHALL appear before `in ip fc00::/7 deny` and `out ip fc00::/7 deny`. When no house router `/128` is known, Babel SHALL deny `fc00::/7` in and out and SHALL NOT add a house allow. The legacy `/64` SHALL NOT be exported. Stock OpenWrt `ula_prefix` SHALL stay filtered. Assigning the house `/128` SHALL NOT make any other address in that prefix on-link. Another house `/128` SHALL be reachable only as a Babel host route. IPv4 Babel next hops SHALL stay link-local. `10.254` SHALL remain the Iroh bind.

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
- THEN each render redistributes that `/128`, allows `fd81:2b22:8aca::/48 eq 128` before `in ip fc00::/7 deny` and `out ip fc00::/7 deny`, and contains neither `ge` nor `le`
- AND a stock OpenWrt `ula_prefix` is not redistributed

#### Scenario: No house guild keeps the blanket deny

- GIVEN no house router `/128`
- WHEN Babel config is rendered for `br-mesh` and for `mjolnir0`
- THEN each contains `in ip fc00::/7 deny` and `out ip fc00::/7 deny` and no house `/48` allow

#### Scenario: House address is a host route, not a covering prefix

- GIVEN a valid house guild id
- WHEN the daemon assigns addresses on `br-mesh`
- THEN the legacy prefix length is `/64`, the house address prefix length is `/128`, and no house `/64` or `/48` is installed
