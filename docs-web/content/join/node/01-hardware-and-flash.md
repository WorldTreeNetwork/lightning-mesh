---
id: node.hardware
title: Choose a router and flash OpenWrt
description: Pick hardware the mesh supports and get a stock box onto official OpenWrt.
path: node
order: 1
audience: [operator, agent]
status: built
time: 30–60 min per box
requires: []
next_step: node.install
verified_against: 2f6dc6b (2026-09-24)
---

# Choose a router and flash OpenWrt

A Lightning Mesh node is a normal OpenWrt router running one extra daemon,
`mjolnir-meshd`. This page gets you from a **stock** boxed router to plain
OpenWrt that you can reach over ethernet.

If the box already broadcasts `⚡` or `Lightning Mesh` after a minute on
power, it is pre-flashed — go to [Unbox](../house/01-unbox.md) instead.

> **Status: built.** This is how every node in the live fleet was made.

## What you need

| Thing | Why |
|---|---|
| A supported router (below) | The daemon is built for aarch64 MediaTek boards |
| An ethernet cable and a computer | First contact is over the router's LAN port |
| [Git LFS](https://git-lfs.com/) | The Cudy-signed intermediate images are stored with LFS |
| A clone of the repo | `git clone https://github.com/WorldTreeNetwork/lightning-mesh` |

## Supported hardware

The build produces a static `aarch64-unknown-linux-musl` binary. The radios
must use the **mt76** driver (MediaTek MT7981 or MT7986). Every node in the
live fleet is a Cudy MT7981 box:

| Model | OpenWrt board id | In the fleet |
|---|---|---|
| Cudy WR3000S v1 | `cudy_wr3000s-v1` | yes |
| Cudy M3000 v1/v2 | `cudy_m3000-v1` / `-v2` | yes (two) |
| Cudy TR3000 v1 | `cudy_tr3000-v1` | yes |
| Cudy AP3000 Outdoor v1 | `cudy_ap3000outdoor-v1` | yes |

Also reasonable, per the hardware research
([synthesis](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/docs/research/openwrt-mesh-hardware/synthesis.md)):
Banana Pi BPI-R3 (MT7986) and OpenWrt One (MT7981). Avoid Qualcomm
ath11k/ath12k boards for mesh.

**One mt76 limitation to know:** an 802.11s mesh point and a client access
point on the *same* radio break the mesh join. The installer turns the
co-located AP off. The supported fix is a USB Wi-Fi dongle for clients.
Only the Ralink RT5370 (`148f:5370`) is on the supported list.

MikroTik RouterOS containers were tried and **retired**. Don't use that path.

## Step 1: Match the board label

**Do:** read the model and version from the label on the router itself, not
from the box or the listing.

**Expect:** an exact model and hardware version (for example "AP3000 Outdoor
V1.0").

**If not:** stop. A wrong intermediate image bricks Cudy boxes. Indoor,
Outdoor and Wall AP3000s all need different images.

## Step 2: Remove Cudy's signature check (Cudy boxes only)

Stock Cudy firmware only accepts signed images.

**Do:**
1. Fetch the intermediates: `git lfs pull` in your clone.
2. Open the stock Cudy web UI. An indoor AP3000 wants stock firmware 2.4.7
   first.
3. Upload the matching `*.bin` from
   [`deploy/openwrt/vendor-firmware/`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/deploy/openwrt/vendor-firmware/README.md):

   | Directory | Board |
   |---|---|
   | `ap3000-v1/` | AP3000 / AP3000 V1.1 (indoor) |
   | `ap3000-outdoor-v1/` | AP3000 Outdoor v1 |

   For any other Cudy model, use the matching zip from
   [Cudy's OpenWrt download page](https://www.cudy.com/blogs/faq/openwrt-software-download).
   Never use either of the above on an AP3000 Wall.

**Expect:** the box reboots into a minimal OpenWrt at `192.168.1.1`, handing
out DHCP on its LAN ports.

**If not:** re-check the label against the table. Don't try another image
"to see if it works".

## Step 3: Install official OpenWrt

**Do:**
1. Isolate the box from any house network that already uses `192.168.1.1`.
2. Get the **sysupgrade** image for your board id from the
   [OpenWrt firmware selector](https://firmware-selector.openwrt.org/). Use
   OpenWrt 25.12.5 or newer. Units with the newer F50L1G41LC flash chip
   (serial-number week 2543 or later) need at least 24.10.5.
3. Flash it from the minimal web UI, or with `sysupgrade`.
4. Set a root password and add your SSH public key.

**Expect:** `ssh root@192.168.1.1` works from your computer.

**If not:** clear a stale host key with `ssh-keygen -R 192.168.1.1`, because
every fresh OpenWrt box uses the same address. Stock Cudy firmware used
`192.168.10.254` for management, in case the flash didn't take.

## Step 4: Protect the node identity across future upgrades

The node's mesh identity will live in `/etc/mjolnir/`. A sysupgrade that
doesn't keep it gives the router a new identity.

**Do:**
```sh
ssh root@192.168.1.1 "grep -qx /etc/mjolnir/ /etc/sysupgrade.conf || echo /etc/mjolnir/ >> /etc/sysupgrade.conf"
```

**Expect:** `/etc/mjolnir/` is listed in `/etc/sysupgrade.conf`.

Next: [Install Lightning Mesh on the router](02-install.md).
