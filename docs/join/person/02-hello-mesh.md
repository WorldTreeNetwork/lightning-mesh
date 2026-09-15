---
id: person.hello
title: Say hello
description: Connect immediately when online, or use the offline portal to reach the local hello.mesh front desk.
path: person
order: 2
audience: [person, agent]
status: built
time: 2 min
requires: [person.connect]
next_step: person.identity
verified_against: 3b862d5 (2026-09-14)
---

# Say hello

Every router runs a small front desk at **hello.mesh**. It shows who's
around, what you can open, and how the routers connect, and it's where you
make your identity.

> **Status: built.** An online mesh reports an open connection immediately. An
> offline mesh opens a local explanation and hello.mesh link. No button controls
> internet access.

## Step 1: Confirm the connection

Right after you join, your phone or laptop checks whether the mesh currently has
an internet route. With one, it marks the Wi-Fi connected without a sheet. With
no route, it may open a sheet explaining that the local mesh is still available.

**Do:** wait for the Wi-Fi indicator to show connected.

**Expect:** normal internet access without a button when any mesh router has an
uplink. With no uplink, use **Open hello.mesh** in the sheet for local people and
services. The sheet clears automatically after routing returns and the device
checks again.

**If not:** forget the network and rejoin it, then see
[Troubleshooting](troubleshooting.md).

## Step 2: Open hello.mesh

**Do:** in your normal browser (Safari, Chrome, Firefox…), type exactly:

```
http://hello.mesh
```

Type the `http://`. There's no `https` version, and some browsers treat a
bare `hello.mesh` as a search.

**Expect:** a page that says **"You're connected to `<router name>`"** with
the subtitle "A community mesh running on the routers around you. No
internet required."

**If not:**
- Your browser searched instead: type the full `http://hello.mesh`.
- "Server not found": turn off Private DNS or secure DNS
  ([Troubleshooting](troubleshooting.md)).
- Still nothing: open `http://10.42.x.1`, using the router address from
  [Connect to the Wi-Fi](01-connect.md), step 3. Come back to
  `http://hello.mesh` for step 3 of this guide, though. Identities made at
  the numeric address are kept separately.

## Step 3: A quick tour

From top to bottom:

| Section | What it shows |
|---|---|
| **You're connected to…** | Your router, and a button to set up your identity |
| **People** | Everyone who has introduced themselves on this network: a name, a short key, and when they were last seen ("here now", "seen 5m ago"). No accounts |
| **Services** | Things you can open on this mesh, as links |
| **Routers** | The routers that make up the mesh, and a map of which ones hear each other over the air |

The page updates itself every few seconds.

Next: [Create your identity](03-identity.md).
