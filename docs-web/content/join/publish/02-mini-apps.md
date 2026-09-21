---
id: publish.miniapps
title: Mini-apps (coming soon)
description: What will let a published service appear as an app card inside hello.mesh and receive the visitor's identity, and what exists today.
path: publish
order: 2
audience: [developer, agent]
status: coming-soon
time: reference
requires: [publish.service]
next_step: contribute
verified_against: 503cd17 (2026-09-13)
---

# Mini-apps (coming soon)

Mini-apps will let a service on the mesh show up as an **app card inside
hello.mesh**, and ask visitors to share their IdentiKey with one tap.

> **Status: coming soon. Nothing here is deployed on the mesh yet.**
> - Both designs are accepted after independent security review.
> - The shared contract code is merged but not on routers.
> - The Apps shelf and the identity bridge aren't built yet.
>
> Track progress in beads epic `mjolnir-mesh-ncy`.

## What's decided

| Piece | Design |
|---|---|
| Marking a service as an app | Add `app=v1` (and optionally `path=/entry`) to the service's TXT record |
| App details | The app serves `/.well-known/mesh-app.json` (name, description, icon, `embed: card` or `link`, height). **Routers** fetch it, so a visitor's browser never contacts an app before the visitor opens it |
| How it appears | A sandboxed iframe on the app's own origin, loaded only when tapped, with an open-in-new-tab control. Otherwise a plain link |
| What can be embedded | Only apps on a non-reserved `.mesh` **name**. Apps on raw IP addresses always open as links |
| Identity | The app asks hello.mesh over `postMessage`. hello.mesh shows consent outside the app's frame and returns the same signed assertion `/assert` issues today |

The specs are in
[`openspec/changes/add-mini-app-contract`](https://github.com/WorldTreeNetwork/lightning-mesh/tree/main/openspec/changes/add-mini-app-contract)
and
[`openspec/changes/add-embedded-assert`](https://github.com/WorldTreeNetwork/lightning-mesh/tree/main/openspec/changes/add-embedded-assert).

## What you can do today

1. Publish your service ([previous page](01-publish-a-service.md)) and add
   `--txt app=v1`. Nothing displays it yet, but the marker is already valid
   on the wire.
2. Serve a manifest your routers will be able to read:
   ```json
   {"v":1,"name":"Keyed","description":"Tasks for the house","icon":"/icon.png","embed":"card","height":480}
   ```
3. To sign people in now, use the full-page identity flow, `/assert`. See
   [Sign in to mesh apps](../person/04-sign-in.md) and the
   [assertion protocol](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/docs/network-coordination/identity-assertion.md).

Next: [Contribute](../contribute.md).
