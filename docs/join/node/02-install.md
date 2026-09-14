---
id: node.install
title: Install Lightning Mesh on a router
description: Build the daemon, set fleet Wi-Fi settings, and install onto a fresh OpenWrt node with automatic rollback.
path: node
order: 2
audience: [operator, agent]
status: built
time: 20–40 min
requires: [node.hardware]
next_step: node.join
verified_against: 503cd17 (2026-09-13)
---

# Install Lightning Mesh on a router

You build two binaries on your computer, then one script stages everything
on the router and applies it with a safety net. If the router doesn't come
back healthy, it restores its previous state by itself.

> **Status: built.** Same scripts the maintainers use for the live fleet.

## What you need

| Thing | Why |
|---|---|
| Docker (or OrbStack) | Cross-compiles the aarch64 router binaries |
| [Bun](https://bun.sh/) | Only for the optional hello.mesh front desk |
| Root SSH to the router at `192.168.1.1` | From [the previous page](01-hardware-and-flash.md) |

## Step 1: Build the router daemon

**Do:**
```sh
deploy/openwrt/build.sh
```

**Expect:** `deploy/openwrt/mjolnir-meshd-aarch64` exists, a static binary of
about 9–10 MB. The first build takes a couple of minutes.

**If not:** make sure the Docker daemon is running. The daemon can't be
built natively on macOS; Docker is the supported path.

## Step 2: Build the hello.mesh front desk (recommended)

The front desk is the web page people see at `hello.mesh`. The mesh runs
without it, but new users won't have a front door.

**Do:**
```sh
deploy/openwrt/build-hello.sh
```

**Expect:** `deploy/openwrt/mjolnir-hello-aarch64` exists. The script builds
the web app, embeds it, then cross-compiles. That order matters: a stale
web build ships a stale page.

## Step 3: Set the fleet Wi-Fi settings

Every node in one mesh must share these values, or the mesh won't form and
phones won't roam.

**Do:**
```sh
cp deploy/openwrt/fleet-secrets/wireless.env.example deploy/openwrt/fleet-secrets/wireless.env
$EDITOR deploy/openwrt/fleet-secrets/wireless.env
```

| Variable | What it is | Live fleet value |
|---|---|---|
| `MESH_ID` | 802.11s backhaul mesh id (routers only; phones never join it) | `mjolnir-mesh` |
| `MESH_KEY` | Backhaul encryption. Empty means an open backhaul | empty (open) |
| `CLIENT_SSID` | Network name phones join | `Lightning Mesh` (factory default is `⚡`) |
| `CLIENT_ENC` / `CLIENT_KEY` | Client Wi-Fi security. `none`, or `psk2`/`sae-mixed` plus a key | `none` (open) |
| `COUNTRY` | Regulatory domain. **Required**; set yours | `US` |
| `FT_KEY` | Optional 802.11r fast-roaming secret (`openssl rand -hex 32`) | empty |

The backhaul runs on 5 GHz, channel 36, unless you change it.
`fleet-secrets/wireless.env` is gitignored. Don't commit it.

**Joining your own mesh to an existing one:** copy that mesh's
`MESH_ID`, `MESH_KEY`, band and channel exactly.

## Step 4: Review the SSH keys you're about to install

> **Read this before installing on a router you own.**
> The repo ships
> [`deploy/openwrt/files/etc/dropbear/authorized_keys`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/deploy/openwrt/files/etc/dropbear/authorized_keys),
> and the installer merges those public keys into the router's root SSH
> keys. Unless you edit that file first, the project maintainers get root
> SSH access to your router.

**Do:** replace the file's contents with your own public key(s), or delete
the lines you don't want.

**Expect:** the file contains only keys you trust with root on this router.

## Step 5: Install

**Do:**
```sh
deploy/openwrt/install-node.sh --wireless deploy/openwrt/fleet-secrets/wireless.env root@192.168.1.1
```

What happens:
1. **Stage.** Copies binaries, configs, `setup-wireless.sh` and the applier to
   `/root/mjolnir-stage`, and pre-fetches packages. A router with no internet
   gets them from `deploy/openwrt/pkg-cache/`.
2. **Apply, detached.** `mjolnir-apply` snapshots the current state, applies,
   then checks health. Your SSH session may drop when Wi-Fi restarts. That's
   expected; the script reconnects.
3. **Health gate.** On a router that's already meshed, the overlay address and
   a neighbour must come back within 120 s, or it rolls back. A brand-new
   router has no baseline, so the gate is skipped.

**Expect:** the script prints `>> OK: applied …` and then the "Next (fresh
node only)" hints.

**If not:** the router restored its previous config. Read the log:
```sh
ssh root@192.168.1.1 'cat /root/mjolnir-stage/apply.log'
```

The installer's closing hint mentions `backhaul_iface 'br-lan'` for a wired
bench. For a real 802.11s node, keep the template default, `br-mesh`.

## Step 6: Check the install

**Do:**
```sh
ssh root@192.168.1.1 'cat /root/mjolnir-stage/result; sha256sum /usr/bin/mjolnir-meshd'
sha256sum deploy/openwrt/mjolnir-meshd-aarch64
```

**Expect:** the result file starts with `OK`, and both hashes match.

The daemon isn't started yet on a fresh router. You'll give it peers first.

Next: [Join the mesh](03-join-the-mesh.md).
