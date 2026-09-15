# Independent architecture advise — add-household-trust-contract

<!-- Reader timed out before its verdict. Retained as incomplete evidence, not advise acceptance. -->

## Blind take (pre-registered, before opening design.md / tasks.md / delta)

Formed from proposal **Why/What** only, `living-specs/spec.md`, the cited
control-plane doc (`secure-context-and-control-plane.md`), and `portal.rs`.
These are the concerns the author must have answered — not a competing design.

1. **PIN** — Owner authority is a key the owner holds, rooted in a
   physical-presence claim. It must never be derivable from serving a page,
   holding a friendly name, or winning a gossip race. `portal.rs` confirms the
   public front desk proves nothing about ownership; the contract must say so.
2. **PIN** — Every consumer verifies owner-signed records itself; the entry /
   forwarding router is untrusted, and no node trusts a client-declared flag.
3. **PIN** — Four authorities must stay distinct and never collapse into each
   other: owner (root), issuer (delegator), operation holder (the key that
   executes), keyspace membership (mere presence). Membership ≠ authorization;
   an attractive role label ≠ router administration.
4. **PIN** — Every operation is bound to target node + operation + complete
   args + challenge + authority epoch + request id; replay state is persisted;
   nodes return signed receipts; a delegated grant is strictly narrower than
   owner.
5. **PIN** — Revocation needs a monotonic epoch/sequence so a stale-but-validly
   -signed authority is refused; recovery = the exportable phrase reconstitutes
   owner root without letting a hostile browser/router lock the owner out or
   let an attacker in.
6. **REFUSE** — Wall-clock as a trusted validity input. Nodes reboot without a
   trustworthy RTC; validity windows must degrade to sequence/epoch or an
   explicit "authority uncertain" denial, never fail-open on unknown time.
7. **REFUSE** — Reusing an identity/origin across owners, or letting a one-hour
   name lease drive long-lived authority; ownership transfer rotates identity,
   it does not inherit trust.
8. **REFUSE** — Accepting this contract as evidence of a built verifier. The
   living-spec rule is explicit: a `SHALL` in `changes/` is not runtime. The
   advise is contract-only.
9. **TRADEOFF** — One explicit authority boundary buys hostile-router safety at
   the cost of a hard bootstrap (physical pairing) and offline-availability
   friction (cannot confirm revocation while partitioned). The design must
   split the posture: **fail-closed on authority, fail-open on public browsing
   / internet** — and state that split explicitly.
10. **HUMAN DECISION I EXPECT** — How much offline authority a partitioned node
    may exercise on stale-but-unrevoked capabilities (availability vs. safety),
    and whether transfer/recovery can ever be adjudicated without the owner's
    physical presence. These are policy calls, not code calls.
