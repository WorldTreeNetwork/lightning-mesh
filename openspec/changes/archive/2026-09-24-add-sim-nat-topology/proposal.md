# add-sim-nat-topology

> **ACTIVE BUILD**

Bead `mjolnir-mesh-sim.6`. Human activated 2026-09-18 (`activate all`).

## Why

Household shape: one node on the ISP LAN, one behind an aftermarket NAT.
They still mesh on 802.11s; WAN hole-punch is not the path.

## What

- isp-cpe and house-nat domains
- node-a WAN on isp-lan (`gateway=auto`); node-b WAN on nat2-lan
- overlay `10.254` via 802.11s

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

No new UI because the surfaces are libvirt nets and meshd overlay.

## Out of scope

- Two-site iroh-over-WAN; roam harness (`sim.7`)
