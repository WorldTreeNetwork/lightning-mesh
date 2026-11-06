# source

# Reticulum carriers

> Sources: underlay.rs and Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum carriers](../raw/notes/2026-11-06-reticulum-carriers.md)
> Updated: 2026-11-06

The notes list Reticulum interfaces: serial, KISS, AX.25, RNode LoRa, a pipe, Ethernet, UDP, TCP, and I2P. Each is described as `transmit` and a blocking `receive`. This repo does not contain `Interface.py`.

meshd does not spawn any of those. `tun/underlay.rs` is routing table 210 and the relay-route policy. It is not a multi-link datagram connection, and Babel does not absorb a Reticulum interface. There is no `reticulum-carriers` Cargo feature.

A radio or a serial line would be a second `GossipTransport` beside iroh, or a process beside meshd. That is a proposal. The live backhaul is 802.11s `br-mesh` and wired node-links.
