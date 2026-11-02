---
spike_id: overlay-ula-demux
hypothesis_id: null
source_synthesis: null
run_at: 2026-11-01T16:40:00Z
time_spent_min: 35
verdict: go
version-pinned: tun 0.8.14, Linux 7.2.11, iproute2 on the host
---

# Spike: an fd /128 selects an open peer and leaves with the route

## Claim Tested

An `fd` `/128` host route selects an already-open datagram peer, and that selection disappears when the route is withdrawn. `fd` is not bound as a local UDP socket. No Iroh handshake is opened.

## Minimal Test

A tun device `spikedmx` stood in for `mjolnir0`. One peer, `10.254.0.2`, was marked open. Its link-local is `fe80::2`, the same derivation as `overlay_link_local`. The kernel route was `fd81:2b22:8aca::2/128 via fe80::2 dev spikedmx onlink`, with a permanent neighbor so the echo was emitted. A reader rebuilt a host map from `ip -6 route show dev spikedmx` and counted a packet only when its destination was that `/128` and the next hop was the open peer. The route was then deleted and the map rebuilt. A second address, `fd81:2b22:8aca::99/128 via fe80::3`, mapped to `10.254.0.3`, which was not open. `fd12:3456:789a::1` had no route. The process never created a UDP socket.

## Evidence

`docs/spikes/overlay-ula-demux/evidence.log` via `sudo docs/spikes/overlay-ula-demux/repro-command.sh`.

- `house_route_ok=true`, `table_after_house=1`, `via_open=fe80::2`.
- One packet to `fd81:2b22:8aca::2` hit the open peer (`during_open=1`). Eight other packets on the tun, neighbor discovery and similar, were not counted (`during_dropped=8`).
- Ping did not get an echo reply (`ping_house_ok=false`). Nothing on the far side answered. The claim is selection, not reachability.
- After `route del`: `after_delete_still_mapped=false`, `replay_after_delete=none`, `ping_after_delete_ok=false`, `after_open=1`. The second send did not select the peer.
- `fd81:2b22:8aca::99` mapped to `10.254.0.3` (`random_mapped=10.254.0.3`) and did not increment the open peer (`open_hits_unchanged_for_random=true`).
- The stock prefix was not mapped (`stock_mapped=false`, `ping_stock_ok=false`).
- `fd_bound_on_udp6=false`.

## Verdict: go

A kernel `/128` whose next hop is `fe80::2` is enough to choose the already-open `10.254.0.2` peer, and deleting that route removes the choice. A next hop for a peer that is not open does not fall through to the open one. A stock prefix is not selected. The tun was a real tun, not a veth. Production `OverlayRouter::next_hop` still returns nothing for `fd`, and `fib_mirror` still reads only `ip -4 route`. This run did not change that code and did not open Iroh.
