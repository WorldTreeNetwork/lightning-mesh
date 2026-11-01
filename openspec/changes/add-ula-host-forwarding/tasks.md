# Tasks

- [ ] When a house router `/128` is known, both Babel renders allow that `/48` with `eq 128` before the `fc00::/7` deny, and redistribute that `/128` before `redistribute local deny`
- [ ] With no house guild, both renders keep the blanket `fc00::/7` deny and add no house allow
- [ ] Do not emit `ge` or `le`, do not install a house covering prefix, and do not put `fd` in the Iroh bind set
- [ ] Test the rendered allows, the no-guild deny, and that a stock prefix is not an allow
- [ ] Fresh advise after preparation

## Owed from fresh advise (2026-11-01, astra-arch-review)

- [ ] R1: Reconcile the cross-site forwarding contract with `OverlayRouter::next_hop` dropping non-link-local IPv6 and `fib_mirror` reading only IPv4. Specify IPv6 host-route next-hop resolution and withdrawal through existing Iroh connections (or its explicit prerequisite), and require actual overlay-path acceptance for both house hosts, the return path, and an unknown destination. See `reviews/2026-11-01-advise.md`.
- [ ] R2: Scope the ge/le prohibition to new house rules in design, tasks, and the delta; retain the existing IPv4 gateway `le 0` export and require gateway-on/off regression coverage in both renderers.
- [ ] R3: Define the source and eligibility of the “known” house address, wire its refresh/removal into both Babel reconciler contracts, and require transition coverage for startup, guild replacement/removal, unusable addresses, and restart without stale house permissions.
