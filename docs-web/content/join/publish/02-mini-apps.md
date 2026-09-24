---
id: publish.miniapps
title: Mini-apps
description: Mark a published service as an app card on hello.mesh, serve a manifest, and receive identity from the front desk.
path: publish
order: 2
audience: [developer, agent]
status: built
time: 15 min
requires: [publish.service]
next_step: contribute
verified_against: 2f6dc6b (2026-09-24)
---

# Mini-apps

A mini-app is a published web service that hello.mesh shows as a **card
on the Apps shelf**, not only as a line in Services. Visitors tap **Open
here** (embed) or **Open** (new tab). The app can ask hello.mesh who the
visitor is without ever seeing the private key.

> **Status: built** in current hello.mesh (`AppsPanel`, `GET /api/apps`,
> identity bridge). Older nodes have no Apps section — the marker is still
> valid on the wire. Trusted HTTPS names (no certificate warning) are
> **not** part of this page.

## What visitors see

**Do:** open `http://hello.mesh` as a person on the mesh.

**Expect:** **Apps · tap to open**, a count, then cards. Mini-apps do not
also appear under Services. Empty copy: "No apps on this mesh yet."

**If not:** see [Find services](../person/05-services.md) and
[Troubleshooting](../person/troubleshooting.md#i-dont-see-an-apps-section).

## Mark a service as an app

**Do:** when you [publish](01-publish-a-service.md), include:

```sh
mjolnir-meshd publish keyed --port 3000 --txt proto=http --txt app=v1 --txt path=/
```

`path` is optional (default `/`). It must start with `/` and stay on that
origin. Non-http/https protocols are never mini-apps, even with `app=v1`.

**Expect:** the directory record carries `app=v1`. After hello refreshes
(a few seconds), the name leaves Services and appears under Apps.

## Serve a manifest

hello.mesh does **not** fetch this from the visitor's browser. **The
router** GETs `/.well-known/mesh-app.json` on the service's own IP and
port (only `10.42.0.0/16` or `10.254.0.0/16`), then serves the result at
`GET /api/apps`.

**Do:** serve JSON like:

```json
{
  "v": 1,
  "name": "Keyed",
  "description": "Tasks for the house",
  "icon": "/icon.png",
  "embed": "card",
  "height": 480
}
```

| Field | Rule |
|---|---|
| `v` | Must be `1` |
| `name` | ≤ 40 chars |
| `description` | ≤ 140 chars |
| `icon` | Same-origin path; node inlines a small png/jpeg/webp/svg |
| `embed` | `card` or `link` (default `link`) |
| `height` | CSS px, clamped 120–640 |

**Expect:** the card shows name, description, icon. `embed: card` plus a
`.mesh` **name** (not a raw IP) gets **Open here**. Raw IPs always link
out.

**If not:** missing or failed manifests still list the mini-app, just
undecorated. Timeouts are 3 seconds; the node retries at most every 5
minutes.

## Identity from a card

**Do:** from the embedded card, ask hello.mesh over `postMessage`
(`mesh: mini-app/v1`, `type: identity.request`, a nonce, one MessagePort).

**Expect:** hello.mesh shows consent **outside** the iframe (same facts as
`/assert`). On approve, the token comes back only on that port. The card
may show "Identity shared with …".

**If not:** no identity yet — the visitor uses
[Create your identity](../person/03-identity.md). Link-out tiles use the
full-page [sign-in](../person/04-sign-in.md) flow instead.

App-supplied code never runs on the hello.mesh origin. The iframe is
sandboxed and loaded only after a tap.

Living spec:
[`openspec/specs/mesh-mini-apps/spec.md`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/specs/mesh-mini-apps/spec.md).

Next: [Contribute](../contribute.md).
