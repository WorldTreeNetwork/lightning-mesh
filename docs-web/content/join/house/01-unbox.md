---
id: house.unbox
title: Unbox a pre-flashed router
description: Plug in a Lightning Mesh router that already has the software on it, and get it on the air.
path: house
order: 1
audience: [person, operator, agent]
status: built
time: 10 min
requires: []
next_step: house.first-hour
verified_against: 723b308 (2026-09-19)
---

# Unbox a pre-flashed router

A pre-flashed Lightning Mesh router is already running OpenWrt and
`mjolnir-meshd`. You do not flash it. You plug it in, join its Wi-Fi, and
open the front desk.

> **Status: built.** Same software as the live fleet. Factory client Wi-Fi
> is open and named `⚡` unless the person who flashed the box renamed it.

## What you should have

| Thing | Why |
|---|---|
| The router | Indoor units have LAN ports. Outdoor units have a PoE injector or adapter |
| Power | The brick or PoE injector that came with the box |
| Ethernet (optional) | Plug WAN into your existing modem or house router if you want the mesh to share internet |
| A phone or laptop | To join the client Wi-Fi |

You do not need a GitHub account, a password, or Lightning Admin for the
first hour.

## Step 1: Place it

**Do:** put the router where people actually sit, not in a closet if you
can help it. Keep it away from microwave ovens and thick metal.

**Expect:** after power-on, LEDs come up within about a minute. The unit
is ready when the radio LED is steady or slowly blinking, not racing.

**If not:** try another outlet. If nothing lights, the brick or PoE injector
is the first suspect, not the software.

## Step 2: Power, then optional internet

**Do:**
1. Connect power.
2. If you want internet on the mesh, plug an ethernet cable from your
   **existing modem or house router** into the Lightning Mesh box's **WAN**
   port (often labelled WAN, or the port set apart from the others). Do
   **not** plug WAN into a LAN port of the same Lightning Mesh box.
3. Leave the LAN ports free for a computer or a switch if you need a
   wired device on the mesh.

**Expect:** the box boots by itself. You never log into a vendor web UI
for this step.

**If not:** a box with no WAN cable still serves local mesh (hello.mesh,
people, `.mesh` names). Internet is optional.

## Step 3: Find the client Wi-Fi

**Do:** on a phone or laptop, open Wi-Fi settings and look for:

| How it was flashed | Network name |
|---|---|
| Factory / default image | `⚡` (a single lightning-bolt character) |
| Project home fleet | `Lightning Mesh` |
| Custom flash | Whatever the flasher set as `CLIENT_SSID` |

**Expect:** an **open** network (no lock). Join it. You should get an
address that starts with `10.42.` within a few seconds.

**If not:**
- You see `mjolnir-mesh` — that is the **routers'** backhaul, not for
  phones. Ignore it.
- No Lightning Mesh name at all — wait two minutes after power-on, then
  stand closer. If it still never appears, the box may not have been
  flashed; see [Choose a router and flash OpenWrt](../node/01-hardware-and-flash.md).
- The network has a lock — the flasher set a password. Ask them.

Do not set this phone or laptop's default route through the mesh if you
are also using it to talk to the internet on another network. One radio
joins one Wi-Fi at a time.

Next: [First hour on the mesh](02-first-hour.md).
