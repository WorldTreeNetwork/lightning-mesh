# add-mesh-sim-lab

> **ACTIVE BUILD**

Bead epic `mjolnir-mesh-sim`. Human activated 2026-09-17 (`activate` on the
current intend DAG). Steer 2026-09-17 recorded on `mjolnir-mesh-sim.1`
(recommended forks, decide-for-me). This change is the architecture pass.
It does not implement vwifi, images, or the roam harness.

## Why

Roam keep-IP (`sz9`, `5wc`, `wvg`) and household NAT already exist as
product problems. The dataplane is coded; the missing piece is a
repeatable OpenWrt fleet that is not the live Cudy boxes and not a
walking phone. QEMU/libvirt already runs Windows builds
(`morphist-win11`). That host pattern can run OpenWrt guests. It cannot
boot Filogic sysupgrade images. Without a named lab contract, a later
act will either emulate the wrong machine, leak a virt wireless profile
onto metal, or close `5wc` from a Linux STA.

## What

- Name the capability `mesh-sim-lab`.
- Daily lab is this Linux x86_64 KVM host. Guests are OpenWrt x86_64
  on PC/q35. The shipped aarch64 musl binary is a smoke guest only
  (`armsr`, QEMU `-M virt`).
- Air is vwifi + `mac80211_hwsim` (two radios: client AP + 802.11s).
  Ethernet NAT is virtio. Mgmt SSH and vwifi TCP ride a third net,
  never the simulated air.
- Layout is `deploy/sim/`, not under `deploy/openwrt/`, so fleet
  rollout scripts cannot pick the fixture up.
- A sim wireless/apply profile runs only on a guest with a positive
  sim-guest marker; any physical OpenWrt board is refused before a
  wireless write.
- A green sim roam does not close `5wc`, `wvg`, `wvg.1`, or `sz9.1`.
- First instrument (later change `add-sim-roam-keep-ip`): Linux STA hops
  A→B and records keep-IP, proto 158, LeaseBook, egress gap.
- NAT fixture (later change): one node on an ISP LAN, one behind an
  aftermarket NAT; they still mesh on 802.11s.

## Impact

- Capabilities: ADDED `mesh-sim-lab`
- ADRs: will amend `ARCHITECTURE.md` (lab is virt OpenWrt + vwifi, not
  Filogic; sim is not metal roam proof)

## User journey & surfaces

An agent or operator on this workstation starts named libvirt domains
instead of joining Lightning Mesh (which drops the chat uplink). They
SSH on the mgmt net, run `install-node.sh` against sim guests, move a
STA with `vwifi-ctrl` / `wpa_cli roam`, and read a harness log.

- Working: two mesh nodes plink, a STA keeps its IP across an AP hop,
  overlay `10.254` answers, harness prints gap + LeaseEntry.
- Empty: domains shut off; `virsh start` brings the fixture back.
- Failed: apply rolls back on the guest; metal fleet is untouched;
  harness exits non-zero and does not close roam beads.
- Off: lab stopped; live Cudy nodes are the only mesh.

No new UI because the surfaces are libvirt/`virsh`, SSH, `install-node.sh`,
`mjolnir-apply`, hello.mesh on the guest LAN gateway, and a CLI harness
log. Lightning Admin is not a sim console.

## Out of scope

- Booting `mediatek/filogic` / Cudy sysupgrade in QEMU
- Implementing vwifi, x86 image, libvirt XML (`mjolnir-mesh-sim.2`–`.4`)
- Sim install profile and NAT fixture (`sim.5`, `sim.6`)
- Roam keep-IP harness (`sim.7`) and DNA-like STA script (`sim.9`)
- aarch64 smoke guest (`sim.8`)
- Closing `5wc` / `wvg` / `sz9.1` from sim
- Shared-L2 island (`190`) as the sim roam path
- 802.11r on the OPEN client SSID
- Mac as vwifi-server host
- Call-survive (`wvg.1`) as the first instrument
- `add-guest-roam` living spec (deferred by `wvg` until call evidence)
