# Delegated authority and the hello.mesh invitation

> 2026-09-14 refinement. User decision: one initial owner delegates to admins,
> members, and guests. Everyone connecting should be invited to discover the hub
> and optionally create an identity. Implementation remains PENDING.

## Evidence after pulling latest

Reviewed identikey-core at `63fa13660772ec0c7de5c7b5f0051ba3ed75aef0` and
identikey-protocol at `093ceb13148d5fb5267ad309ecdfea15f2637d9e`.
Both checkouts fast-forwarded cleanly. Mesh was already current at `ae3550e`.

| Source | Finding | Integration consequence |
|---|---|---|
| [Capability v1](https://github.com/identikey/identikey-protocol/blob/093ceb13148d5fb5267ad309ecdfea15f2637d9e/docs/standards/identikey-capability-v1.md) | Biscuit is the adopted agency format; this document is a draft without a capability runtime crate or filled vectors in that workspace | Define a mesh application profile and verifier; do not invent another token format or claim integration exists |
| [Keyspace v1](https://github.com/identikey/identikey-protocol/blob/093ceb13148d5fb5267ad309ecdfea15f2637d9e/docs/standards/identikey-keyspace-v1.md) | FOKS-inspired device/user/keyspace hierarchy; house and access-node profiles | Reuse this model, not FOKS servers or a new membership protocol |
| [Core guild mapping](https://github.com/identikey/identikey-core/blob/63fa13660772ec0c7de5c7b5f0051ba3ed75aef0/openspec/specs/guild-keyspaces/spec.md) | Architectural mapping is folded; membership is not an XID rewrite or an agency grant | Keep membership, operational permissions, and data permissions distinct |
| [Core runtime proposal](https://github.com/identikey/identikey-core/blob/63fa13660772ec0c7de5c7b5f0051ba3ed75aef0/openspec/changes/add-keyspace-runtime/proposal.md) | ACTIVE BUILD, describes missing apply path; inspected Rust source has no keyspace apply implementation | Track `identikey-core-trr.3`, not a claim of working wrap/rotation; login wrapping is `trr.2` |
| Same mapping and current protocol | Native X25519 C2 roster admission waits on `ikp-2l8`; keyspace authors need Ed25519 | A passkey, Schnorr key, or service-held Sign key cannot be silently substituted |
| [Existing portal](../../../crates/mjolnir-hello/src/portal.rs) and [living spec](../../specs/captive-portal/spec.md) | Optional identity CTA plus one pass-through already exist | Extend discoverability and hub framing; no mandatory signup |

## One owner; roles are permission presets

The purchaser claims the first node and becomes the one initial owner. Setup
does not require a second owner. Offer verified recovery material and credential
backup, then delegation when useful. Co-ownership is a later explicit grant or
transfer, not an automatic side effect of inviting an admin.

| Preset | Intended permission ceiling | Default delegation |
|---|---|---|
| Owner | Claim/release nodes, network policy, grants/revocation, issuer approval, recovery/transfer | May grant subordinate authority |
| Admin | Selected nodes' diagnostics and configuration | Off unless owner grants a specified delegation ceiling |
| Member | House participation and explicitly allowed services; own identity/name | No router administration |
| Guest | Guest connectivity and explicitly shared services | No admin rights or house-secret access |

Use **Member** for the role; **resident** still describes a person or Wi-Fi
audience. A member need not live in the house. Display exact permissions beneath
the role label: “Manage Wi-Fi on Hall and Office until Friday.” Do not present
roles as a numeric ladder or infer more rights from a label alone.

### Proposed mesh Biscuit profile

An owner-authorized local issuer mints the maximum allowed scope. Identity proof
and the issuer's Ed25519 Biscuit authority key are different keys: a P-256
passkey can authenticate a person but does not sign Biscuit authority blocks.
Pin issuer public keys through physically bootstrapped owner authority. Issuer
private material belongs in trusted custody, never copied across all routers.
Existing grants can verify offline without an always-on cloud issuer.

The profile must bind house, target node/resource, allowed operation set,
audience, issuer/grant epoch, and expiry. Optional restrictions include node
subsets, allowed settings, maintenance windows, and delegated-issuance ceilings.
Example intent, not frozen Datalog syntax:

```text
Owner authorizes admin: configure Wi-Fi on this house's nodes
  → narrower scope: Hall and Office only, expires Friday
  → narrower scope: read radio diagnostics only, expires in one hour
```

The destination verifies the Biscuit chain AND all checks against trusted request
facts. It also verifies a fresh structured signed operation. For privileged mesh
operations, require holder binding: the verified signer fingerprint is injected
by the verifier; a token must not assert its own trusted holder/request facts.
This is an explicit mesh profile requirement; the protocol's existing holder
rules in §7 otherwise apply to secret redemption, not every agency token.

Monotonic attenuation narrows rights; it does not prove which human appended a
block. Nor can appending a Bob-holder check remove an existing Alice-holder
check. For a new delegate, an authorized issuer verifies the parent's delegable
ceiling and issues a new holder-bound grant within it, retaining lineage and
revocation linkage. This is scoped reissuance, not a claim that attenuation can
change holders. The first UI can ask the owner to approve each recipient.
Autonomous admin-to-member issuance requires an explicit delegated-mint policy.

The application must define trusted-fact origins, authorization limits, expiry
with unreliable clocks, revocation epochs, persisted replay checks, and test
vectors. Offline verification cannot discover a revocation the node has not
received. Show pending enforcement and bound exposure with short-lived grants;
clock rollback/reboot must not extend grants silently. Rate/use counts require
verifier-maintained state. Tokens and secrets never go in URLs or public logs.

### Keyspaces complement delegation

Use the existing house keyspace profile for shared membership/key material,
and an optional nested access-node profile for lower-assurance local participation.
SSID association or creating a public identity does not automatically join either.
House-owner authority, network-admin capability, and keyspace `admin: true` are
distinct: an admin allowed to change channels need not rotate group keys.

A house may associate its product ID with the protocol genesis-hash keyspace ID;
do not invent a second identifier for the keyspace itself. PTK is the shared
keyspace secret; PUK is the member's user key used as the wrap target. Removal
updates membership; cryptographic eviction needs the specified epoch rotation.
It cannot erase old plaintext or an already-copied key. Access-node membership
churn must not rotate the house secret per visitor. Recrypt remains the separate
encrypted-data permission layer.

Do not block public browsing/soft identity on runtime keyspace work. Do block any
claim of encrypted group membership or successful key distribution until apply,
recipient compatibility, rotation, and revocation are implemented and tested.

## Secure-context boundary: defer passkeys, keep the welcome

A trusted local LAN does not make an HTTP origin a browser secure context.
Passkeys need a secure context and valid relying-party scope; a familiar TLD alone
does not fix either. A publicly delegated hostname with a trusted certificate can
resolve to a private mesh address and work offline while that certificate remains
valid. A private CA requires client trust installation. No proposal here treats
clicking through a certificate error as reliable passkey enablement. See
[W3C WebAuthn](https://www.w3.org/TR/webauthn-2/) and
[Secure Contexts](https://www.w3.org/TR/secure-contexts/).

Proposed delivery split:

- Now-designable: local hub, optional browser identity, public/consented directory,
  local services, and honest internet status, all usable without passkeys.
- Privileged administration: installed Lightning Admin signer and typed verified
  control, after implementation. Never import an owner key into HTTP hello.mesh.
- Later: secure per-node origins and tested passkey lifecycle; follow the existing
  [secure-context proposal](../../../docs/network-coordination/secure-context-and-control-plane.md).

HTTP browser identity remains low-assurance: serving code/network attackers may
read or use it. Explain “This identity lives in this browser. Use Lightning Admin
for protected keys and network administration.” Do not call it a passkey or use it
to protect house secrets. Upgrading to protected custody requires explicit trusted
credential binding/rotation; it is not automatic merely because the user installs
an app. Never import an existing high-value identity seed into the welcome page.

## Welcome experience

Goal: everyone can discover **hello.mesh as the house's local hub**, even if
they decline identity. The invitation should explain a useful destination before
asking for a cryptographic action.

### First connection

Retain the portal's two-action contract: **Create your IdentiKey** links to the hub
and **Just the internet, please** dismisses the invitation. Proposed surrounding copy:

> Welcome to Lightning Mesh.
> hello.mesh is your local hub for people, shared apps, and what's happening here.
> Make an optional identity to introduce yourself. You can explore without one.
> Internet access does not require an identity. Come back at http://hello.mesh/.

The identity CTA lands on the readable hub with the identity panel visible; it
does not enroll a user or hide hub content behind a form. At the hub, **Create an
identity** and **Explore without an identity** are clear choices; returning users
see their current identity without being asked to regenerate it.

Hub content order: house welcome/name → internet and local-service status →
shared services / house information → optional introduction and people → technical
network detail. House notices/links are proposed content, not an existing CMS.
Tell users the actual publication scope before they announce a name. Current
directory replication must not be described as house-private without enforcement.

### Consent, persistence, and return visits

- Browsing is anonymous by default. Creating a key, announcing a name, accepting
  house membership, and receiving admin authority are separate transitions.
- Show a preview of what will be visible and to whom before announcement. No silent
  creation from a MAC address, SSID association, or captive-portal dismissal.
- Guide users to their regular browser for a durable identity. Portal windows may
  have separate/ephemeral storage; don't promise their identity will appear in Safari,
  Chrome, or another node origin. If handoff is unavailable, show the literal URL
  and keep browsing usable. Never transfer private seeds in a handoff URL.
- Expose hello.mesh on the printed Wi-Fi card/router label, a distinct hub URL QR,
  and the desktop “Open house hub” action. A Wi-Fi-join QR and hub URL QR have
  different jobs. QR does not confer trust or admin permission.
- Do not depend on every OS opening a portal. Test actual Apple/Android/Windows
  behavior, DNS/privacy settings, saved networks, and no-internet operation.
- Respect dismissal; don't force a welcome on every roam. Current implementation
  remembers release by IP for 12 hours in node-local memory; that is neither a
  person identity nor a durable fleet-wide preference. Cross-node suppression needs
  a privacy-conscious design and field tests, not MAC tracking as identity.
- If internet is unavailable, explain that the pass-through only dismisses the
  invitation and cannot restore internet. Do not display a newly verified success
  merely because a synthetic OS probe response was sent. Local hub remains usable.

Later standards-based discovery may advertise a venue-information link via
CAPPORT. Its API requires HTTPS, and client presentation varies; it is not an
HTTP workaround. See [RFC 8908](https://www.rfc-editor.org/rfc/rfc8908.html).

## Acceptance additions

Validate a first-time visitor discovers the hub, explores without an identity,
understands what an introduction publishes, and can find the hub after dismissing
the portal. Repeat without internet and with no automatic portal presentation.
Verify admin delegation cannot widen rights, change holder by contradictory
attenuation, convert membership into admin rights, or survive expiry by clock
rollback. These require implementation tests before shipping, not prose approval.
