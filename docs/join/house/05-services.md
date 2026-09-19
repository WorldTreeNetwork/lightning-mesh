---
id: house.services
title: Share something on the house mesh
description: How household services show up as .mesh names, and how to publish one from a router.
path: house
order: 5
audience: [person, operator, agent]
status: partial
time: 15 min
requires: [house.administer]
next_step: house.add-router
verified_against: 723b308 (2026-09-19)
---

# Share something on the house mesh

A "service" is anything with a TCP port you want people on the mesh to
open by name: a wiki, a camera page, a game, a printer admin UI. The
directory on hello.mesh is the index. DNS for `.mesh` is served by the
routers, so it works with the WAN unplugged.

> **Status: partial.** Operator publish from SSH is built. App cards
> inside hello.mesh are not deployed. Claiming a pretty name from the
> front desk is not deployed.

## What residents see

**Do:** open `http://hello.mesh` → **Services**.

**Expect:** links such as `https://walkie-talkie.mesh`. Tapping one opens
the app. A self-signed certificate warning is normal for `https://…mesh`.

**If not:** "Nothing has been shared" means nobody has published on this
mesh yet. Internet sites are unrelated; they need a WAN uplink.

Residents do not need an IdentiKey to open a published HTTP service.
Sign-in is only when an app asks, via
[Sign in to mesh apps](../person/04-sign-in.md).

## Publish from a router (operator)

The command must run **on the router** (`mjolnir-meshd` talks to
`127.0.0.1`). There is no `meshd` alias.

**Do:** SSH to the router whose LAN holds the thing you are sharing, then:

```sh
mjolnir-meshd publish photos --port 8080
```

**Expect:** `published photos.mesh  ip=10.42.x.1 port=8080`. Within a
minute it appears under Services on every hello.mesh.

**If the app is another machine on that LAN:**

```sh
mjolnir-meshd publish photos --ip 10.42.7.42 --port 8080 --mac aa:bb:cc:dd:ee:ff
```

**Expect:** a router-scoped name like `photos.ab12.mesh`. Share the full
name.

Full flags, key-owned names, and the HTTP APIs:
[Publish a service](../publish/01-publish-a-service.md).

## House rules of thumb

- Prefer a `.mesh` name over asking people to remember `10.42.x.y`.
- Do not bridge client LANs to "make Chromecast work everywhere." Publish
  instead, or keep the device on one router.
- A service that needs the public internet still needs a gateway. A
  service that is only local should keep working when the WAN is out —
  that is the point.

Next: [Add another router](06-add-router.md).
