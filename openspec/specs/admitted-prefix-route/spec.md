# admitted-prefix-route

A signed pair says the two houses admit each other's prefix. While both
admit ops are in the set the caller presents, and the app TUN session is
up, Admin installs a device route for the foreign house `/48` on that
TUN. Omitting either op removes the route. There is no withdrawal op.
A stock prefix, this house's own `/48`, and a pair that does not include
this house get no route. Babel and the Iroh bind set stay unchanged.
Folded from `add-admitted-prefix-route` (2026-11-01).

Code: `admin/src-tauri/src/app_tun.rs` (`sync_admitted_routes`,
`prefixes_for_house`).

## Requirements

### Requirement: An admitted foreign prefix is a TUN device route

Admin SHALL install a route for a foreign house `/48` only when `admitted_pairs` contains a pair that includes the house that opened the app TUN session and that session is up. The house id SHALL be the one stored at open. A later sync SHALL NOT accept a different guild. The route SHALL point at that TUN and SHALL NOT assign the `/48` on the interface. Omitting either admit op, so the pair is absent, SHALL remove the route. A failed deletion, or a failed observation of whether the route is gone, SHALL keep the prefix owned and SHALL surface the failure so the next sync retries. A prefix this session wrote SHALL stay owned when the observation of that write fails. Confirmed absence SHALL drop it. There SHALL be no withdrawal op. A stock OpenWrt prefix, this house's own `/48`, and a pair that does not include this house SHALL NOT gain a route. This SHALL NOT change Babel or the Iroh bind set.

#### Scenario: Both directions install the foreign prefix

- GIVEN this house is guild `11` repeated and the presented set admits guild `22` repeated
- WHEN the app TUN session syncs routes
- THEN `fd8f:78a9:2caa::/48` is a route on that TUN and `fd81:2b22:8aca::/48` is not assigned on it

#### Scenario: Dropping one direction removes the route

- GIVEN the foreign `/48` route is installed
- WHEN the caller presents no pair
- THEN that `/48` route is gone

#### Scenario: A stock prefix is not routed

- GIVEN the presented pair admits only the foreign house
- WHEN routes are listed
- THEN `fd12:3456:789a::/48` is absent

#### Scenario: The open session keeps its own house

- GIVEN the TUN was opened for guild `11` repeated
- WHEN the caller presents a valid pair of guild `11` and guild `22` as if the local house were guild `22`, or presents a pair of two other guilds
- THEN `fd81:2b22:8aca::/48` is not a route on that TUN and no prefix from the unrelated pair is installed

#### Scenario: A failed delete is retried

- GIVEN the foreign `/48` route is installed and deletion fails
- WHEN the caller presents no pair
- THEN the route is still present and the sync reports failure
- WHEN the next sync presents no pair and deletion succeeds
- THEN that `/48` route is gone
