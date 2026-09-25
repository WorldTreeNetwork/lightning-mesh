# Design: add-mesh-sim-lab

Cross-cutting lab contract. Isolation-sensitive. Steer 2026-09-17.

## Pins

- **Virt OpenWrt, not Filogic.** Daily guests are x86_64 KVM on a
  PC/q35 machine (libvirt `pc-q35-*` / QEMU `-M q35`). QEMU `-M virt`
  is only the optional aarch64 `armsr` smoke guest (`sim.8`).
  Cudy sysupgrade, MT7981 DTB, SPI-NAND, DSA, mt76 caldata are out.
- **Daily ISA is the host.** This workstation is x86_64 + KVM. Guests
  match that. `x86_64-unknown-linux-musl` is a second meshd artifact
  beside the shipped aarch64 musl binary. aarch64 is smoke (`sim.8`).
- **Air is vwifi.** Host `vwifi-server`; guest `vwifi-client` +
  `kmod-mac80211-hwsim` with two radios (2.4 AP, 5 GHz 802.11s).
  Link strength is guest coordinates + distance loss (`vwifi-ctrl set`).
  Control frames ride mgmt TCP `172.16.0.0/16`.
- **Mgmt is not a dataplane.** `backhaul_iface` is the 802.11s hwsim
  vif (or the later NAT scenario's intended underlay), never the mgmt
  virtio NIC. Guest firewall/sysctl drops babel (6696), iroh, and
  overlay forwards on mgmt. A harness that still has SSH on mgmt but
  has partitioned vwifi MUST lose babel adjacency and peer `10.254`.
  Green overlay via mgmt alone is a failed run.
- **Ethernet is virtio.** Separate from air. ISP CPE NAT, aftermarket
  NAT, node WAN/LAN. Double-NAT still meshes on 802.11s.
- **`deploy/sim/` is the tree.** Not a subdir of `deploy/openwrt/`.
  Operational shape copies `admin/scripts/windows-build/`: named
  domains, `virsh start`/`shutdown`, SSH BatchMode, no from-scratch
  provision every run.
- **Sim profile is opt-in on a sim guest, not a metal deny-list.**
  The image plants a positive marker (for example `/etc/mjolnir/sim-guest`
  plus `ubus call system board` target `x86/64` or `armsr/*`). The
  sim wireless/apply path requires that marker and completes every
  check before the first UCI/wireless write. Absence of the marker
  is refusal — Cudy, OpenWrt One, Banana Pi, and any future metal
  included. Do not teach `setup-wireless.sh` to skip `band=2g`/`5g`
  on metal.
- **Sim is not metal proof.** Harness logs may cite `5wc` as the
  scenario they *imitate*. They must not `bd close` `5wc`, `wvg`,
  `wvg.1`, or `sz9.1`.
- **STA is Linux.** Same OpenWrt image, STA-only UCI. iPhone DNAv4
  stays `sz9.1` / `sim.9` (script informs, does not close).
- **No guest-roam living spec.** `wvg` deferred that until call
  evidence. This capability is the lab, not the roam dataplane
  (`roam.rs` already exists).

## Topology (target fixture)

```
host: vwifi-server, libvirt
  mgmt     172.16.0.0/16   SSH, vwifi TCP
  isp-lan  192.168.1.0/24  ISP CPE LAN
  nat2-lan 192.168.50.0/24 aftermarket LAN

guests:
  isp-cpe     NAT to host, DHCP isp-lan
  house-nat   WAN isp-lan, LAN nat2-lan, masquerade
  node-a      WAN isp-lan, 10.42.A/24, hwsim AP+mesh, gateway=auto
  node-b      WAN nat2-lan, 10.42.B/24, hwsim AP+mesh, no default export
  phone       STA only, DHCP on client SSID
```

Incremental land is allowed: mgmt + two nodes before ISP/NAT VMs.

## Why not the first instinct

Booting `cudy_ap3000outdoor-v1-squashfs-sysupgrade.bin` in QEMU fails
the machine model, not a missing flag. Netns spikes already cover
overlay without OpenWrt. This lab exists to run procd, UCI,
`mjolnir-apply`, `wpad-mesh`, and `iw station dump` — the seams
`roam_loop` and install actually parse.

## Slice order (later changes, after this advise)

1. `sim.2` x86 image + musl meshd
2. `sim.3` libvirt domains/nets (weave with 2)
3. `sim.4` vwifi air
4. `sim.5` install-node + hwsim profile + metal guard
5. `sim.6` NAT topology weave `sim.7` roam keep-IP harness
6. `sim.8` aarch64 smoke (parallel after this pin)
7. `sim.9` DNA-like STA script after the harness

## Residual

- hwsim ≠ mt76 (`oaq`, HE80 airtime, caldata)
- virtio switch ≠ DSA copper merging client `/24`s (`heal.rs`)
- Linux STA ≠ iPhone DNAv4 (`sz9.1`)
- vwifi distance loss ≠ field RSSI
- TCG aarch64 will be slow; do not use it for the hop loop
