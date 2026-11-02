# Design

## Filters

Babeld uses the first matching filter. Both `render_babeld_conf` and `render_overlay_babeld_conf` grow the same four lines when a house router `/128` is known, placed so the allow is before the existing `fc00::/7` deny:

```
redistribute local ip <house-router-/128> allow
in ip <house-/48> eq 128 allow
out ip <house-/48> eq 128 allow
```

`redistribute local deny` and `redistribute deny` stay. `in ip fc00::/7 deny` and `out ip fc00::/7 deny` stay. No `ge` and no `le`. No house guild file means no house allow, which is today's blanket deny.

`eq 128` is the allow that admits peer host routes without admitting a covering prefix. It is not the IPv4 `ge`/`le` form that babeld 1.13 rewrites by adding 96 (`configuration.c` does that only for `AF_INET`). Spike `house-ula-forward` on babeld 1.14 installed the peer `/128`, pinged it, left a stock `/128` uninstalled, and left a random address in the same `/64` unreachable. Before Babel, that peer address was not on-link on `br-mesh`. A legacy `/64` in a different `/48` stayed on-link for its own siblings, so dual-assign does not blackhole the house prefix.

The veth in `house-ula-forward` is not `mjolnir0`. Babel's job is the kernel host route. `overlay-ula-demux` showed a tun can turn `fd81:2b22:8aca::2/128 via fe80::2` into the open peer `10.254.0.2`, and that the choice is gone after `route del`.

## Overlay demux

`Fib` stays an IPv4 table filled from `ip -4 route`. Beside it, the overlay reconciler polls `ip -6 route show dev mjolnir0` and keeps only `/128` routes whose next hop is `fe80::H` in the `overlay_link_local` shape. `H` maps back to `10.254` the same way `next_hop` already maps a babel `fe80` destination. `OverlayRouter::next_hop` returns that `10.254` for a matching destination, then the existing connection index. `fe80` destinations stay on today's path. Any other IPv6 destination, including a stock prefix and an unknown address in the house `/48`, stays `None` and the reader drops it. The next poll after the kernel route disappears removes the selection. No new Iroh dial, and `fd` is not added to the bind set.

## Known address

"Known" is the house router `/128` that `house_addr_usable` would publish: present on `br-mesh` at prefix 128, not tentative, not DAD-failed. A missing or malformed guild file, or an unusable address, means no house allow. Both Babel reconcilers pass that address into the renderer on startup and on each reconcile, and a guild change, removal, or restart drops the previous allow with the old lines.

## Qualifiers

New house lines do not use `ge` or `le`. The existing gateway line `redistribute ip 0.0.0.0/0 le 0 metric 128` stays when the gateway is on and stays absent when it is off.

## What this node does not change

The house address remains a `/128` on `br-mesh` only. The legacy `/64` stays. IPv4 client routes keep a link-local next hop. Iroh's bind set still excludes `fd`. Two sites of one house are not a foreign admission.

## Unverified hypotheses

None. `house-ula-forward` is go for kernel and Babel on a veth: `docs/spikes/house-ula-forward/finding.md`. `overlay-ula-demux` is go: `docs/spikes/overlay-ula-demux/finding.md`. An `fd` `/128` via `fe80::2` selected the open `10.254.0.2` peer and that selection was gone after the route was deleted. The spike did not open an Iroh handshake, did not bind `fd`, and did not change `OverlayRouter::next_hop`.
