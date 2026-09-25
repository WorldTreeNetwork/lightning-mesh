# add-sim-libvirt-fixture

> **ACTIVE BUILD**

Bead `mjolnir-mesh-sim.3`. Human activated 2026-09-18 (`activate all`).

## Why

The lab contract copies `morphist-win11`: named domains, virsh start/shutdown,
SSH BatchMode. No domains exist yet.

## What

- libvirt nets: mgmt `10.99.0.0/24` (not 172.16/16 — this host already
  has Docker `172.16.0.0/22`), isp-lan `192.168.1.0/24`, nat2-lan `192.168.50.0/24`
- Named domains (incremental: node-a, node-b first) on q35, virtio NICs
- Start/stop scripts under `deploy/sim/` matching windows-build shape

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

`virsh start` + `ssh -o BatchMode=yes root@10.99.0.x`. No new UI because
the surfaces are libvirt and SSH, like `admin/scripts/windows-build/`.

## Out of scope

- vwifi (`sim.4`), install profile (`sim.5`), NAT VMs (`sim.6`), roam (`sim.7`)
