# steer add-mesh-sim-lab

**When.** 2026-09-17
**Depth.** standard (decide-for-me on intend recommended forks)

## Decided

- Host: this Linux x86_64 KVM workstation (user | prior conversation)
  Why: QEMU 11.1 + `/dev/kvm` + `morphist-win11` already here; vwifi-server is a Linux host tool.
- Daily guests: OpenWrt x86_64 + `x86_64-unknown-linux-musl` meshd (user | prior conversation)
  Why: KVM speed class of win11. aarch64 TCG is not the roam loop.
- aarch64: smoke only (`sim.8`, TCG armsr here or later Mac HVF) (user | prior conversation)
  Why: same ISA as the shipped binary, not the daily fixture.
- Air: vwifi + mac80211_hwsim, two radios, distance loss via `vwifi-ctrl` (user)
  Why: Freifunk/LibreMesh GSoC 2025 already did OpenWrt AP–STA and mesh this way.
- vwifi transport: TCP on mgmt `172.16.0.0/16` (decide-for-me)
  Why: documented OpenWrt path; vsock is optional later, not a blocker.
- Tree: `deploy/sim/` at repo root, not under `deploy/openwrt/` (decide-for-me)
  Why: `update-fleet.sh` walks `fleet-nodes.conf`; a sibling tree cannot be an accidental node.
- Evidence: sim green does not close `5wc`, `wvg`, `wvg.1`, or `sz9.1` (decide-for-me)
  Why: Linux STA is not an iPhone; hwsim is not mt76.
- Phone VM: OpenWrt guest, STA-only UCI, same image as nodes (decide-for-me)
  Why: one image; `wpa_cli roam` / vwifi move is the hop.
- NAT: ISP CPE VM + aftermarket NAT VM; node-a on ISP LAN, node-b double-NAT (user)
  Why: household shape; mesh path is 802.11s, not WAN hole-punch.
- Isolation: sim wireless/apply profile refused on Cudy/Filogic board-id (decide-for-me)
  Why: `setup-wireless.sh` must not learn to skip radios on metal.

## Skipped

- none — leftover forks auto-logged below

## Auto

- [AUTO] Do not boot vendor-firmware Filogic sysupgrade.bin in QEMU.
- [AUTO] Mgmt net is not bridged onto simulated wifi.
- [AUTO] First instrument is roam keep-IP (`sim.7`), not call-survive.
- [AUTO] Shared-L2 (`190`) is not the sim roam path.
- [AUTO] This architecture change does not implement later beads.

## Feeds change

Daily lab is KVM x86 OpenWrt on this host with vwifi air and virtio NAT.
Capability `mesh-sim-lab` lives in `deploy/sim/`. Later nodes build image,
libvirt, vwifi, install profile, NAT, and the roam harness as separate
changes. Sim is a harness, not a substitute for metal roam or iPhone DNA.
