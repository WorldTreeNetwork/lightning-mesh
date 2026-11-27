---
title: SDR and radio integration
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, transport, radio, sdr]
sources: [crates/mjolnir-mesh/src/radio.rs, crates/mjolnir-mesh/src/crdt/sync.rs, wiki/claysaurus/raw/notes/2026-11-06-sdr-radio.md]
state: [FUTURE]
confidence: medium
---

# SDR and radio integration

> Sources: radio.rs, sync.rs, and Clay's radio notes, 2026-11-06
> Raw: [SDR survey](../raw/notes/2026-11-06-sdr-radio.md)
> Updated: 2026-11-06

The live backhaul is 802.11s on `br-mesh`, plus wired node-links. There is no `backhaul.rs`.

`radio.rs` reads `iw dev`, `iw station dump`, and `iw mpath dump`. It writes `radio.json` for the browser. It does not transmit.

The seam for another carrier is `GossipTransport` in `crates/mjolnir-mesh/src/crdt/sync.rs`: async `broadcast` and `recv` of raw bytes. `GossipSync` takes one implementation. It postcard-encodes a `GossipMessage` and does not know the carrier. The production implementation is `IrohGossipTransport` in `mjolnir-meshd`. A second carrier would be a second implementation, and a second run loop. It would not be plugged into the same `GossipSync` as iroh, and it would not inherit QUIC. Those bytes are cleartext unless that implementation encrypts them.

`tun/underlay.rs` is routing table 210, mark `0x4d4a`, and relay-route policy. `OuterGuard` in `bin/meshd/transport.rs` is the nft table for iroh sockets. Neither one is an SDR device or a place to attach IQ samples. `DatagramConn` is the TUN encapsulation seam. The tree has no `UnderlayRoute`, `spawn_underlay_routing`, or `SdrTransport`.

The draft surveys RTL-SDR, HackRF, SoapySDR, 802.11ah HaLow, and LoRa as hardware that could someday carry those bytes. None of that hardware is called from this repo. Prices, sample rates, and star counts in the draft are not checked here.
