# Tasks

- [x] When a house router `/128` is known, both Babel renders allow that `/48` with `eq 128` before the `fc00::/7` deny, and redistribute that `/128` before `redistribute local deny`
- [x] With no house guild, both renders keep the blanket `fc00::/7` deny and add no house allow
- [x] New house lines do not use `ge` or `le`; the gateway `le 0` line stays when the gateway is on and stays absent when it is off; do not install a house covering prefix; do not put `fd` in the Iroh bind set
- [x] Resolve an `mjolnir0` IPv6 `/128` via `fe80::H` to the open `10.254` connection, and drop that selection when the route is gone
- [x] Test the rendered allows, the no-guild deny, and that a stock prefix is not an allow
- [x] Fresh advise after preparation

## Owed from fresh advise (2026-11-01, astra-arch-review)

- [x] R1: Reconcile the cross-site forwarding contract with `OverlayRouter::next_hop` dropping non-link-local IPv6 and `fib_mirror` reading only IPv4. Specify IPv6 host-route next-hop resolution and withdrawal through existing Iroh connections (or its explicit prerequisite), and require actual overlay-path acceptance for both house hosts, the return path, and an unknown destination. See `reviews/2026-11-01-advise.md`.
- [x] R2: Scope the ge/le prohibition to new house rules in design, tasks, and the delta; retain the existing IPv4 gateway `le 0` export and require gateway-on/off regression coverage in both renderers.
- [x] R3: Define the source and eligibility of the “known” house address, wire its refresh/removal into both Babel reconciler contracts, and require transition coverage for startup, guild replacement/removal, unusable addresses, and restart without stale house permissions.
