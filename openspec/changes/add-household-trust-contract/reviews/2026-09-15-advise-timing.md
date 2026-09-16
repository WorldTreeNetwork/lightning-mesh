# Targeted timing/profile review
> **ADVISE:** accept
> **READER:** sol-arch-review
> **SPAWN:** /Users/dukejones/work/WorldTree/lightning-mesh/.spawns/add-household-trust-contract-advise-timing-1789537951-98063-3a82ccdf

## Blind pass

Written before opening `design.md`, `tasks.md`, `steer.md`, or `time-evidence.md`:

1. Pin Standard as the default and require Strict/Standard/Relaxed stale-authority maxima of 15 minutes, 24 hours, and 7 days.
2. Pin token defaults and ceilings separately from stale-authority maxima so a long-lived token never implies equally long offline authority.
3. Pin grant-class caps that override profile allowances, with the shortest cap winning for grants spanning classes.
4. Refuse every unlimited configuration-changing or delegated-Admin grant, regardless of profile or owner request.
5. Permit no-wall-clock-expiry only for private read-only diagnostics on a registered, holder-bound owner device.
6. Make that diagnostic exception epoch-bound and revocation-bound; verified epoch or holder revocation must terminate it.
7. Refuse privileged use whenever trustworthy time or freshness cannot be established, including rollback and restart cases.
8. Define freshness as locally verifiable signed authority and revocation state, never mere connectivity or an untrusted wall clock.
9. Require partition tests per profile proving issuer revocation cannot leave mutating authority usable beyond the stated stale maximum.
10. Tradeoff: Relaxed improves offline operability but knowingly expands a removed administrator's partition window, so it must be opt-in and persistently warned.

## Comparison and steelman

The amendment answers the blind-pass concerns without reopening the accepted trust architecture in `reviews/2026-09-15-advise-2.md`.

- `specs/household-authority/spec.md` and `design.md` agree on the two independent timers, Standard default, and exact Strict/Standard/Relaxed values: token default/ceiling/stale maximum are 15m/1h/15m, 24h/7d/24h, and 7d/30d/7d. Issuance-time policy prevents a later profile change from lengthening an existing grant.
- Grant-class caps are explicit and compositional: primary network mutation is capped at 24h (1h Strict), installation is one digest-bound transaction within 15m, authority-sensitive changes are owner-only/single-use within 5m, and a mixed grant takes the shortest cap. Both contract and pin refuse unlimited mutation and delegated Admin authority.
- The narrow no-wall-clock-expiry exception is private read-only diagnostics on the owner's registered holder-bound device. The contract ends it on verified holder revocation or authority-epoch change; `time-evidence.md` further makes the issuance-profile stale maximum and missing-time refusal apply during a partition. Thus “no expiry” is not immortal authority.
- Freshness fails closed. The pin excludes process wall time, requires destination-produced authenticated `TimeEvidence`, persists a nondecreasing high-water floor, and refuses missing, regressed, unauthenticated, or post-reboot-unrecoverable evidence. Ordinary forwarding remains outside this administrative failure mode.
- The negative-vector set extends F-D for issuer revocation under partition at 15m, 24h, and 7d, plus a positive inside-window control. That distinction prevents an implementation from passing merely by refusing every partitioned request.

The strongest case against the policy is the Relaxed profile's seven-day bounded exposure: a removed issuer or administrator can remain effective on an isolated destination until the stale limit. The text does not hide that cost; Relaxed is opt-in, carries a persistent warning, and remains subordinate to tighter grant-class caps. This is a deliberate availability/security tradeoff, not an unbounded-authority escape.

## Verdict

A worker can implement the timers, issuance-time profile binding, class caps, epoch/revocation-bounded diagnostic exception, and authenticated-time fail-closed behavior without guessing values or treating wall clock or internet reachability as authority. The per-profile issuer-revocation partition vectors make the stated exposure bounds testable. No send-back boxes are owed; `tasks.md` remains unchanged.

This accepts the proposed contract and implementation pin only. A `SHALL` under `openspec/changes/` is not evidence that the verifier exists or that these vectors pass at runtime.
