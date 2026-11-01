# add-admitted-prefix-route

> **ACTIVE BUILD**

## Why

A signed pair says the two houses admit each other's prefix. Nothing yet turns that pair into a route, so the foreign `/48` is not reachable through the app TUN.

## What

- While both admit ops are in the set the caller presents, Admin installs a device route for the foreign `/48` on the open app TUN.
- Omitting either op removes that route. There is no withdrawal op and no tombstone.
- A stock prefix, the local house `/48`, and any pair that does not include this house get no route.

## Impact

- Capabilities: ADDED admitted-prefix-route
- ADRs: none

## User journey & surfaces

No new UI because the route follows the admit ops the caller already presents and the TUN `open_app_tun` already opened. The guild surface does not gain a button.

## Out of scope

- Router Babel export of the foreign prefix (`add-router-admitted-ula`)
- A new admit withdrawal kind
- Binding `fd` as an Iroh candidate
