---
title: LimeSDR and bladeRF
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, transport, sdr, limesdr, bladerf]
sources: [wiki/claysaurus/raw/notes/2026-11-06-limesdr.md]
state: [FUTURE]
confidence: low
---

# LimeSDR and bladeRF

> Sources: Clay's radio notes, 2026-11-06
> Raw: [LimeSDR and bladeRF](../raw/notes/2026-11-06-limesdr.md)
> Updated: 2026-11-06

The draft proposes a point-to-point link between two LimeSDR USB devices, or two bladeRF devices, carrying gossip beside iroh. The daemon in that sketch is `mjolnir-meshd`, not `lightning-meshd`.

What the sketch gets wrong about this tree:

- `GossipSync` is constructed with one transport. Two carriers are two loops, or a fan-in that does not exist. They do not both feed one `GossipSync`.
- The iroh endpoint does not use an SDR as an underlay through `OuterGuard`. `OuterGuard` marks iroh's sockets so the routing policy can see them. It does not modulate IQ.
- Babel routes IP prefixes. It does not route baseband samples.

The draft's radio plan, left as a plan and not a measurement: GFSK at 9600 baud in a 200 kHz channel, 915 MHz or 868 MHz, host-side modem, USB 3, LimeSuite or libbladeRF. Range figures and week counts are the draft's estimates.

Nothing here opens a LimeSDR or a bladeRF.
