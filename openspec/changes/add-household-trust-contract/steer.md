# steer add-household-trust-contract

**When.** 2026-09-15
**Depth.** standard

Held in the second laptop session alongside the secure-context and SSH-free
control-plane proposal
([`docs/network-coordination/secure-context-and-control-plane.md`](../../../docs/network-coordination/secure-context-and-control-plane.md)).
Recorded here because both decisions below feed this contract's downstream
beads.

## Decided
- **Offline validity for privileged grants (g3y3): keep the contract as
  written** (user, confirmed in the change session the same day).
  - Configuration-changing grants default to 900 seconds. Owner policy may set
    up to a 3600-second v1 ceiling.
  - Renewal needs fresh authorization. Policy can't lengthen issued grants.
  - An earlier same-day note recorded "15 minutes maximum". That was corrected:
    15 minutes is the default, not a hard maximum.

  Why: offline admin availability for owners who choose it, with small
  default exposure to unpropagated revocation.
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

**Result of the change pass (2026-09-15): no delta change needed.** Every
decision above was already in this change:
- **Grant lifetime.** design.md "Delegated v1 timing policy" and the spec
  requirement "Separate bounded recovery and administrative lifetimes" carry the
  900-second default and 3600-second ceiling, matching
  `add-mesh-admin-capabilities`.
- **Claim ceremony.** design.md "Claim and recovery boundary" already says the
  WPS window isn't proof, and requires device-specific proof plus a fresh
  physical ceremony ("WPS-like interaction, not use of the WPS authentication
  protocol").
- **Signer.** proposal.md's journey already names Lightning Admin's setup.
- **Scope.** HTTPS isn't in this contract. It's planned in
  `openspec/changes/add-https-aliases` (`b6j.1`), which depends on this
  contract's house and node identities.

The existing review evidence therefore still applies. The pending ceremony
re-review for the tightened recovery policy is unchanged by this steer.
