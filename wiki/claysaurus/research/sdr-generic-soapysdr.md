---
title: SoapySDR as one transport
created: 2026-11-05
updated: 2026-11-06
type: interop
tags: [interop, transport, sdr, soapysdr]
sources: [wiki/claysaurus/raw/notes/2026-11-06-soapysdr.md]
state: [FUTURE]
confidence: low
---

# SoapySDR as one transport

> Sources: Clay's radio notes, 2026-11-06
> Raw: [SoapySDR](../raw/notes/2026-11-06-soapysdr.md)
> Updated: 2026-11-06

The draft proposes one `SdrGossipTransport` over the `soapysdr` crate, so LimeSDR, bladeRF, HackRF, and an RTL-SDR would share a driver. Nothing in this repository calls SoapySDR. There is no `soapysdr` dependency and no Cargo feature for it.

The draft's path is: postcard bytes, a frame and a CRC, a GFSK modem, then `write_stream` / `read_stream` of complex samples. That code is not written. The Rust field names in the draft (`dev_info.driver`, `dev_info.label`) are sketches, not a checked API.

An RTL-SDR receives only. A receive-only device cannot be a `broadcast` implementation by itself.

SoapyRemote, in the notes, is a TCP service on the machine that has the USB device. It is not `OuterGuard`, and it is not an underlay route for the iroh endpoint.
