# add-admin-app-tun

> **ACTIVE BUILD**

## Why

The app `/128` is already a pure function of the house guild and an Ed25519 key. Lightning Admin does not open an interface for it, so two admins cannot exchange a packet at those addresses.

## What

- Admin computes its app `/128` with `app_ula` from the house guild id and the actor public key (subnet 1).
- On Linux it opens one TUN, assigns that `/128`, and passes one IP packet through `spawn_encap_pair` to the peer selected by a 32-byte node id.
- The existing guild surface shows the address. No new button. No foreign `/48`, no Babel change, no client SSID write.

## Impact

- Capabilities: ADDED admin-app-tun
- ADRs: none

## User journey & surfaces

An operator who already has an actor seed opens Lightning Admin and selects a house guild on the existing guild surface. That surface shows the app `/128` and does not open a TUN. A second process on the same host calls `open_app_tun` with that guild id, the actor public key, one peer node id, that peer's app `/128`, and the `DatagramConn` bound to that id. Empty seed or empty guild shows no address and opens no TUN. An unknown peer id, a failed TUN open, or a non-Linux build shows the address as not up and does not write a route.

## Out of scope

- Routing an admitted foreign `/48` (`add-admitted-prefix-route`)
- Router Babel export (`add-ula-host-forwarding`, `add-router-admitted-ula`)
- Binding the ULA as an Iroh candidate or removing `10.254` (`remove-v4-backhaul-pin`)
- Phones, AAAA, client-LAN RA
