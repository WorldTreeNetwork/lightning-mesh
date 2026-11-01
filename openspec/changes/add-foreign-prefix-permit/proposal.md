# add-foreign-prefix-permit

> **ACTIVE BUILD**

Preparation: the house-ula-backhaul grant (user, 2026-10-31, "activate and run") covers this dependency-ready node. `add-house-ula-prefix` is folded. The spike on this node is skipped. Fresh advise is owed before any implementation.

## Why

Two houses can reach each other only after each has accepted the other's prefix. One signature must install nothing, and a stock router prefix must never count as a house.

## What

- Add a signed guild op, `keyspace.admit-prefix`, on the existing guild log.
- Admit a pair only when both directions verify and each named prefix is that peer's house `/48`.
- Leave guild membership, the client SSID, Babel, and the Admin TUN unchanged.

## Impact

- Capabilities: ADDED `foreign-prefix-permit`. MODIFIED `admin-guild` (the log may carry the new kind).
- ADRs: none in this change. The route that uses an admission is a later node.

## User journey & surfaces

No new UI because the operator signs the op with the existing guild log (`lightning-guild` / `sign_guild`). The log reader must keep listing the guild after the op is appended. A window control that installs a route is a later change.

## Out of scope

- Exporting the admitted `/128`s on Babel (`add-router-admitted-ula`).
- Routing an admitted `/48` through the Admin TUN (`add-admitted-prefix-route`).
- Copying one house's log onto the other house or onto a router.
- Treating two sites of one house as a foreign admission.
- Merging ownership or `10.42` claims.
- Phones, AAAA, client-LAN RA, and the `10.254` Iroh pin.
