# steer add-ula-host-forwarding

**When.** 2026-11-01
**Depth.** standard

## Decided
- Cross-site path: measure the overlay demux before landing (user)
  Why: Babel on a veth can install a house `/128`, and `OverlayRouter::next_hop` still drops every `fd` destination. This node must not publish host routes or teach the demux until a spike shows an `fd` `/128` selecting an already-open Iroh connection and withdrawing with the route. `fd` stays out of the Iroh bind set. `10.254` stays the dial pin. IPv4 next hops stay link-local.

## Skipped
- None.

## Feeds change
Do not act on the Babel allows until spike `overlay-ula-demux` says go. The spike claim is: an `fd` host route selects an already-open Iroh datagram connection, and that selection disappears when the route is withdrawn. A go on `house-ula-forward` does not cover this. `bny6.5`, `bny6.7`, and `bny6.8` stay behind this node. `bny6.9` does not wait on it.
