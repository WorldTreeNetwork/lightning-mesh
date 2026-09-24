---
id: index
title: Join Lightning Mesh
description: Start here. Choose how you are joining a Lightning Mesh network and follow the steps.
path: index
order: 0
audience: [person, operator, developer, agent]
status: built
time: reference
requires: []
next_step: house.unbox
verified_against: 2f6dc6b (2026-09-24)
---

# Join Lightning Mesh

Lightning Mesh is a network of ordinary routers that talk to each other
directly, with no company in the middle. It keeps working when the internet
doesn't. Pick the path that matches what you actually have:

| I have… | Start here | Time |
|---|---|---|
| A **pre-flashed** Lightning Mesh router (Wi-Fi named `⚡` or `Lightning Mesh` after it boots) | [Unbox](house/01-unbox.md) | 10 min |
| A **stock** Cudy (or other) box that still shows the vendor UI | [Flash OpenWrt](node/01-hardware-and-flash.md) | 1–2 h |
| Only a phone or laptop, and someone else's mesh is already on the air | [Connect to the Wi-Fi](person/01-connect.md) | 10 min |
| Something to share (camera, wiki, game) on a mesh I can SSH to | [Publish a service](publish/01-publish-a-service.md) | 10 min |
| Code to contribute | [Contribute](contribute.md) | |

If you just powered a box and **no** Lightning Mesh / `⚡` network appears
after two minutes, it was not pre-flashed. Use the flash path, not Unbox.

## The path for a new house (pre-flashed box)

1. [Unbox](house/01-unbox.md): power, optional WAN, join the client Wi-Fi.
2. [First hour](house/02-first-hour.md): `http://hello.mesh`, optional identity.
3. [How it works](house/03-how-it-works.md): client Wi-Fi vs backhaul, addresses, roaming, directory.
4. [Administer](house/04-administer.md): names, internet sharing, SSH, Lightning Admin.
5. [Share a service](house/05-services.md): `.mesh` names and the Apps shelf.
6. [Add another router](house/06-add-router.md): second box — **SSH peer exchange is still required**.

## The path for a new person

1. [Connect to the Wi-Fi](person/01-connect.md): join the open network.
2. [Say hello](person/02-hello-mesh.md): open `http://hello.mesh`.
3. [Create your identity](person/03-identity.md): an IdentiKey you hold, and its recovery phrase.
4. [Sign in to mesh apps](person/04-sign-in.md): approve apps with your identity.
5. [Find services](person/05-services.md): Apps shelf and Services list.
6. [Take your identity with you](person/06-leave.md): restore it anywhere; nothing locks you in.

Stuck? [Troubleshooting](person/troubleshooting.md).

## What works today

Each page carries a status. It describes what's **in the software**, not a
promise that every box in a mixed fleet has been updated.

| Status | Meaning |
|---|---|
| **built** | In the current tree and meant to work on a current node |
| **partial** | Works, with a stated gap |
| **coming soon** | Designed or in progress, not usable yet |

| Built | Partial or not yet |
|---|---|
| Open Wi-Fi, internet when any router has WAN, offline hello.mesh | Seamless *sessions* when you walk between routers (DHCP roam exists; long calls can still drop) |
| IdentiKey with a 24-word recovery phrase | Deleting your identity from a mesh |
| Sign-in from another tab (`/assert`) **and** from an Apps card | Reviewing or revoking remembered app approvals (clear site data) |
| `.mesh` names, operator publish, key-owned name API | Claiming a pretty name from the hello.mesh UI |
| **Apps shelf** on hello.mesh (`app=v1` + manifest) | Trusted HTTPS names without a certificate warning (in progress) |
| Adding routers, automatic internet sharing | A membership gate; **automatic** peering (you still paste node ids) |

## Reading this guide as an agent

- Reading order and one-line summaries:
  [`llms.txt`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/docs-web/content/join/llms.txt).
- Every page starts with YAML frontmatter: `id`, `path`, `status`, `requires`,
  `next_step`, `verified_against`.
- Every step has **Do**, **Expect**, **If not**.
- Treat `status: coming-soon` as design, not as a working feature.

The human site is [https://lightning.worldtree.network/](https://lightning.worldtree.network/).
To rebuild it, run `cd docs-web && bun run build`. To publish, `docs-web/scripts/publish.sh`.
