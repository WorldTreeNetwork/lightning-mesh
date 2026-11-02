---
spike_id: iroh-ula-dial
hypothesis_id: iroh-ula-dial
source_synthesis: null
run_at: 2026-11-01T19:43:50-08:00
time_spent_min: 25
verdict: go
version-pinned: iroh 1.0.0
---

# Spike: current Iroh binds and dials the house ULA

## Claim Tested

Current Iroh binds and dials the house ULA without a duplicate-address failure.

## Minimal Test

`docs/spikes/iroh-ula-dial` creates a veth pair and assigns two `/128`s with `nodad`. One end is the fixture router address `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`. The other is `fd81:2b22:8aca:0:1111:2222:3333:4444` in the same `/48`. Each side gets a host route for the other. One ICMP echo confirms the path before Iroh runs.

Each endpoint is `Endpoint::builder(presets::Minimal)` with relays disabled, one `bind_addr` on its `/128`, and ALPN `mjolnir-ula-spike/1`. That is the daemon's single-address bind shape. Endpoint B dials endpoint A's id at `[fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df]:40123` and writes `ula` on one unidirectional stream.

## Evidence

`ping_ok=true` (1 packet, 0.056 ms). Both binds succeeded. The fixture address appears once in A's bound sockets and once in A's advertised addresses. B's address appears once in each of B's lists. `connect` returned a connection. The accepted payload was `[117, 108, 97]` (`ula`). `duplicate_error=false`. The log is `docs/spikes/iroh-ula-dial/evidence.log`.

`bind_addr` on the IPv6 `/128` still leaves the preset IPv4 socket on `0.0.0.0`. That socket advertised this host's IPv4 addresses (`10.200.0.1`, `100.121.90.25`, `172.16.0.1`, `172.16.4.1`, `172.17.0.1`, `192.168.0.108`) beside the single ULA. The ULA itself was not repeated.

The veth was deleted when the process exited.

## Verdict: go

Iroh 1.0.0 bound the fixture house `/128` and completed a direct dial to that socket. The bind and the dial produced no duplicate-address error, and the `/128` was present once. This run does not choose a replacement for the `fe80::H` next hop that `overlay_link_local` derives from `10.254`, and it does not remove that IPv4 address. `remove-v4-backhaul-pin` still waits on that steer. A bind of only the `/128`, with this builder, also publishes every IPv4 address on the host.
