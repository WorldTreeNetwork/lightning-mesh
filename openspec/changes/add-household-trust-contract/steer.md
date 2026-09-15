# steer add-household-trust-contract

**When.** 2026-09-15
**Depth.** standard

Held in the second laptop session alongside the secure-context and SSH-free
control-plane proposal
([`docs/network-coordination/secure-context-and-control-plane.md`](../../../docs/network-coordination/secure-context-and-control-plane.md)).
Recorded here because both decisions below feed this contract's downstream
beads.

## Decided
- **Maximum offline validity for privileged grants (g3y3): 15 minutes** (user,
  took the recommendation on record).
  - Applies to configuration-changing grants. They stop being usable after 15
    minutes without fresh verified revocation or authority information.
  - Continued use needs explicit owner reauthorization. An owner with their
    signer present may reissue locally while offline.
  - Doesn't expire ordinary internet access or require hub identity.

  Why: keeps exposure to unpropagated revocation small; the owner can still
  act offline.
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
- **g3y3 is now closed at 15 minutes.** The "maximum offline validity" gap at
  design.md line ~79 is filled with that value. Keep the distinction between
  losing internet and losing authority freshness.
- **Owner claim.** Keep the WPS window as insufficient proof on its own. Make
  the claim ceremony name physical presence (WPS or storage-node button) plus a
  reviewed per-device proof.
- **Signer.** Note Lightning Admin as the first hard-custody signer surface.
- **Scope.** Keep HTTPS and certificate work out of this contract. It depends
  on this contract's owner and issuer model but is planned under `b6j.1`.
- **Re-review.** The next independent ceremony review should cover the 15-minute
  value alongside the tightened recovery policy.
