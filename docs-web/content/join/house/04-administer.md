---
id: house.administer
title: Administer the house
description: Rename the Wi-Fi, name routers, control internet sharing, reach SSH, and use Lightning Admin for what it can do today.
path: house
order: 4
audience: [operator, agent]
status: partial
time: 20 min
requires: [house.how-it-works]
next_step: house.services
verified_against: 2f6dc6b (2026-09-24)
---

# Administer the house

Day-to-day house control is a short list. Most of it is SSH to the
overlay. Lightning Admin today only discovers boxes and sets the **client
Wi-Fi name**.

> **Status: partial.** Overlay SSH, WPS WAN window, names, gateway
> auto/never, and Lightning Admin's network-name apply are built. A
> "Manage network" owner UI with roles is in progress, not on the box.

## Reach a router

Prefer the overlay. Ethernet at `192.168.1.1` is recovery.

| Where you are | How |
|---|---|
| On the mesh Wi-Fi or a LAN port | `ssh root@10.254.x.y` |
| On the network upstream of the mesh | Press **WPS** once on the box (15-minute WAN SSH from that LAN only), then SSH to its WAN address |
| Nothing else works | Ethernet to a LAN port, `ssh root@192.168.1.1`, then `ssh-keygen -R 192.168.1.1` if the host key changed |

You need an authorized SSH key. A WPS press does not pair Wi-Fi and does
not open the whole internet. Copy files with `scp -O` (Dropbear has no
SFTP).

A useful `~/.ssh/config` stanza:

```
Host 10.254.*
    User root
    StrictHostKeyChecking accept-new
```

Find `10.254.x.y` on hello.mesh under **Routers**, or from
`service mjolnir-meshd diag` on a box you can already reach.

## Lightning Admin (desktop)

**Do:** on a computer that can see the house LAN, run Lightning Admin
(`admin/` in the repo: `bun install` then `bun run tauri dev`, or a
Windows build from `admin/scripts/windows-build/`).

**Expect:** a list of discovered routers and a **Network name** field.
Apply writes the client SSID (`CLIENT_SSID`) on reachable nodes. An empty
name does not apply.

**If not:** you are not on a network that can see the boxes. Join the
mesh Wi-Fi or the upstream LAN after a WPS window. Guild membership
inside Admin is not built.

Renaming the SSID this way does **not** change the backhaul id
(`mjolnir-mesh`) and does not touch identity.

## Rename one router

**Do:**

```sh
ssh root@10.254.x.y
uci set mjolnir.meshd.name='kitchen'
uci commit mjolnir
service mjolnir-meshd restart
```

**Expect:** hello.mesh shows **kitchen** within a minute.

**If not:** `service mjolnir-meshd diag` should still be healthy. A restart
drops client Wi-Fi for a few seconds.

## Share internet, or refuse to

Default is `gateway=auto`: a working WAN is offered to the whole mesh.

**Do** this on a box plugged into a guest, metered, or captive network:

```sh
uci set mjolnir.meshd.gateway='never'
uci commit mjolnir
service mjolnir-meshd restart
```

**Expect:** that box still serves local clients; it does not become the
house's default route to the internet.

## Health and recovery

| Question | Command |
|---|---|
| Is this box healthy? | `service mjolnir-meshd diag` |
| Last staged apply | `cat /root/mjolnir-stage/result` and `apply.log` |

A dead flash is a new identity. The node secret never leaves the box. A
replacement is a new router: [Add another router](06-add-router.md).

Longer operator reference: [Operate your node](../node/04-operate.md).

Next: [Share something on the house mesh](05-services.md).
