# Storage node: open decisions

**Status: open.** Each decision lists options and, where we have one, a
proposed default. Record the outcome here, then link a bead.

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

Should storage nodes keep local copies of published services and apps, and of
what?

| Candidate | Mirror? | Notes |
|---|---|---|
| App static bundles and `/.well-known/mesh-app.json` manifests | Candidate | Keeps apps loadable when the origin host is down. Needs integrity (content hash) so a mirror can't alter an app |
| Directory snapshots | Candidate | Lets a rejoining site catch up quickly |
| Users' encrypted backups | Candidate | Only ciphertext; owner holds the key |
| App databases / live state | Probably not | Belongs to the app; mirroring live state needs app-level replication |
| Anyone's private keys | **Never** | |

Open: the replication primitive (content-addressed blobs over iroh, or CRDT
pointers plus pull-on-demand). Pending the architecture consult in D3 and D4.

## D3: Control plane without SSH

Publishing (`mjolnir-meshd publish`) and node admin currently require SSH to a
router, because the control API only listens on `127.0.0.1`. We're replacing
that with a UI path in Lightning Admin and/or hello.mesh.

Open questions:
- Where authorization lives: per-node owner keys, name-owner keys, or both.
- Transport: signed HTTP via any router's hello service, proxied over the
  overlay, or iroh RPC.
- Which operations require a hard-custody key (desktop app or extension) and
  which a browser-held key may do.

Architecture consult in progress (2026-09-13). Record the recommendation here.

## D4: HTTPS and certificates

`.mesh` isn't a public suffix, so apps and hello.mesh can't get certificates
browsers trust. Without a secure context, browsers withhold WebCrypto key
protection, camera and microphone, service workers and app install.

Options under consideration:
- Public-suffix names under a project domain with real certificates obtained
  by DNS challenge (bead `mjolnir-mesh-3fg`), with certificate keys held by the
  app host.
- A per-mesh private certificate authority installed on devices.
- Keep plain HTTP for walk-up use, and make a native app or browser extension
  the secure path for strong keys.

Architecture consult in progress (2026-09-13). Record the recommendation here.
