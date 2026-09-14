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
verified_against: 503cd17 (2026-09-13)
---

# Connect to the Wi-Fi

Lightning Mesh is a network of ordinary routers that talk to each other
directly. You join it like any Wi-Fi network. No account and no password.

> **Status: built.**

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

**Expect:** you connect within a few seconds. A sign-in sheet may pop up
("You're on Lightning Mesh"). That's the next page, so leave it open.

Joining the Wi-Fi doesn't make you a member of anything or create an
account. It just gets you on the network.

## Step 3: Confirm you're on the mesh

**Do:** look at your connection details (on a phone: tap the network name;
on a laptop: `ipconfig getifaddr en0` on a Mac, `ip -4 addr` on Linux).

**Expect:** an address that starts with `10.42.`, for example `10.42.61.23`,
with a router address ending in `.1`, for example `10.42.61.1`. That router
address tells you which router you're connected to.

**If not:** you're on a different network. Check the name in step 1.

## Good to know

- **You land on whichever router is closest by radio.** Each router has its
  own address range, so your address can differ from a friend's in the same
  room.
- **Moving around:** walking to another router may briefly interrupt
  connections, and you may get a new address. Seamless roaming is being built
  but hasn't been field-tested, so don't count on long calls surviving a walk
  across the building.
- **Internet:** you get internet if any router in the mesh has a working
  uplink. Everything on the mesh itself (hello.mesh, `.mesh` names, people
  nearby) works with no internet at all.
- **Some phones hide `.mesh` names.** If your phone uses "Private DNS" or
  your browser uses "secure DNS", turn it off for this network. See
  [Troubleshooting](troubleshooting.md).

Next: [Say hello](02-hello-mesh.md).
