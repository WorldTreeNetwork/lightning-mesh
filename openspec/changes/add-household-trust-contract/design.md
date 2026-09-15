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

> **Confirmed by latest steering:** [the corrected decision](steer.md) retains
> the 900-second default and owner-configurable 3600-second v1 ceiling below.
> The earlier same-day 15-minute hard-cap note is superseded. Renewal requires
> fresh authorization; policy cannot lengthen issued grants. `ai0.1.2` tracks
> the exact time-evidence mechanism and independent engine-readiness review,
> not another human policy decision.

Duke delegated this choice with "decide for me" and resumed the campaign.
The physical recovery window defaults to 120 seconds, is configurable by an
authorized owner, and closes after one successful recovery. This is a WPS-like
interaction, not use of the WPS authentication protocol. Enrollment proof and
fresh physical confirmation are still both required. Reboot or loss of the
window's monotonic timing state closes it; retry cannot silently extend it.

Configuration-changing admin grants default to 900 seconds. Owner policy may
configure a positive lifetime up to 3600 seconds for v1. Renewal requires fresh
authorization checks, including verified current authority/revocation state;
it is not a sliding lease extended by activity. Policy changes cannot lengthen
an already-issued grant or exceed its issuer/delegation ceiling. Internet access
and ordinary public hub use have no such administrative expiry requirement.
Unreliable expiry evidence refuses privileged changes, not ordinary connectivity.

The finite recovery-window configuration limits and concrete timing mechanism
remain part of the bf7.1 ceremony design, not a new human gate. They must exclude
an unlimited window. The 15-minute default and one-hour admin ceiling resolve
the offline-validity decision g3y3; internet connectivity alone is not evidence
that authority information is fresh.

## Qualification owed downstream (continued)

Owner binding races/crash recovery (bf7.1), token origin/holder/attenuation vectors
(ai0.1), non-ingesting consumer forgery/replay rejection (ai0.9) and exactly-once
acceptance/unknown outcomes (b6j.2) must be implemented and tested independently.
This node is the shared contract, not a claim those acceptance tests already pass.
