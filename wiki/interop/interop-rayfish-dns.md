---
title: Rayfish Magic DNS Patterns
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, rayfish, dns, naming]
sources: [public/rayfish/src/dns/mod.rs]
state: [FUTURE]
confidence: medium
---

# Rayfish Magic DNS Patterns

Adopt rayfish's DNS patterns for Lightning Mesh's `.mesh` name resolution.

## Concept

Rayfish has a 39K-line Magic DNS responder (`src/dns/mod.rs`) with `.ray` domain support, DNS-over-TLS forwarding, per-network DNS configs, platform-specific DNS handling (Android Private DNS), and overlay IP filtering. Mesh has a simpler `.mesh` DNS responder.

**Verified from dns/mod.rs:** Magic DNS is reached via `MAGIC_DNS_V6 = 200::53` (IPv6). No host-level port 53 bind is made — `forward::run_mesh` intercepts UDP DNS packets destined for the magic IP and hands them to `resolver::Resolver`. The resolver is reached via the TUN route, not a local socket. `MAGIC_DNS_V4 = 100.100.100.53` is unusable (Tailscale anti-spoof drops it), so the overlay is IPv6-only.

## Rayfish DNS Architecture

Rayfish's DNS handles:

- **`.ray` name resolution** -- maps peer hostnames to overlay IPs
- **Forwarder chain** -- upstream DNS via DoT/TLS, with overlay IP filtering
- **Per-network DNS** -- each network can have its own DNS config
- **Platform handling** -- Android Private DNS, Windows NCSI, macOS mDNS
- **Overlay IP filtering** -- never asks own resolver for overlay addresses
- ** resolv.conf isolation** -- control plane never points at data plane DNS

## Mesh DNS (Current)

Mesh has a `.mesh` DNS responder that resolves names from the ServiceBook and LeasedNameBook CRDTs. It is simpler but functional.

## Patterns to Adopt

### 1. Control Plane / Data Plane DNS Isolation

Rayfish never asks its own Magic DNS for control plane lookups. If a mesh node's resolv.conf points at its own DNS, a restart waits on the data plane it is trying to bring up.

**Rayfish pattern** (`transport.rs` line 159-217):

- Control plane resolver is explicit, never reads host resolv.conf
- Filters overlay addresses from host resolvers
- Falls back to public DNS (Cloudflare, Quad9, Google) when host has none
- Capped at 4 nameservers, keeps both IP families

**Mesh application:** Mesh's iroh endpoint should use an explicit DNS resolver that filters mesh overlay IPs.

### 2. DNS-over-TLS Forwarding

Rayfish forwards unresolved queries to upstream DNS via DoT. Mesh's DNS responder could add DoT forwarding for non-`.mesh` names.

**Mesh application:** When a `.mesh` name is not found in the ServiceBook, forward the query via DoT to upstream DNS.

### 3. Per-Network DNS Configs

Rayfish allows each network to specify its own DNS forwarder. Mesh could use this for multi-network setups where different networks need different DNS (e.g., corporate DNS for work networks).

**Mesh application:** Add `dns_forwarder` to the mesh network config.

### 4. Overlay IP Filtering in DNS

Rayfish filters overlay IPs from DNS responses to prevent loops. Mesh should do the same for `.mesh` addresses that resolve to overlay IPs.

**Mesh application:** When resolving `.mesh` names, filter out any IPs that are in the mesh overlay range (`10.42.0.0/16`).

## Benefits

- Control plane survives host DNS failure
- Non-`.mesh` names resolve via upstream DNS (DoT)
- Per-network DNS configs for multi-operator setups
- Prevents DNS-based routing loops

## Risks

- DoT adds TLS overhead to DNS queries
- Per-network DNS configs add complexity
- 45K lines of rayfish DNS is too large to copy; only patterns should be borrowed

## Effort

Low-Medium. These are pattern-level adoptions, not code copies. DNS-over-TLS forwarding is ~100 lines. Control plane DNS isolation is ~50 lines. Per-network DNS configs are ~50 lines.
