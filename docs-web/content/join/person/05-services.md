---
id: person.services
title: Find services
description: Discover and open things people have shared on the mesh — Apps cards and the Services list.
path: person
order: 5
audience: [person, agent]
status: built
time: 2 min
requires: [person.hello]
next_step: person.leave
verified_against: 2f6dc6b (2026-09-24)
---

# Find services

People and devices on the mesh can share things under a `.mesh` name that
works from every router. hello.mesh splits them:

| Panel | What it is |
|---|---|
| **Apps** | A published web service marked `app=v1`. Cards, optional embed, optional identity |
| **Services** | Everything else (printers, unmarked web apps, other protocols) as links |

You do not need an IdentiKey to *open* a public HTTP service. You need one
only when the app asks you to sign in.

> **Status: built** on current hello.mesh. Older nodes have Services only.

## Step 1: Look at Apps, then Services

**Do:** on `http://hello.mesh`, read **Apps** (open by default; "tap to
open"), then **Services**.

**Expect:**
- Apps: a count, a name, a host like `keyed.mesh`, maybe a description and
  icon. Buttons **Open** (new tab) and, when the mesh allows embed,
  **Open here**.
- Services: clickable `http://` / `https://` links, or a bare address for
  non-web protocols.
- Empty Apps: "No apps on this mesh yet."
- Empty Services: "Nothing has been shared on the mesh yet."

**If not:** nobody has published on this mesh. To share something yourself,
you need operator access — see
[Publish a service](../publish/01-publish-a-service.md). There is no
"claim this name" button on the front desk yet.

## Step 2: Open a service or app

**Do:** tap **Open** or a Services link. For a card, **Open here** loads it
on the front desk; **Close** tears the frame down. Only one card is open at
a time.

**Expect:** the app opens.

**If you see a certificate warning:** apps on `https://…mesh` addresses use
certificates the mesh made itself, because `.mesh` isn't a public internet
name. Your browser can't vouch for them, so it warns.
- **Do:** check the address is the one you meant, then continue past the
  warning (for example "Show details → visit this website" on Safari, or
  "Advanced → Proceed" on Chrome).
- Only do this for `.mesh` addresses you reached from hello.mesh.
- Camera, microphone, and "install this app" often stay blocked until
  trusted HTTPS names ship. That is a browser rule, not a broken mesh.

**If a name doesn't open:** services can go offline. An entry published by
an app disappears from the list within about 90 seconds after it stops
renewing. Try again later, or ask whoever runs it.

## Typing a name directly

Any `.mesh` name shown in Apps or Services also works in the address bar,
on every router, for example `http://walkie-talkie.mesh`. Type the
`http://` or `https://` so your browser doesn't search for it.

## Internet access

| Situation | What works |
|---|---|
| A router in the mesh has an internet uplink | Everything: mesh services **and** the internet |
| No router has internet | hello.mesh, `.mesh` services, People and your identity. Internet sites don't load |

Your phone may open a local mesh sheet when no internet route is available.
If websites don't load but `http://hello.mesh` does, there is no internet
uplink right now. That isn't your phone's fault.

Next: [Take your identity with you](06-leave.md).
