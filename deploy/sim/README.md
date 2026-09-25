# Mesh sim lab

Laptop-local OpenWrt fleet on libvirt/QEMU. Not Filogic. Not the live
Cudy boxes. Capability `mesh-sim-lab` (`add-mesh-sim-lab`).

This directory is the lab. `deploy/openwrt/` remains metal. Do not add
sim guests to `fleet-nodes.conf`. Do not join Lightning Mesh from this
workstation to reach the lab (`mesh-errand.sh` is for metal).

## What this is

| Piece | Daily lab | Not |
|---|---|---|
| Host | this Linux x86_64 KVM box | Mac as vwifi-server |
| Guest machine | OpenWrt x86_64, libvirt **q35** | QEMU `-M virt` (that is `armsr` smoke only) |
| Daemon | `x86_64-unknown-linux-musl` meshd | the shipped aarch64 musl binary |
| Air | vwifi + `mac80211_hwsim` (AP + 802.11s) | mt76 / Cudy sysupgrade |
| Ethernet / NAT | virtio (ISP CPE + aftermarket NAT) | WAN hole-punch as the mesh path |
| Mgmt | `10.99.0.0/24` SSH + vwifi TCP | overlay / babel / iroh |

Operational shape copies `admin/scripts/windows-build/`: named domains,
`virsh start` / `shutdown`, SSH BatchMode, no from-scratch provision
every run.

## Evidence

A green sim roam **does not** close `mjolnir-mesh-5wc`, `wvg`, `wvg.1`,
or `sz9.1`. The harness may name those beads as the scenario it imitates.
Linux STA is not an iPhone. hwsim is not mt76.

## Isolation

The lab image plants a **positive** sim-guest marker
(`/etc/mjolnir/sim-guest`). The sim wireless/apply profile requires it
and finishes that check before any UCI/`wireless` write. No marker →
refuse, including Cudy, OpenWrt One, Banana Pi, and any other metal.

`backhaul_iface` is the 802.11s hwsim vif, never the mgmt NIC. Partition
vwifi and overlay/`10.254` must die while mgmt SSH still works. Overlay
that still works via mgmt alone is a failed run.

## Target fixture (later beads)

```
host: vwifi-server, libvirt
  mgmt     10.99.0.0/24
  isp-lan  192.168.1.0/24
  nat2-lan 192.168.50.0/24

guests: isp-cpe, house-nat, node-a, node-b, phone (STA-only)
```

## Rebuild artifacts (`sim.2`)

```bash
deploy/sim/build-meshd.sh                 # -> deploy/sim/mjolnir-meshd-x86_64
SKIP_WEB=1 deploy/sim/build-hello.sh      # optional; -> deploy/sim/mjolnir-hello-x86_64
deploy/sim/build-image.sh                 # -> deploy/sim/openwrt-x86-64-generic-ext4-combined-efi.img.gz
```

`OPENWRT_RELEASE` defaults to `25.12.5` (same series as the metal
fleet). Imagebuilder cache is `deploy/sim/imagebuilder/` (gitignored).
These paths are not consumed by `install-node.sh` / `update-fleet.sh`.

Domains are `mjolnir-mesh-sim.3`–`.6`. Until those land, `virsh list
--all` will not show sim guests.

## Start / stop (`sim.3`)

```bash
deploy/sim/start.sh     # nets + sim-node-a / sim-node-b (q35)
# wait for leases (script prints them):
virsh -c qemu:///system net-dhcp-leases lightning-sim-mgmt
ssh -o BatchMode=yes root@10.99.0.10
ssh -o BatchMode=yes root@10.99.0.11
deploy/sim/stop.sh
```

`VIRSH_URI` defaults to `qemu:///system`. Disks live in `deploy/sim/disks/`
(gitignored backing qcow2 clones).

## Wireless (`sim.4`)

TCP vwifi on mgmt (`10.99.0.1:8212`), not vsock. Guests already have
`kmod-mac80211-hwsim`; reload it with `radios=0` then `vwifi-client`.

```bash
# host: vwifi-server + vwifi-ctrl (built from https://github.com/Raizo62/vwifi)
# guests: musl vwifi-client (Alpine build) + libnl/libstdc++
deploy/sim/start-vwifi.sh
vwifi-ctrl ls
vwifi-ctrl set <id> 0 5000 0   # distance; hwsim RSSI may stay -10 dBm,
                               # inactive time / tx is the tell
```

Partition test: `killall vwifi-server` — SSH to 10.99.0.10 still works;
`10.254` ping fails; restore with `vwifi-server -l` + guest `vwifi-client`.

## Install (`sim.5`)

```bash
deploy/sim/build-meshd.sh              # meshd
# mjolnir-txn from mjolnir-apply:
#   same docker, cargo build -p mjolnir-apply --bin mjolnir-txn
deploy/sim/install-node.sh root@10.99.0.10
deploy/sim/install-node.sh root@10.99.0.11
```

Uses x86 musl artifacts and `deploy/sim/setup-wireless.sh` (refuses
unmarked metal). Fleet `install-node.sh` still defaults to aarch64.

## NAT household (`sim.6`)

```
sim-isp-cpe   10.99.0.12   WAN=libvirt default, LAN=192.168.1.1
sim-house-nat 10.99.0.13   WAN=192.168.1.x, LAN=192.168.50.1 (masq)
sim-node-a    10.99.0.10   WAN eth1=192.168.1.x  gateway=auto
sim-node-b    10.99.0.11   WAN eth2=192.168.50.x gateway=0
```

```bash
deploy/sim/start.sh
deploy/sim/configure-nat.sh
# overlay is 802.11s, not WAN:
ssh -o BatchMode=yes root@10.99.0.11 'ping -c1 10.254.69.196'
# inbound WAN to B fails (double NAT):
ssh -o BatchMode=yes root@10.99.0.10 'ping -c1 -W2 192.168.50.148'
```

isp/nat2 libvirt nets are L2-only; the CPE guests own `.1` + DHCP.

## Roam keep-IP (`sim.7`)

`deploy/sim/roam-keep-ip.sh` — Linux STA (`sim-phone` 10.99.0.14) hops
A→B. Must not `bd close` 5wc. 2026-09-24 run: assoc A→B, IPv4 kept
`10.42.69.106`, **RED** because proto 158 `/32` stayed on A as well as B.

Client AP needs a dedicated `hostapd` (`ssid LightningMesh` ch 6);
netifd/wpad leaves hwsim 2g AP at txpower 0. Split mgmt `eth0` off
`br-lan` so STA DHCP is `10.42.x` not `10.99`.

## aarch64 smoke (`sim.8`)

```bash
deploy/sim/smoke-armsr.sh
```

TCG `qemu-system-aarch64 -M virt` boots OpenWrt armsr/armv8 initramfs and
runs `deploy/openwrt/mjolnir-meshd-aarch64 --help` via virtio-9p. Not the
daily q35 lab.

## DNA-like STA (`sim.9`)

```bash
deploy/sim/dna-probe.sh
```

RFC 4436 INIT-REBOOT: change the associated AP MAC, `udhcpc -r $IP`,
record keep vs hole. Informs `sz9.1`; does **not** close it.

Do not boot `deploy/openwrt/vendor-firmware/*.bin` in QEMU.
