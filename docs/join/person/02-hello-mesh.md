---
id: person.hello
title: Say hello
description: Connect without a portal gate and open hello.mesh, the front desk every router serves.
path: person
order: 2
audience: [person, agent]
status: built
time: 2 min
requires: [person.connect]
next_step: person.identity
verified_against: a402557 (2026-09-14)
---

# Say hello

Every router runs a small front desk at **hello.mesh**. It shows who's
around, what you can open, and how the routers connect, and it's where you
make your identity.

> **Status: built.** Connectivity checks report an open connection immediately.
> Opening hello.mesh is voluntary and does not control internet access.

## Step 1: Confirm the connection

Right after you join, your phone or laptop should mark the Wi-Fi as connected.
There is no welcome-sheet button, login, or account step required for internet
access.

**Do:** wait for the Wi-Fi indicator to show connected.

**Expect:** normal internet access when any mesh router has an uplink. Local
`.mesh` services still work when the mesh has no internet uplink.

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
