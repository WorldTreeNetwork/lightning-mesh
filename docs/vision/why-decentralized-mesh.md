# Why Decentralized Mesh Networking

## The Problem

Today's networks are centralized by default. Your home has one router. Your office has managed switches and a network admin. Events rely on a single AP or a vendor-locked mesh system (Ubiquiti, eero, Google WiFi). If the central device fails, the network fails.

This centralization is:
- **Fragile**: Single point of failure
- **Inflexible**: Can't add capacity by just plugging in another router
- **Vendor-locked**: Mesh systems only work with their own hardware
- **Opaque**: You don't control the software, the routing, the DNS

For events like DWEB (Decentralized Web), the irony is sharp: a movement about decentralization runs on centralized infrastructure.

## The Vision

**Any router can join the mesh. Any router can leave. The network keeps working.**

mjolnir-mesh turns commodity OpenWrt routers (like $60 GL.iNet travel routers) into nodes in a self-organizing mesh network. Plug in a router, it joins. Unplug it, the mesh adapts. No configuration. No central controller. No vendor lock-in.

What this enables:

**1. Pop-up networks for events**
DWEB conference, 200+ attendees. Organizers bring 10 routers, plug them in around the venue. Within seconds, a unified network forms. Same SSID, shared IP space, devices roam seamlessly as people move between rooms. Anyone can bring an extra router to boost coverage in a corner — just plug it in.

**2. Resilient home networks**
Your main router, a travel router, maybe a third for the garage. All coordinated. If your main router dies, the others keep serving. When you take the travel router on a trip, the home mesh shrinks gracefully. When you come back, it rejoins and syncs.

**3. Community networks**
A neighborhood mesh where each household runs a node. Shared local services — file servers, wikis, game servers — discoverable by hostname. Global connectivity via Iroh when nodes have internet access, local-only operation when they don't.

**4. Global roaming**
Your travel router connects to your home mesh via Iroh from anywhere in the world. Devices on your travel router can reach services at home by hostname. Your home devices can reach your travel network. One mesh, spanning the globe, encrypted end-to-end.

## Why Now

Three things that didn't exist 5 years ago make this feasible:

**Iroh (QUIC mesh networking)**: NAT traversal, encrypted connections, peer discovery, relay fallback — all built in. Previously you'd need a VPN server, manual config, port forwarding. Iroh makes global P2P mesh networking as easy as connecting to a server.

**Affordable OpenWrt hardware**: GL.iNet routers ($30-80) run full Linux with 128MB+ RAM, USB, WiFi 6/7. Powerful enough to run a Rust mesh daemon alongside dnsmasq. Available worldwide. No vendor lock-in.

**CRDTs**: Conflict-free replicated data types give us eventually-consistent shared state without consensus protocols. No leader election, no Raft, no Paxos. Just merge. Perfect for a P2P network where nodes come and go.

## What Makes This Different

**vs. Traditional mesh WiFi (eero, Ubiquiti, Google WiFi)**:
- Those systems have a "controller" node. mjolnir-mesh doesn't.
- Those systems only work with their own hardware. mjolnir-mesh works with any OpenWrt device.
- Those systems don't cross the internet. mjolnir-mesh does, via Iroh.

**vs. VPNs (WireGuard, Tailscale)**:
- VPNs are point-to-point or hub-and-spoke. mjolnir-mesh is full mesh.
- VPNs don't coordinate DHCP or DNS across nodes. mjolnir-mesh does.
- VPNs require manual peer configuration. mjolnir-mesh self-organizes.
- Tailscale is close in spirit but requires their coordination server. mjolnir-mesh is fully self-hosted.

**vs. Mesh networking protocols (B.A.T.M.A.N., OLSR, babel)**:
- Those operate at Layer 2/3 — they route packets between nodes but don't coordinate network services.
- mjolnir-mesh coordinates DHCP, DNS, service discovery, and routing as a unified system.
- Those protocols are designed for ad-hoc wireless links. mjolnir-mesh works over any transport Iroh supports (direct, relayed, internet).

## The Architecture in Brief

```
┌─────────────────────────────────────────┐
│  Applications & Devices                  │
│  (standard TCP/IP — no changes needed)   │
├─────────────────────────────────────────┤
│  dnsmasq (DHCP + DNS per router)         │
│  Serves local devices, reads mesh state  │
├─────────────────────────────────────────┤
│  mjolnir-mesh daemon                     │
│  CRDT store ←→ gossip replication        │
│  Hostsfile sync, conflict resolution     │
│  Service discovery, route management     │
├─────────────────────────────────────────┤
│  Iroh node (QUIC mesh)                   │
│  NAT traversal, encryption, tunneling    │
└─────────────────────────────────────────┘
```

Every router runs this stack. No special roles. No leaders. Fully symmetric.

## The Power of Service Discovery

Beyond basic networking, the mesh becomes a platform for services:

- A Raspberry Pi running a wiki joins the mesh → `wiki.mesh` is resolvable from any device on any router
- Someone starts a game server → `minecraft.mesh:25565` appears on every router's DNS
- A projector with a web interface → `projector.mesh` accessible from any phone in the venue
- Mjolnir VMs join the mesh too → spin up a service in a VM, it's instantly discoverable

This turns a group of routers into a **decentralized application platform**, not just a network.

## Relationship to Mjolnir

mjolnir-mesh is part of the Mjolnir ecosystem. Mjolnir provides:
- **MicroVMs** with Iroh built into their network stack
- **MCP (Model Context Protocol)** for AI agent interaction
- **BTRFS snapshots** for instant VM cloning

The mesh layer means VMs can be spawned on any node and be instantly reachable by any device on the mesh. A Mjolnir cluster distributed across mesh routers becomes a decentralized compute platform — VMs migrate, routers come and go, the mesh adapts.

## Who This Is For

- **Event organizers** who need reliable, flexible networking without enterprise infrastructure
- **Decentralization advocates** who want their network to match their values
- **Home labbers** who want seamless multi-router setups
- **Community network builders** working on local mesh infrastructure
- **Developers** building P2P applications who need a networking layer that "just works"

## Current Status

mjolnir-mesh is in active development. The Iroh-based mesh node exists (`crates/mjolnir-node`), the MoQ media layer is scaffolded (`crates/mjolnir-moq`), and the CRDT/DHCP/DNS coordination layer is designed and documented. Next: implementation of the CRDT store and dnsmasq integration.

## References
- Technical overview: docs/mesh-network-coordination.md
- CRDT design: docs/architecture/dhcp-crdt.md
- Network architecture: docs/architecture/network-architecture.md
- dnsmasq integration: docs/architecture/dnsmasq-integration.md