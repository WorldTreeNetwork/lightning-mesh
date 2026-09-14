# Storage nodes

**Status: spec draft.** Nothing here is built as a product yet. This directory
is where we write the software spec for Lightning Mesh storage nodes before
building them. Sections marked **Decision needed** are open.

A **storage node** is a small Linux computer on the mesh that does what a
router can't: hold lots of data, run apps, and (optionally) run local AI
inference. Routers route; storage nodes host.

| Page | What it covers |
|---|---|
| [Hardware](hardware.md) | Reference builds (Raspberry Pi 5 + SSD, Pi 5 + Hailo) and their limits |
| [Software spec](software-spec.md) | Roles, services, how a storage node joins and is managed |
| [Open decisions](decisions.md) | Choices we still have to make, with options |

## Why they exist

- **Routers are too small to host.** Mesh routers have 256 MB–1 GB of RAM and
  flash measured in megabytes. They run the network, DNS and the hello.mesh
  front desk, and nothing heavier.
- **Apps need a home on the mesh.** The first real app, walkie-talkie
  (`walkie-talkie.mesh`), already runs on a Raspberry Pi wired to a mesh
  router. Storage nodes turn that one-off into a supported role.
- **Offline is the point.** When the internet is gone, what the mesh can still
  do depends on what's stored and served on the mesh itself.

## Principles the spec must keep

These come from the project as a whole and aren't up for renegotiation here:

1. **No implicit authority.** A storage node is a peer. The mesh keeps working
   without any storage node, and no storage node is "the server".
2. **Exit test.** Anyone storing data on a storage node must be able to take
   their keys and data with them and leave, without asking permission.
3. **Private keys never live on a node.** A storage node may hold encrypted
   user data. It never holds a user's private key in the clear.
4. **Same management plane as routers.** Reached over the `10.254.x` overlay
   and managed with the same signed, UI-driven control plane as routers. No
   SSH-only workflows.

## Related

- [Join Lightning Mesh](../join/index.md): the user-facing guide
- [Publish a service](../join/publish/01-publish-a-service.md): how apps get `.mesh` names today
- [Node operations](../deploy/node-operations.md): management plane design for routers
