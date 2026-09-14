# Storage node: open decisions

**Status: open.** Each decision lists options and, where we have one, a
proposed default. Record the outcome here, then link a bead.

The architecture proposal that covers D2–D4 is
[Secure context, certificates, and an SSH-free control plane](../network-coordination/secure-context-and-control-plane.md).
It was informed by the [Astra-6 consult](consults/2026-09-13-astra-6-secure-context.md).

## D1: One node class or two

Is local AI (Hailo) an optional add-on to a storage node, or a separate node
class?

| Option | For | Against |
|---|---|---|
| One class, optional accelerator | One image, one management story | The Pi 5's single PCIe lane makes SSD plus Hailo awkward |
| Two classes (storage, inference) | Each build is simple and fast | Two images and two sets of docs |

Proposed default: one software image with the inference role enabled only when
a Hailo device is detected. Keep hardware builds separate.

## D2: What a storage node mirrors

**Proposed:** CRDT metadata plus signed content-hash pointers plus opt-in
`iroh-blobs` pulls. Mirroring needs publisher permission, storage-owner quotas
and explicit pins.

| Candidate | Mirror? | Notes |
|---|---|---|
| App static bundles and `/.well-known/mesh-app.json` manifests | Yes, opt-in | Verified by content hash. A replica serves under its **own** origin; copying bytes never grants another app's origin |
| Directory snapshots | Yes | Timestamped. Must never resurrect expired services |
| Users' client-encrypted backups | Yes, opt-in | Ciphertext only. The user keeps a full export; deletion across replicas can't be guaranteed |
| App databases / live state | No | Needs app-level replication design |
| Private keys, CA or DNS credentials, sessions, plaintext private data | **Never** | |
| Arbitrary executable workloads | **Never** automatically | Installing software needs the node owner's capability |

## D3: Control plane without SSH

**Proposed:** one typed admin protocol over authenticated iroh connections.
- Lightning Admin calls it directly.
- mjolnir-hello forwards signed envelopes over HTTP through any router.
- The destination node verifies authorization itself.

Authorization:

| Operation | Authorized by | Signer |
|---|---|---|
| Publish, renew or unpublish your own name | Name owner key | Browser key is enough |
| Node-scoped actions (device names, hosting, storage, radio, installs, ownership transfer) | Node owner capability | Installed signer (Lightning Admin first) |

Node ownership is bootstrapped by physical pairing (a WPS-button window), never
by first gossip claim. Prerequisite: owner-signed name records verified by every
consumer.

Awaiting Duke's decision. Bead `mjolnir-mesh-b6j.2`.

## D4: HTTPS and certificates

**Proposed:**
- Keep `http://hello.mesh` as walk-up.
- Add key-qualified HTTPS origins under a delegated domain (bring-your-own
  supported), answered locally by mesh DNS.
- Issue certificates with ACME DNS-01 through a replaceable DNS adapter. An
  owner-signed issuance authorization approves each one. TLS keys stay on each
  host, with no shared or wildcard keys.
- Never reuse a security origin across owners.
- Use the `classic` 90-day profile with opportunistic renewal and an
  offline-validity display.
- Hard custody comes from an installed signer, not from HTTPS or Service
  Workers.
- A private per-mesh CA is for managed devices only.

Awaiting Duke's decision. Bead `mjolnir-mesh-b6j.1` (supersedes the direct
lease-to-certificate idea in `mjolnir-mesh-3fg`).
