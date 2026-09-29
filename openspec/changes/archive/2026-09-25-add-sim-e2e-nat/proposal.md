# add-sim-e2e-nat

> **ACTIVE BUILD**

Bead `mjolnir-mesh-0wgr.2`. Unblocked after `0wgr.1` folded. Intend: no extra human-gate.

## Why

Household NAT was shown live. It needs a gated harness so overlay-over-WAN does not silently become the path.

## What

- `deploy/sim/e2e/run.sh nat`
- A WAN 192.168.1.0/24, gateway=auto; B WAN 192.168.50.0/24, no default export
- overlay traceroute 1 hop; A cannot ping B WAN

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

CLI harness. No new UI.

## Out of scope

- Closing `dsd`. Two-site iroh-over-WAN.
