# hello.mesh — Demo Runbook (S8 / 2uq on-fleet validation)

**For:** the two remaining sprint-001 stories that need real hardware —
`dat` (S8, e2e acceptance) and `2uq` (field-validate `/users` + service gossip
on the physical 802.11s fleet). All code is complete, verified natively and
in-container; this is the procedure to validate it on a live two-node fleet and
rehearse the DWeb demo.

## Prereqs

- Two (or more) OpenWrt mt76 nodes on an 802.11s backhaul, running
  `mjolnir-meshd` (the deployed data plane). Fully offline is the target case.
- Docker on the build host (for the cross-builds).

## 1. Build the artifacts

```bash
# Daemon (already how you ship it)
deploy/openwrt/build.sh                    # -> deploy/openwrt/mjolnir-meshd-aarch64

# Front desk: builds the SSG frontend, embeds it, then cross-builds the server.
# The frontend embed MUST happen before the Rust build (rust-embed) — build-hello.sh
# does this in order (bun run build:embed → cargo cross-build).
deploy/openwrt/build-hello.sh              # -> deploy/openwrt/mjolnir-hello-aarch64
```

## 2. Deploy to the fleet

```bash
deploy/openwrt/install-node.sh <node>      # stages meshd + mjolnir-hello + configs,
                                           # enables the procd services, health-gated
```
Confirm on each node: `service mjolnir-hello status`, and that
`/etc/config/mjolnir` has the hello section (bind = LAN-IP:80, `directory_file`,
`spool_dir`).

## 3. S8 acceptance — the demo flow (fully offline)

Run with no internet on the mesh. Let A and B be two different nodes.

**A. Directory is live and mesh-wide**
- On a phone on node A's AP, open `http://<A-lan-ip>/` (or `http://hello.mesh`
  once `e21.1` DNS lands). Page loads in **< 2s** (NFR1).
- The directory shows *both* nodes as neighbors (mesh-wide, from the gossiped
  address book), not just the local one.
- Cross-check from a shell: `curl http://<A-lan-ip>/api/directory` and
  `curl http://<B-lan-ip>/api/directory` — both list both nodes.

**B. A friend comes online across the mesh** (the headline)
- On the phone at node A: tap **"Create an identity"** (rung-1 soft key).
- Within **~15s**, that identity appears in the directory served by node **B**
  (`curl http://<B-lan-ip>/api/directory` → the new entry under `identities`).
  Path: browser → `POST /api/identity` (A) → spool → `p6u` ingest → `/users`
  gossip → converges to B → B's `directory.json` → B's `/api/directory`.

**C. A remote service is found**
- Publish a service at node A (however services are announced on the fleet —
  the `ServiceEntry` gossip path, story `7jb`).
- It appears in node B's directory `services` within ~15s.

**D. Plug in a third node**
- Join a third router; it appears in every node's directory within seconds
  (the neighbor/address-book path).

Pass = A–D all observed in one offline run. Cross-*site* over iroh is a stretch
(Growth `FR29`), not required for this gate.

## 4. 2uq — field-validate gossip on real radio

The spike proved `/users` convergence over an in-process transport; `bc7.1`
already fleet-validated `/users` 4-node. `2uq` extends that to **services** and
to the **spool-ingest** path on real 802.11s: confirm a `ServiceEntry` and a
spool-created identity both converge across the radio backhaul within target
latency, survive node churn, and heal via anti-entropy after a missed update.

## Success metrics (from the PRD)

| Metric | Target |
|---|---|
| Page load (mt7986-class, local WiFi) | < 2s |
| Cross-node propagation (identity/service, over 802.11s) | < 15s |
| Concurrent browser clients per node | ≥ 50, no 5xx |
| Offline operation | 100% of the above with no internet |

## Notes / known integration points

- **DNS**: `hello.mesh` name resolution is the separate `e21.1` track. Until it
  lands, use the node LAN IP (the front desk binds there regardless — `FR18`).
- **Fallback filename**: the SSG SPA fallback is currently `index.html`, which
  `build:embed` writes into the embed dir; `build-hello.sh` guards on its
  presence. If the committed placeholder vs. built-index churn is annoying,
  switch the adapter `fallback` to `200.html`.
- **Identity→username mapping**: `p6u` maps a spooled submission to a `UserEntry`
  keyed by pubkey with `display_name` = label; revisit if the directory should
  show something friendlier.
