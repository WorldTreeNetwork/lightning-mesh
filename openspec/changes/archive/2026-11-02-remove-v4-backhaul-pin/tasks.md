# Tasks

- [x] Wait for `docs/spikes/iroh-ula-dial/finding.md` to say go. Do not mark this skipped from a campaign grant
- [x] Steer a next-hop key that replaces the last 16 bits of `10.254` before any bind edit. User chose the peer's house router `/128`
- [x] Fresh advise after preparation
- [x] R4 (astra-arch-review, round 2): Finish R2 by specifying the peer `/128` underlay-route source and startup dependency in LAN and overlay modes, reconciling the retained Babel-only reachability clause, and defining outer/inner route separation when `mjolnir0` has the same peer host route. Add an isolated relay-disabled cold-start journey without a manually preinstalled peer route, plus route-change/path-loss checks. See reviews/2026-11-01-advise-r2.md
- [x] R1 (astra-arch-review): Specify the Babel observation and authenticated peer association that yield the next-hop house `/128`, including interface scope, foreign-house peers, IPv4 next-hop representation, and invalidation; add transit, IPv4 payload, and withdrawal/replacement fixtures. See reviews/2026-11-01-advise.md
- [x] R2 (astra-arch-review): Specify link-local control dispatch and cold `/128` dial bootstrap without an existing overlay route; define how outer Iroh traffic avoids recursive use of `mjolnir0`, with cold-start and route-change verification. See reviews/2026-11-01-advise.md
- [x] R3 (astra-arch-review): Specify usable-address startup ordering, endpoint invalidation/recovery on house or interface changes, mode/explicit-bind scope, and the Iroh configuration removing its default IPv4 socket; add socket/candidate and lifecycle checks. See reviews/2026-11-01-advise.md
- [x] After accept, bind and dial Iroh on the usable house router `/128`, assign no `10.254`, do not run pt9, call `clear_ip_transports`, and key each open connection by `router_ula` of the local guild and the authenticated peer endpoint id
