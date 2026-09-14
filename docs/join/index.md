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
next_step: person.connect
verified_against: 503cd17 (2026-09-13)
---

# Join Lightning Mesh

Lightning Mesh is a network made of ordinary routers that talk to each other
directly, with no company in the middle. It keeps working when the internet
doesn't. Pick how you're joining:

| I want to… | Start here | Time |
|---|---|---|
| Use the mesh from my phone or laptop | [Connect to the Wi-Fi](person/01-connect.md) | 10 min |
| Add my own router to a mesh | [Choose a router and flash OpenWrt](node/01-hardware-and-flash.md) | 1–2 h |
| Share an app or device on the mesh | [Publish a service](publish/01-publish-a-service.md) | 10 min |
| Help build it | [Contribute](contribute.md) | |

## The path for a new person

1. [Connect to the Wi-Fi](person/01-connect.md): join the open network.
2. [Say hello](person/02-hello-mesh.md): the welcome sheet and `http://hello.mesh`.
3. [Create your identity](person/03-identity.md): an IdentiKey you hold, and its recovery phrase.
4. [Sign in to mesh apps](person/04-sign-in.md): approve apps with your identity.
5. [Find services](person/05-services.md): open what people have shared.
6. [Take your identity with you](person/06-leave.md): restore it anywhere; nothing locks you in.

Stuck? [Troubleshooting](person/troubleshooting.md).

## What works today

Each page carries a status. It describes what's **deployed**, not what's
planned.

| Status | Meaning |
|---|---|
| **built** | Works on the live mesh today |
| **partial** | Works, with a stated gap |
| **coming soon** | Designed or in progress, not usable yet |

| Built | Not yet |
|---|---|
| Open Wi-Fi, welcome sheet, hello.mesh front desk | Seamless roaming between routers (being field-tested) |
| IdentiKey with a 24-word recovery phrase | Deleting your identity from a mesh |
| Signing in to mesh apps | Claiming your own `.mesh` name from hello.mesh |
| `.mesh` services, published by operators and apps | App cards inside hello.mesh ([mini-apps](publish/02-mini-apps.md)) |
| Adding routers, automatic internet sharing | A membership gate for who may add routers |

## Reading this guide as an agent

- Reading order and one-line summaries: [`llms.txt`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/docs/join/llms.txt).
- Every page starts with YAML frontmatter:
  - `id`, `path`, `status`
  - `requires` and `next_step`, which link pages by `id`
  - `verified_against`, the commit the steps were checked against
- Every step has the same three parts:
  - **Do**: the action or command
  - **Expect**: the observable result
  - **If not**: the diagnosis
- Treat `status: coming-soon` pages as design, not as working features.

To build this guide as a website, run `bunx vitepress build docs/join`.
