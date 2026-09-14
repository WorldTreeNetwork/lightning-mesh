# Storage node software spec

**Status: spec draft.** This is the skeleton we'll fill in together. Each
section states what's settled, what's proposed, and what's open. Open items
link to [decisions](decisions.md).

## 1. Roles

A storage node can take any of these roles. Roles are independent; a small
site might run all of them on one Pi.

| Role | What it does | Status |
|---|---|---|
| **App host** | Runs apps (like walkie-talkie) and publishes them with key-owned `.mesh` names | Exists informally (walkie-talkie on a Pi) |
| **Mirror** | Keeps copies of published app bundles, manifests and directory snapshots so they stay available offline or when their origin host is down | Proposed. See [D2](decisions.md#d2-what-a-storage-node-mirrors) |
| **Backup vault** | Stores users' **encrypted** backups; the node never sees the keys | Proposed |
| **Inference** | Local AI on the Hailo accelerator, offered as a mesh service | Proposed. See [D1](decisions.md#d1-one-node-class-or-two) |
| **Control-plane helper** | Serves the heavier parts of the admin and publish UI that routers are too small for | Proposed. See [D3](decisions.md#d3-control-plane-without-ssh) |

## 2. Operating system and base

- **Proposed:** Raspberry Pi OS Lite (64-bit) or Debian arm64, headless.
- **Proposed:** read-mostly root with app data on the SSD, so a power cut
  doesn't corrupt the system.
- **Open:** image-based updates (A/B root) versus package updates. Mesh routers
  already use staged apply with health-gated rollback, and storage nodes should
  match that behaviour.

## 3. Joining the mesh

- **Settled:** a storage node is wired to a router's LAN port. It gets a client
  address in that router's `10.42.x.0/24`.
- **Proposed:** it runs the mesh daemon in a host role, with no radio and no
  subnet claim. It gets a node identity (`/etc/mjolnir/secret`), a derived
  `10.254.x` overlay address, and takes part in gossip. That makes it reachable
  and manageable like a router.
- **Open:** whether the host role is a mode of `mjolnir-meshd` or a separate
  lighter daemon.

## 4. Identity and trust

- **Settled:** users' private keys never live on a storage node in the clear
  (exit test, and the custody rules in
  [user identity](../network-coordination/user-identity.md)).
- **Proposed:** each node has an **owners** claim, the set of operator keys
  allowed to administer it. It's the same model as the router fleet-admin
  design (bead `mjolnir-mesh-bf7`).
- **Open:** how apps on a storage node get TLS certificates. See
  [D4](decisions.md#d4-https-and-certificates).

## 5. Services a storage node offers

| Service | Interface | Status |
|---|---|---|
| App hosting runtime | Containers or systemd units per app | Open |
| `.mesh` name publishing | Key-owned name lease (`/api/challenge` + `/api/name-claim` on any router) | Protocol built; client library not yet |
| Mirror / blob store | Content-addressed | Proposed |
| Encrypted backup | Per-user, client-side encrypted | Proposed |
| Inference API | HTTP on a `.mesh` name | Proposed |

## 6. Management

- **Settled direction:** no SSH-only workflows. Publishing, app install, and
  admin happen from the Lightning Admin desktop app or the hello.mesh web app,
  authorized by signed requests. See [D3](decisions.md#d3-control-plane-without-ssh).
- **Proposed:** health endpoint and metrics on the overlay address, visible in
  hello.mesh's Routers view as a distinct node type.

## 7. Data and exit

- **Settled:** any user data on a storage node must be exportable by its owner
  in a standard format, and deletable.
- **Open:** retention, quotas, and what happens to mirrored data when the
  original publisher's name lease expires.

## 8. Out of scope for the first version

- Acting as an internet gateway (routers already do this)
- Replacing routers for radio or DHCP
- Multi-site replication over the internet (after local mirroring works)
