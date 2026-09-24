---
id: node.operate
title: Operate your node
description: Reach, update, rename, diagnose and recover a mesh router without walking a cable to it.
path: node
order: 4
audience: [operator, agent]
status: built
time: reference
requires: [node.join]
next_step: publish.service
verified_against: 2f6dc6b (2026-09-24)
---

# Operate your node

The overlay is the management plane. Once a router has joined, you reach
and update it over the mesh. Ethernet at `192.168.1.1` is only for recovery.

> **Status: built.** Remote OTA over the internet with signed payloads is
> planned, not built.

## Reach a router

| Where you are | How |
|---|---|
| On the mesh (its Wi-Fi or a LAN port) | `ssh root@10.254.x.y`, routed directly with no jump host |
| On the network upstream of the mesh | Add `ProxyJump root@192.168.1.1` through one wired router |
| On someone else's network the router is plugged into | Press the router's **WPS button** (below) |
| Nothing works | Ethernet to its LAN port, `ssh root@192.168.1.1` |

SSH on a router's WAN side is firewalled by default.

A matching `~/.ssh/config` stanza for on-mesh use:
```
Host 10.254.*
    User root
    StrictHostKeyChecking accept-new
```

Copy files to OpenWrt with `scp -O`. Dropbear has no SFTP.

## Open SSH from the upstream network with the WPS button

**Do:** press the router's WPS button once.

**Expect:**
- The WPS LED blinks.
- SSH (TCP 22) is accepted on the WAN side, but only from the address ranges
  on that WAN interface right now, never from the whole internet.
- It closes by itself after 15 minutes (`mjolnir.wan_admin.timeout`), on a
  second press, a reboot, or a firewall reload.

You still need an authorized SSH key. The press doesn't do Wi-Fi WPS pairing.

## Update

**One router:** re-run the installer. Config is kept, a running daemon
restarts onto the new build, and a failed health check rolls it back.
```sh
deploy/openwrt/install-node.sh root@10.254.x.y
```

**A fleet:**
```sh
deploy/openwrt/update-fleet.sh                 # every node in fleet-nodes.conf, one at a time
deploy/openwrt/update-fleet.sh --wireless deploy/openwrt/fleet-secrets/wireless.env
deploy/openwrt/validate-fleet.sh               # read-only sweep afterwards
```
It halts on the first failure (that router has already rolled itself back),
and skips unreachable routers. Unchanged binaries aren't restarted.

## Rename routers and the network

| Change | How |
|---|---|
| One router's display name | `uci set mjolnir.meshd.name='attic'; uci commit mjolnir; service mjolnir-meshd restart` |
| All router names from the inventory | `deploy/openwrt/seed-names.sh` |
| The Wi-Fi name for the whole fleet | Lightning Admin desktop app, or `deploy/openwrt/apply-network-name.sh` |

Lightning Admin (`admin/`) is a desktop app that discovers routers and sets
the fleet's network name. Guild membership in it is planned (bead `st1.5`).

## Internet sharing

`option gateway 'auto'` (the default) makes any router with a working WAN
share internet with the whole mesh, and stop when the uplink drops. Set
`never` on a router plugged into an untrusted, metered or captive network:
```sh
uci set mjolnir.meshd.gateway='never'; uci commit mjolnir; service mjolnir-meshd restart
```

## Diagnose

| Question | Command |
|---|---|
| Is everything up? | `service mjolnir-meshd diag` (read-only, safe on live nodes) |
| Same build everywhere? | `logread -e "mjolnir-meshd starting"` on each router |
| Did the last apply work? | `cat /root/mjolnir-stage/result` and `apply.log` |

## Recover

- An experimental flag hung the daemon, for example `lan_tunnels=1` (keep
  it `0`):
  ```sh
  ssh root@<node> 'uci set mjolnir.meshd.lan_tunnels=0; uci commit mjolnir; service mjolnir-meshd restart'
  ```
- A bad build passed the health gate: re-run `install-node.sh` with a
  known-good build.
- Unreachable over the mesh: ethernet to `192.168.1.1`. Clear the host key
  first with `ssh-keygen -R 192.168.1.1`.

A dead router means a new identity. Its secret never leaves the box, by
design. Add the replacement as a new node.

Next: [Publish a service on the mesh](../publish/01-publish-a-service.md).
