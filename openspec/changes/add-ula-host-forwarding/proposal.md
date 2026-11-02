# add-ula-host-forwarding

> **ACTIVE BUILD**

## Why

Two routers of one house cannot reach each other's house `/128` today. Babel denies all of `fc00::/7`, and a covering prefix would make the kernel treat the other site as on-link. The house address has to be a host route. On the overlay, that host route has to select an already-open Iroh connection, because `OverlayRouter::next_hop` currently drops every `fd` destination.

## What

- When a house guild id is configured, both Babel renders allow that house `/48` at prefix length 128, then deny the rest of `fc00::/7`.
- The node's own house `/128` is redistributed. Other local addresses, including the legacy `/64` and a stock OpenWrt prefix, are not.
- The legacy `/64` stays on `br-mesh`. No house `/64` or `/48` is installed. `10.254` stays the Iroh bind. IPv4 next hops stay link-local.
- A house `/128` route on `mjolnir0` whose next hop is `fe80::H` selects the already-open connection for that `10.254` peer. Deleting the route drops the selection. `fd` is not an Iroh candidate.

## Impact

- Capabilities: MODIFIED identity-derived-ula
- ADRs: none

## User journey & surfaces

No new UI because an operator reaches the other site with the house `/128` the daemon already computes. The window that shows that address is a later node.

## Out of scope

- Admitted foreign `/48` export (`add-router-admitted-ula`)
- Admin TUN and foreign routes (`add-admin-app-tun`, `add-admitted-prefix-route`)
- Using the house `/128` as the Iroh dial address (`remove-v4-backhaul-pin`)
- Dropping the compiled legacy `/64` (dual-assign stays until a later cutover)
- Phones, AAAA, client-LAN RA, public NAT66, `10.42` pool changes
