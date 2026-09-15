# make-captive-portal-uplink-aware

> **ACTIVE BUILD**

## Why

An online mesh must never wait for a portal action, but an offline mesh benefits
from the operating system opening a local explanation and a path to
`hello.mesh`. Portal state should describe internet availability rather than a
person's dismissal state.

## What

- Treat the router's current kernel default route as the local internet-availability signal.
- Report CAPPORT open and return exact OS success probes while that route exists.
- Report CAPPORT captive and serve an offline mesh page when no default route exists.
- Keep the legacy pass endpoint idempotent without allowing it to override network state.

## Impact

- Capabilities: MODIFIED `captive-portal`
- ADRs: none

## User journey & surfaces

A person joining an online mesh gets internet immediately without a sheet or
button. A person joining an offline mesh sees that internet is unavailable and
can open `hello.mesh` for local people and services. When routing recovers, the
next OS check succeeds automatically.

## Out of scope

- Blocking or filtering client traffic.
- Adding another active internet probe to `mjolnir-hello`.
- Changing gateway selection or default-route installation.
