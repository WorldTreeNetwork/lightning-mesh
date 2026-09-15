---
id: person.services
title: Find services
description: Discover and open things people have shared on the mesh, and what to expect from internet access.
path: person
order: 5
audience: [person, agent]
status: built
time: 2 min
requires: [person.hello]
next_step: person.leave
verified_against: 3b862d5 (2026-09-14)
---

# Find services

People and devices on the mesh can share services (a chat app, a file
drop, a printer) under a `.mesh` name that works from every router.

> **Status: built.** App cards inside hello.mesh are coming soon; today
> services open as links.

## Step 1: Open the Services list

**Do:** on `http://hello.mesh`, look at **Services** (open by default).

**Expect:** a count and a list. Web services are clickable links such as
`https://walkie-talkie.mesh`. Other entries show an address.

**If not:** "Nothing has been shared on the mesh yet." Nobody has published
anything on this mesh. To share something yourself, see
[Publish a service](../publish/01-publish-a-service.md).

## Step 2: Open a service

**Do:** tap a link.

**Expect:** the app opens in your browser.

**If you see a certificate warning:** apps on `https://…mesh` addresses use
certificates the mesh made itself, because `.mesh` isn't a public internet
name. Your browser can't vouch for them, so it warns.
- **Do:** check the address is the one you meant, then continue past the
  warning (for example "Show details → visit this website" on Safari, or
  "Advanced → Proceed" on Chrome).
- Only do this for `.mesh` addresses you reached from hello.mesh.

**If a name doesn't open:** services can go offline. An entry published by an
app disappears from the list within about 90 seconds after it stops
renewing. Try again later, or ask whoever runs it.

## Typing a name directly

Any `.mesh` name shown in Services also works in the address bar, on every
router, for example `http://walkie-talkie.mesh`. Type the `http://` or
`https://` so your browser doesn't search for it.

## Internet access

| Situation | What works |
|---|---|
| A router in the mesh has an internet uplink | Everything: mesh services **and** the internet |
| No router has internet | hello.mesh, `.mesh` services, People and your identity. Internet sites and apps that need them don't load |

Your phone may open a local mesh sheet when no internet route is available.
If websites don't load but `http://hello.mesh` does, there is no internet uplink
right now. That isn't your phone's fault.

You can't claim your own `.mesh` name from hello.mesh yet. Names are
published by apps and router operators.

Next: [Take your identity with you](06-leave.md).
