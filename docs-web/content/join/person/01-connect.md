---
id: person.connect
title: Connect to the Wi-Fi
description: Join a Lightning Mesh network from a phone or laptop and confirm you are on it.
path: person
order: 1
audience: [person, agent]
status: built
time: 1 min
requires: []
next_step: person.hello
verified_against: 2f6dc6b (2026-09-24)
---

# Connect to the Wi-Fi

Lightning Mesh is a network of ordinary routers that talk to each other
directly. You join it like any Wi-Fi network. No account and no password.

> **Status: built.**

## Before you start (one radio)

A phone or laptop has **one** Wi-Fi radio. Joining this network drops the
Wi-Fi you were using for the internet (home, café, "Pirate Radio", …).
That is expected. Do not change your default route on a machine you still
need as a working uplink unless you mean to.

## Step 1: Find the network

**Do:** open your Wi-Fi settings and look for the network your host named.

| Setup | Network name |
|---|---|
| New router, factory settings | `⚡` (a single lightning-bolt character) |
| The project's home fleet | `Lightning Mesh` |
| Someone else's mesh | Whatever they called it. Ask them |

**Expect:** the network shows as **open** (no lock icon).

**If not:** the host added a password. Ask them for it. Networks named
`mjolnir-mesh` are the routers' own link to each other; phones can't join
those.

## Step 2: Join

**Do:** tap the network name.

**Expect:** you connect within a few seconds. If the mesh has no internet
right now, a sheet may explain that the *local* mesh is still available.
That is [the next page](02-hello-mesh.md), not a login.

Joining the Wi-Fi doesn't make you a member of anything or create an
account. It just gets you on the network.

## Step 3: Confirm you're on the mesh

**Do:** look at your connection details (on a phone: tap the network name;
on a laptop: `ipconfig getifaddr en0` on a Mac, `ip -4 addr` on Linux).

**Expect:** an address that starts with `10.42.`, for example `10.42.61.23`,
with a router address ending in `.1`, for example `10.42.61.1`. That router
address tells you which router you're connected to.

**If not:** you're on a different network. Check the name in step 1.

## Walking around (roaming)

You land on whichever client radio is strongest. Each router owns its own
`10.42.x.0/24`, so two people in the same room can have different prefixes.

| What the software does | What you should expect |
|---|---|
| Same SSID on every box | You stay on "the house Wi-Fi" as you walk |
| Household DHCP: first MAC keeps its IP, every AP can OFFER it | Often you keep `10.42.a.b` when you change radios |
| Guest path: if you kept another node's IP, the visited node installs a host route | Built; **not** proven on every phone |
| Live TCP/UDP sessions (a call, a download) | May still drop or stall. Reconnect the app |

Do not plan a long call on a walk across the building yet.

## Good to know

- **Internet:** you get internet if **any** router in the mesh has a working
  uplink and is allowed to share it (`gateway=auto`). hello.mesh and `.mesh`
  names work with no internet at all.
- **Some phones hide `.mesh` names.** If your phone uses "Private DNS" or
  your browser uses "secure DNS", turn it off for this network. See
  [Troubleshooting](troubleshooting.md).

Next: [Say hello](02-hello-mesh.md).
