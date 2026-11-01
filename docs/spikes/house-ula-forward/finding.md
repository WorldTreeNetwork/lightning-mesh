---
spike_id: house-ula-forward
hypothesis_id: null
source_synthesis: null
run_at: 2026-11-01T09:46:00Z
time_spent_min: 25
verdict: go
version-pinned: babeld 1.14 (Arch extra) runtime; babeld 1.13 configuration.c read for the IPv4 plen quirk
---

# Spike: a house /128 is a Babel host route and is not on-link

## Claim Tested

A node /128 in the house prefix is reachable from another site through Babel on a tunnel interface, and the local kernel does not treat that address as on-link on br-mesh.

## Minimal Test

Two network namespaces, each with a dummy `br-mesh` holding only its house `/128`, joined by a veth that stands in for the overlay interface. babeld 1.14 ran with an exact `/128` allow and, in a second trial, `in/out ip fd81:2b22:8aca::/48 eq 128 allow`, both followed by `fc00::/7 deny`. A stock `/128` was configured on the far side and not allowed. The legacy `/48` was also assigned as a `/64` on the near side.

## Evidence

`docs/spikes/house-ula-forward/evidence.log` via `sudo bash docs/spikes/house-ula-forward/repro.sh`.

- Before Babel, `ip -6 route get fd81:2b22:8aca::2` returned `Network is unreachable`, not `dev br-mesh`. The same for `fd81:2b22:8aca::99`.
- The legacy `/64` sibling `fd53:6213:4797::99` was on-link on `br-mesh`. A different `/48` does not pull the house prefix on-link.
- Babel installed `fd81:2b22:8aca::2 via fe80::… dev ulaA proto babel`. Ping: 2 transmitted, 2 received.
- The stock prefix had no route. Ping of the random house address returned `Network is unreachable`.
- `eq128_installed=1` on babeld 1.14.
- babeld 1.13 `configuration.c` adds 96 to prefix lengths only when `filter->af == AF_INET`. An IPv6 prefix keeps `AF_INET6`, so `eq 128` does not take that IPv4 path. The binary itself did not link here because the `rfc6234` submodule clone was refused.

## Verdict: go

A `/128` is not on-link. Babel installs that host route over a link-local next hop and does not install a stock prefix or answer a random address in the same `/64`. `eq 128` on the house `/48` is a usable allow on babeld 1.14, and the 1.13 source does not apply the known IPv4 qualifier bug to an IPv6 prefix.

The tunnel in this run was a veth, not `mjolnir0`. Babel still spoke its normal protocol and installed a kernel host route. The multicast flag `mjolnir0` already sets is unchanged by this result.
