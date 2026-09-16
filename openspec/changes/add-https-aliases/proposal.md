# add-https-aliases

> **ACTIVE BUILD**

Bead `mjolnir-mesh-b6j.1` (epic `mjolnir-mesh-b6j`). Direction from steer
2026-09-15 (Duke), recorded in
[`docs/network-coordination/secure-context-and-control-plane.md`](../../../docs/network-coordination/secure-context-and-control-plane.md)
§ Decisions. It builds on the Astra-6 consult. Trust and ownership come from
the household campaign `ai0`
([`add-household-trust-contract`](../add-household-trust-contract/proposal.md)).

## Why

Everything on Lightning Mesh is plain HTTP (`http://hello.mesh`) or HTTPS with a
self-signed certificate (walkie-talkie). Browsers treat those pages as insecure
and withhold:
- non-extractable WebCrypto keys and Service Workers
- camera, microphone and location
- app install

People also hit certificate warnings they have to click through. `.mesh` has no
public DNS delegation, so no public CA will issue for it. The mesh needs real,
trusted HTTPS names that:
- resolve **offline** from mesh DNS
- keep each host's TLS private key on that host
- follow key ownership rather than an operator
- never let one owner inherit another's browser origin

## What

- **Delegated zone.** `mesh.worldtree.network` is the project default. A mesh
  may configure its own domain instead. Plain `http://hello.mesh` stays as the
  walk-up front door.
- **Key-qualified origins:**
  - apps: `https://a-<app-key-label>.<mesh-label>.<zone>`
  - router front desks: `https://n-<node-key-label>.<mesh-label>.<zone>`

  Labels derive from the owning public key, so a new owner always gets a new
  hostname and origins are never recycled. `.mesh` names stay friendly
  discovery aliases.
- **Local resolution.** meshd forwards only the configured zone to its own
  responder, and the rebind-protection exception covers only that zone. Hosts
  are answered with mesh addresses from owner-signed records.
- **Certificates.**
  - ACME DNS-01 through a **replaceable DNS adapter** for the zone.
  - The adapter writes a challenge record only for an **owner-signed issuance
    authorization** covering the exact FQDN, TXT digest, ACME account, CSR
    public-key hash, nonce and expiry.
  - Private keys never leave the app host or router.
- **Renewal and honesty.**
  - Let's Encrypt `classic` profile, renewed opportunistically through any
    gateway with internet, using ARI.
  - hello.mesh shows each HTTPS name's remaining offline validity.
  - No 6-day profile.
- **Router front desks.** mjolnir-hello terminates TLS for its own key-qualified
  origin (in the same phase as apps, per steer). Walk-up anycast
  `http://hello.mesh` is unchanged.

## Impact

- Capabilities: ADDED `mesh-https-names`.
- ADR: `design.md` in this change.
- Depends on:
  - `add-household-trust-contract` (house identity for the mesh label, node
    identity, owner keys)
  - owner-signed name records verified by every consumer (`ai0.9` for admin
    records; resolution records need the same guarantee)
  - `add-signed-node-control` (`b6j.2`) for configuring the zone without SSH
- New external dependency: an authoritative DNS adapter for
  `mesh.worldtree.network` reachable by the public ACME CA. It's a convenience
  service, and meshes can replace it or bring their own domain.

## User journey & surfaces

A visitor on the house mesh opens hello.mesh and taps a Services link for an app
hosted on the storage node.

1. **Working.**
   - The link is `https://a-<label>.<mesh-label>.mesh.worldtree.network`. It
     loads with a valid padlock, no warning, and camera and microphone prompts
     work.
   - The router's own front desk is reachable at its `https://n-<label>…`
     origin.
   - hello.mesh shows "HTTPS valid for 71 more days offline" beside each name.
2. **Empty.** A mesh with no configured zone (or no internet ever) shows plain
   `.mesh` links only, as today.
3. **Failed.**
   - Issuance or renewal fails: the service stays reachable over its existing
     path, hello.mesh shows "certificate renewal failing" with the last error,
     and the owner sees it in Lightning Admin.
   - A certificate expires while offline: hello.mesh labels the name "HTTPS
     expired — reconnect a gateway to renew" instead of letting browsers
     silently warn.
4. **Off.** Off the mesh, the key-qualified names don't resolve to anything
   useful. They're local-only by design.

## Out of scope

- Hard custody, the signer and owner claim: `ai0` (`bf7.1`, `ai0.1`, `b6j.2`).
- Owner-signed resolution record format itself: a sibling change after `ai0.9`.
- Private per-mesh CA (managed devices only): later, if ever.
- Moving identity storage to an HTTPS origin: a later identity change.
- Secure embedding of mini-apps inside hello.mesh: waits on hello having a
  secure origin (`ncy`).
- Storage-node mirroring: `b6j` D2.
