# Radio Backhaul & Multi-Hop Discovery — Design Notes & Decisions

Status: living design note. Captures the 2026-06-23 hardware/radio decision, the
tradeoffs behind it, and the protocol work it implies. See beads
`mjolnir-mesh-b1d` (wireless backhaul) and the multi-hop-discovery bead.

## TL;DR

- **Decision: stay on the MikroTik hardware (RouterOS / `wifi-qcom`).** The project
  principle that matters is **protocol symmetry** (a non-authoritative, symmetric
  P2P protocol), which lives at **L3** (iroh + babeld + CRDT) and is satisfied.
  Literal **L2 radio symmetry** (802.11s / IBSS) is *not* required and is *not*
  available on this hardware.
- **Radio layer: identical-config AP/STA nodes.** Every node runs the same
  software (AP on one band, station on the other / same band). No designated root,
  no single point of failure. Which end of a link is "AP" falls out of radio range,
  not hierarchy — transport plumbing, like TCP's `connect()`/`accept()`.
- **Next protocol work: multi-hop discovery** (the "babeld/mDNS synchronization"
  problem). Everything validated so far assumes one shared L2 segment; the forest
  doesn't have that. See the last section.

## Background: the two layers

| Layer | Role | Symmetry requirement |
|---|---|---|
| **L3 overlay** (iroh + babeld + CRDT) | the actual protocol: addressing, routing, self-healing, multi-hop | **Must be symmetric / non-authoritative** — this is the thesis |
| **Radio backhaul** (WiFi) | transport: carry IP between in-range neighbours | Just needs to provide links; AP/STA labelling is below the protocol |

The contribution is the L3 protocol. The radio is plumbing. Keeping these straight
is what resolves the "designated root violates our principles" tension: a *root AP*
(one node everyone depends on) would be a protocol authority and a single point of
failure — unacceptable. A *per-link AP/STA role* on otherwise-identical nodes is
neither.

## Radio-layer findings (`wifi-qcom`, IPQ-5010 / L23UGSR), 2026-06-23

Sourced from current RouterOS docs (see `mjolnir-mesh-b1d` notes for links):

| Capability | Available? |
|---|---|
| 802.11s / HWMPplus mesh | **No** (docs: "not supported on WiFi interfaces") |
| Legacy `wireless` package on this 802.11ax SoC | **No** (conflicts with `wifi-qcom`, can't run) |
| IBSS / ad-hoc | **No** (not in the `wifi-qcom` mode set) |
| AP, station, station-bridge, station-pseudobridge | **Yes** |
| Concurrent AP + station on one radio | **Yes**, but channel-locked to the AP VIF |
| Station auto-reassoc among same-SSID APs | **Yes** (signal-based; topology-blind) |

**OpenWrt is not a shortcut.** It's the wall the project already hit: no board
port exists for the L23UGSR, MikroTik uses RouterBOOT (brick risk, not U-Boot),
`ath11k` support for the IPQ-5010 + QCN-6102 is incomplete upstream, and even if
it booted, `ath11k` mesh support for these chips is uncertain and IBSS is absent.
High effort, real brick risk, uncertain payoff. Not recommended.

## Channels & radios (the recurring question)

**A "true mesh" does not give you more channels.** Any *single-radio* mesh —
802.11s or AP/STA — is inherently **single-channel**: all peers must share one
frequency to hear each other. What 802.11s adds is symmetric peering + native
multi-hop path selection, not channel diversity.

| Topology | Channels | Notes |
|---|---|---|
| One radio, concurrent AP+STA | 1 (shared by all backhaul + any clients) | simplest; airtime contention grows with nodes/hops |
| Dual radio: dedicated backhaul + client | 1 backhaul + 1 client | **practical sweet spot** on the L23UGSR; isolates backhaul airtime from clients; still one backhaul channel mesh-wide |
| Multi-radio backhaul + channel planning | N | true frequency reuse; complex/expensive; only for large/high-throughput meshes |

So: **backhaul on one radio = one channel, period.** The dual-radio split's value
is isolating client traffic from the backhaul, not multiplying backhaul channels.

## Mixed fleet / interop (planned: real-mesh nodes + AP/STA)

- **L3 (the protocol): interoperates with anything.** It's a Linux process over
  IP — runs identically on a RouterOS container, OpenWrt, a Raspberry Pi, a laptop.
  A MikroTik node and an OpenWrt node in the same mesh interoperate perfectly here.
- **L2 (radio): only same-mode interoperates.** Plain **AP + managed-station is
  standard WiFi and interoperates across vendors**; **802.11s ↔ `wifi-qcom` does
  not**. Rule: don't mix radio *modes* within one radio domain.
- **Hybrid plan** (some open routers running real 802.11s, bridged into the AP/STA
  MikroTik fleet): workable, but needs a **gateway node** that participates in both
  radio domains (e.g. an OpenWrt node doing 802.11s on one radio and AP/STA on
  another, bridging them). The L3 overlay spans the whole fleet regardless.

## Open-source narrative (for the talk)

The protocol is open and containerised; the host is irrelevant. Turn the
closed-driver MikroTik from a wart into a **portability demonstration**: run one
node on an open board (RPi + USB WiFi, or an OpenWrt SBC) in the same AP/STA mesh.
The demo becomes *"a symmetric, non-authoritative protocol, in a container, running
identically across a closed-driver MikroTik and an open Linux node, interoperating
in one mesh."* The closed Qualcomm driver lives below the abstraction.

## Hardware / packaging

- The **NetMetal-ax enclosure** variant is already a weatherproof single unit —
  good for "3D-print a case and throw it in the forest." A *separate* mesh radio
  would break the single-unit goal; a point in favour of the integrated MikroTik.
- MikroTiks are sunk cost (past the return window) — kept, not wasted, since
  protocol symmetry is satisfied on them as-is.
- For forest distance: **2.4 GHz omni** is the better backhaul (range + foliage
  penetration) over 5 GHz; omni is mandatory for a mesh (can't aim a directional at
  all neighbours). Reserve 5 GHz + higher-gain antennas for any fixed long hops.

## Forward: multi-hop discovery — the babeld/mDNS synchronization work

**This is the next real protocol problem, and it's platform-independent.**

Everything validated so far (containers, the switch bench) sits on **one shared L2
segment**: mDNS floods to every node, so every node discovers every other directly,
and iroh dials anyone. **A spread-out forest has no such shared segment.** Nodes
that aren't radio-neighbours aren't on a common L2, so:

- **Flat mDNS only reaches direct neighbours.** Node A discovers B and C (in range),
  not distant D.
- **babeld routes *traffic* multi-hop fine** (A→B→C→D at the IP layer), but A
  *learning how to address* D — the address iroh needs to dial — is unsolved by
  mDNS alone.

**Likely direction:** stop relying on flat mDNS for mesh-wide discovery. Use the
**roster** (known node ids) plus **propagate peer addresses over the CRDT gossip
overlay**, which itself rides the babeld-routed IP network. mDNS stays for
direct-neighbour bootstrap; the gossip layer becomes a mesh-wide, eventually-
consistent **address book** (node id → reachable address), synchronized the same
way subnet claims already are. That synchronization between the link-local
discovery (mDNS) and the routed overlay (babeld + gossip) is the "babeld/mDNS
synchronization protocol" to design and build.

This is squarely the kind of non-authoritative, eventually-consistent, symmetric
mechanism the project is about — a good problem, not a blocker.

## Status & next steps

- **Validated:** L3 mesh + derived-IPv4 backhaul + direct iroh tunnels + mDNS
  discovery, end-to-end on a single L2 segment (armv7 Linux containers).
- **Next concrete step:** `mjolnir-mesh-2j6` — 4-node mesh on the wired switch
  (real babeld + client data path). Hardware-agnostic; proves the protocol on the
  known-good shared segment. Precursor to multi-hop discovery.
- **Then:** multi-hop discovery (babeld/mDNS synchronization) — its own bead.
- **Radio:** AP/STA identical-node baseline on the MikroTiks; experiment with
  real-mesh open nodes (hybrid) in parallel; reconcile via L3.
