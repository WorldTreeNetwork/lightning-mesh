# steer add-household-trust-contract

**When.** 2026-09-15
**Depth.** standard

Held in the second laptop session alongside the secure-context and SSH-free
control-plane proposal
([`docs/network-coordination/secure-context-and-control-plane.md`](../../../docs/network-coordination/secure-context-and-control-plane.md)).
Recorded here because both decisions below feed this contract's downstream
beads.

## Decided
- **Grant timing (g3y3): security profiles. DECIDED v2** (user, later the same
  day, after the [Sol consult](notes/2026-09-15-sol-grant-profiles-consult.md);
  supersedes the "FINAL" two-timer values below).
  - **Profiles** (token default / token ceiling / stale-authority maximum):
    - Strict: 15 min / 1 h / 15 min
    - **Standard (default): 24 h / 7 d / 24 h**
    - Relaxed (opt-in, persistent warning): 7 d / 30 d / 7 d
  - **Unlimited** only for private read-only diagnostics on the owner's
    registered holder-bound device. It still ends on revocation or epoch change,
    and it's never available for configuration-changing or delegated Admin
    grants.
  - **Grant-class caps:**
    - network/radio/firewall/DNS/uplink: ≤ 24 h (≤ 1 h Strict)
    - firmware install: one transaction, ≤ 15 min, digest-bound
    - ownership/issuer/recovery-policy/protected keys: owner-only, single-use,
      ≤ 5 min
    - mixed grants take the shortest cap
  - **Also decided:**
    - an unlocked owner signer renews locally without internet
    - profile changes only affect new grants
    - "fresh" means signed authority state, not internet

  Why: Duke found 900 s / 3600 s too strict for self-installed households and
  wanted an easier middle default. Sol cautioned against unlimited mutating
  authority. The stale-authority bound keeps partitioned routers safe while
  tokens get long.
- *(Superseded)* **Grant timing: two separate timers ("FINAL")** (user, a direct
  answer to a question that separated the two timers; superseded by v2 above).
  1. **Token lifetime.** A configuration-changing grant defaults to 900 seconds.
     Owner policy may set up to a 3600-second v1 ceiling. Policy never lengthens
     issued grants, and renewal needs fresh authorization.
  2. **Stale-authority window.** A grant is usable only for **at most 900
     seconds after the destination last verified fresh authority and revocation
     information**. After that, privileged use needs explicit owner
     reauthorization. An owner with their signer present may reissue locally
     while offline.

  A 3600-second token **never** authorizes an hour of stale authority. Ordinary
  connectivity and hub identity are unaffected.

  Why: owners who want longer tokens get them, while an isolated node never
  acts on authority older than 15 minutes.

  History: an earlier note said "15 minutes maximum". A later note said "900 s
  default, 3600 s ceiling" with no separate stale window. Both are superseded by
  this entry.
- **Owner claim ceremony: WPS press (or storage-node button) as physical
  presence plus reviewed per-device proof** (user, reconciled with accepted
  finding F-C).
  - An earlier same-day preference for a WPS-only pairing window was
    withdrawn.
  - `bf7.1` still selects and reviews the concrete per-device proof, under the
    confirmed recovery policy (enrolled recovery credential plus physical
    access).

  Why: F-C says the existing WPS WAN-SSH window isn't device proof.
- **Hard-custody signer order: Lightning Admin desktop first**, then browser
  extension, then mobile (user).

  Why: the desktop app already reaches the fleet, and operators are the first
  users of authenticated admin. It fits this contract's shared web/desktop
  setup.
- **Governance boundary: ai0 governs trust and control** (owner claim `bf7.1`,
  grants `ai0.1`, control-record integrity `ai0.9`, signed node control
  `b6j.2`) (user). The parallel plan keeps:
  - HTTPS on `mesh.worldtree.network` for apps and per-router front desks
    (`b6j.1`)
  - mini-apps (`ncy`)
  - storage nodes (`b6j`, one software image)
  - identity export
  - owner-signed name records for resolution and certificate issuance, beyond
    `ai0.9`'s admin-record scope

## Skipped
- Exact clock-evidence protocol for grant freshness: still belongs to `ai0.1`.
- Concrete per-device proof mechanism: still belongs to `bf7.1`.

## Feeds change

**Result of the change pass (2026-09-15): one delta change.** Everything
except the stale-authority window was already in this change:
- **Grant timing.**
  - design.md "Delegated v1 timing policy" and the spec requirement "Separate
    bounded recovery and administrative lifetimes" now carry the **security
    profiles** (Standard default), the stale-authority maximum per profile, the
    grant-class caps, and the read-only no-expiry boundary. They replace the
    900 s / 3600 s values.
  - `add-mesh-admin-capabilities` design notice and spec are aligned in the same
    commit.
- **Claim ceremony.** design.md "Claim and recovery boundary" already says the
  WPS window isn't proof, and requires device-specific proof plus a fresh
  physical ceremony ("WPS-like interaction, not use of the WPS authentication
  protocol").
- **Signer.** proposal.md's journey already names Lightning Admin's setup.
- **Scope.** HTTPS isn't in this contract. It's planned in
  `openspec/changes/add-https-aliases` (`b6j.1`), which depends on this
  contract's house and node identities.

The existing review evidence still applies to the trust architecture. Per the
Sol consult, the profile values are a policy parameter inside the accepted
contract, not a contract change. Because normative requirements changed, they
need a **targeted timing/profile re-review** together with `ai0.1.2`'s
authenticated time-evidence mechanism. That review covers:
- per-profile stale maxima
- class caps
- epoch-bound no-expiry read-only grants
- fail-closed behaviour
- issuer revocation under partition, tested per profile (extends F-D)

The pending ceremony re-review for the tightened recovery policy is unchanged.
