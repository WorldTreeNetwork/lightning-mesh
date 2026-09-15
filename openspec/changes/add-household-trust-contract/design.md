# ADR proposal: authority is checked at the destination

Status: proposed under activated ai0.3; independent review outstanding.

## Inputs and limits

The household dossier's identity-and-welcome.md records reviewed IdentiKey protocol
commits and distinguishes adopted formats from missing runtimes. Reuse its Biscuit
agency format and FOKS-inspired keyspace separation. Neither this contract nor its
review claims those runtimes are implemented. Existing signed public name claims
are bounded self-service, not authorization for a node operation.

## Principals

House identity is a stable identifier bound to the initial owner's signed genesis
record. A node's pinned identity and ownership epoch bind its current house and
owner authority. Owner credentials authorize issuer keys and ownership transitions;
issuer keys mint bounded Biscuit grants; holder keys sign operations. These are
distinct roles even when a prototype uses one protected device for custody.
An IdentiKey/keyspace membership record and Wi-Fi association confer none of them.

One initial owner is sufficient. Admin, Member and Guest are UI presets, not an
ordered numeric privilege hierarchy. Admin grants enumerate actual operations,
targets and lifetime, with delegation disabled unless explicitly allowed.
Issuer private keys must not be copied to every router. Destinations need public
issuer pins and verified current authority state, not minting secrets.

## Claim and recovery boundary

An unclaimed node accepts one atomic owner binding only after device-specific proof
and a fresh physically armed ceremony are both verified. Bind the ceremony to the
node identity, proposed owner credential, challenge and current unclaimed epoch;
fail closed on reuse, concurrent losing claims or interrupted persistence. Discovery
and the existing WPS WAN-SSH window are not such proof. bf7.1 must select and review
the actual provisioning channel/ceremony; this contract does not endorse unaudited
custom key exchange or assume a trusted browser origin.

After claiming, a physical button alone cannot replace an owner. Normal transfer
requires existing owner authority. Owner recovery requires both a previously
enrolled recovery credential and fresh physical presence at a node, as confirmed
by Duke in response to the recovery-policy question. Neither factor alone suffices;
possession of the credential does not permit remote-only recovery. Bind the local
confirmation to the specific recovery request, node identity and ownership epoch
so an unrelated button press cannot authorize a competing remote request. bf7.1
must define and qualify that ceremony; this decision does not claim hardware proof
already exists or require physical access to every node in the house.
Losing all such credentials requires an explicitly destructive
local reset with loss of prior house authority/data access, not covert takeover.
Recovery must rotate authority epochs and reject previously issued control grants.
Every node must enforce its own durable transition before it reports completion;
offline nodes remain outstanding, not silently recovered/revoked.

## Delegation and authorization

The target verifies pinned house/node identity and authority epoch, approved issuer,
the complete Biscuit chain/checks, authenticated holder and structured request.
Trusted facts come from that verifier: exact house, target, operation, argument
digest, credential, epoch, time evidence and request freshness. A token-supplied
fact cannot override ambient verifier facts or confer issuer trust.

Attenuation may intersect operations, resources and time, never widen them. A change
of holder is not accomplished by adding a contradictory holder caveat. Reissue a
new holder-bound grant only through an approved issuer enforcing the parent's
explicit delegation ceiling, lifetime and revocation lineage. V1 may require owner
approval for every new recipient instead of autonomous delegated minting.

A signed control request binds protocol version, house, node, authority epoch,
operation and all arguments to a one-use challenge/request identifier. The target
must durably serialize acceptance against concurrent replay and revocation. A lost
reply is not permission to execute again: an authenticated retry returns a recorded
receipt or a pending/unknown outcome. Receipt correlation does not prove hardware
success; network apply has its own independently verified result contract.

## Time, partitions and privacy

Persist a trusted time floor and authority revision; clock rollback cannot extend a
grant. If the target cannot establish validity after reboot, privileged operation is
refused with a recovery/time-unavailable reason rather than accepting indefinitely.
Exact clock-evidence protocol and maximum offline validity belong to ai0.1 and must
have negative vectors before implementation is advertised as safe.

Revocation is immediately enforced on nodes that have verified/persisted it. An
isolated node cannot know about a newer revocation: show pending enforcement, and
bound stale authorization with grant expiry. No UI may claim global revocation while
target acknowledgements are missing. Untrusted gossip cannot roll back authority.

HTTP hello.mesh receives no owner seed or reusable admin bearer authority. Installed
signing UI displays authenticated target, operation, impact and expiry; an HTTP
adapter may relay opaque requests but cannot supply trusted confirmation text.
Never put keys, tokens or secret-bearing operation arguments in URLs/public logs.

## Qualification owed downstream

## Delegated v1 timing policy

> **Timing policy: security profiles (Duke, 2026-09-15, decided after the
> [Sol consult](notes/2026-09-15-sol-grant-profiles-consult.md); see
> [steer.md](steer.md)).** This supersedes every earlier same-day timing
> reading (900 s / 3600 s). The recovery window below is unchanged. `ai0.1.2`
> owns the authenticated time-evidence mechanism
> ([engine pin](../add-mesh-admin-capabilities/time-evidence.md)). The profile
> values need a targeted timing re-review.

Duke delegated this choice with "decide for me" and resumed the campaign.
The physical recovery window defaults to 120 seconds, is configurable by an
authorized owner, and closes after one successful recovery. This is a WPS-like
interaction, not use of the WPS authentication protocol. Enrollment proof and
fresh physical confirmation are still both required. Reboot or loss of the
window's monotonic timing state closes it; retry cannot silently extend it.

Configuration-changing admin grants carry **two timers**:
- **token lifetime**: how long the grant exists
- **stale-authority maximum**: how long a destination may act on it without
  verifying fresh authority and revocation information

The owner selects a security profile. **Standard is the default.**

| Profile | Token default | Token ceiling | Stale-authority maximum |
|---|---:|---:|---:|
| Strict | 15 min | 1 h | 15 min |
| **Standard (default)** | **24 h** | **7 d** | **24 h** |
| Relaxed (opt-in, persistent risk warning) | 7 d | 30 d | 7 d |

**Profile rules**
- Owners may shorten either timer. Going beyond a profile requires switching
  profile.
- Profile and policy changes apply only to newly issued grants. They never
  lengthen an issued grant or exceed its issuer or delegation ceiling.
- An unlocked owner signer renews grants locally, with no internet needed.
  Delegates need owner-authorized reissue.
- Renewal is not a sliding lease extended by activity.
- "Fresh" means locally verifiable signed authority state, not internet
  connectivity.
- Relaxed tells the owner plainly: "A removed administrator may retain control
  of an isolated router for up to 7 days."

**Grant-class caps** apply regardless of profile. A token spanning classes takes
the shortest applicable cap.

| Class | Cap |
|---|---|
| Public diagnostics | No grant needed |
| Private read-only diagnostics | May have no wall-clock expiry on the owner's registered, holder-bound device, but still end on holder revocation or authority-epoch change. Exclude secret or personal logs, key export and any mutation. Delegates get at most 30 days |
| Guest Wi-Fi and constrained device management | Follows the profile |
| Primary Wi-Fi, backhaul, radio, routing, firewall, DNS, uplink | Never unlimited. At most 24 h (1 h under Strict); stale authority never exceeds 24 h |
| Firmware or software installation | Single transaction, at most 15 min, bound to the artifact digest, owner approval by default |
| Ownership, issuer, recovery-policy, protected-key changes | Owner-only, single-use, at most 5 min. Claim and recovery ceremonies still apply |

No configuration-changing or delegated Admin grant is ever unlimited. A durable
owner credential is an issuer identity, not a standing execution capability.

Internet access and ordinary public hub use have no administrative expiry
requirement. Unreliable expiry or freshness evidence refuses privileged changes,
not ordinary connectivity.

The finite recovery-window configuration limits and concrete timing mechanism
remain part of the bf7.1 ceremony design, not a new human gate. They must exclude
an unlimited window. The security profiles, grant-class caps and read-only
no-expiry boundary above resolve the offline-validity decision g3y3. Internet
connectivity alone is not evidence that authority information is fresh.

## Qualification owed downstream (continued)

Owner binding races/crash recovery (bf7.1), token origin/holder/attenuation vectors
(ai0.1), non-ingesting consumer forgery/replay rejection (ai0.9) and exactly-once
acceptance/unknown outcomes (b6j.2) must be implemented and tested independently.
This node is the shared contract, not a claim those acceptance tests already pass.
