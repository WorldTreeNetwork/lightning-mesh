# add-sim-x86-artifacts

> **ACTIVE BUILD**

Bead `mjolnir-mesh-sim.2`. Parent epic `mjolnir-mesh-sim`. Living
capability `mesh-sim-lab` (folded `add-mesh-sim-lab` 2026-09-17).
Human activated 2026-09-18 (`activate all` on the sim DAG).

## Why

The lab contract says daily guests are OpenWrt x86_64 on q35 running
`x86_64-unknown-linux-musl` meshd. The repo only ships
`aarch64-unknown-linux-musl` via `deploy/openwrt/build.sh`. Without
an x86 image (babeld, kmod-tun, wpad-mesh, hwsim, dropbear, sim-guest
marker) and a matching meshd, later libvirt/vwifi nodes have nothing
to boot.

## What

- Sibling of `deploy/openwrt/build.sh` produces
  `x86_64-unknown-linux-musl` `mjolnir-meshd` (and hello if present)
  into `deploy/sim/` (not the aarch64 fleet artifact names).
- OpenWrt x86_64 image (imagebuilder or documented equivalent) with
  babeld, kmod-tun, wpad-mesh-mbedtls, kmod-mac80211-hwsim, dropbear,
  and the positive sim-guest marker `/etc/mjolnir/sim-guest`.
- Rebuild command documented in `deploy/sim/README.md`.

## Impact

- Capabilities: MODIFIED `mesh-sim-lab` (guest artifact existence)
- ADRs: none (shape already pinned)

## User journey & surfaces

An agent on this workstation runs the documented rebuild, then boots
a q35 guest from the image (libvirt is `sim.3`).

- Working: image file + x86 musl meshd exist; guest board target is
  x86/64; marker file is in the image.
- Empty: artifacts missing; rebuild is the next command.
- Failed: cross-build exits non-zero; metal `mjolnir-meshd-aarch64`
  is untouched.
- Off: no guests running; chat uplink unchanged.

No new UI because the surfaces are `deploy/sim/` build scripts and
`deploy/sim/README.md`.

## Out of scope

- libvirt domains/nets (`mjolnir-mesh-sim.3`)
- vwifi (`sim.4`)
- install-node sim profile (`sim.5`)
- NAT / roam harness (`sim.6`, `sim.7`)
- aarch64 smoke (`sim.8`)
- Closing `5wc` / `wvg` / `sz9.1`
