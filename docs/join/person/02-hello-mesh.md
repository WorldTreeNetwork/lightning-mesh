---
id: person.hello
title: Say hello
description: Handle the welcome sheet and open hello.mesh, the front desk every router serves.
path: person
order: 2
audience: [person, agent]
status: built
time: 2 min
requires: [person.connect]
next_step: person.identity
verified_against: 503cd17 (2026-09-13)
---

# Say hello

Every router runs a small front desk at **hello.mesh**. It shows who's
around, what you can open, and how the routers connect, and it's where you
make your identity.

> **Status: built.** The welcome sheet hasn't been field-tested on every
> phone model yet, so if it doesn't appear, go straight to step 2.

## Step 1: The welcome sheet

Right after you join, your phone or laptop may show a sheet titled
**"You're on Lightning Mesh"**. It has two buttons:

| Button | What it does |
|---|---|
| **Create your IdentiKey** | Opens hello.mesh inside the sheet |
| **Just the internet, please** | Says "You're all set", closes the sheet, and won't interrupt you again for about 12 hours |

Nothing is blocked either way. This isn't a login wall.

**Do:** tap **Just the internet, please**.

**Why this one:** the sheet is a stripped-down browser, and an identity made
inside it may not be kept. Make your identity in your normal browser
instead (step 2).

**Expect:** "You're all set". The sheet closes.

**If not:** close the sheet with your phone's Done or Cancel control.

The sheet may appear again after a router restart, if you move to another
router, or if your address changes. Tap the same button again.

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
