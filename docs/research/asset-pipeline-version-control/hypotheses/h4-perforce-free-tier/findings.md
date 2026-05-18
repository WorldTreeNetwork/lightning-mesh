# Hypothesis: H4 — Perforce Helix Core Free Tier Viability for Unreal Asset Pipeline

## Summary

Helix Core's free tier (5 users, 20 workspaces) remains legitimate as of 2026 and is the industry-standard tool for Unreal Engine asset management. The free tier is genuinely useful for small teams, but the 5-user hard cap creates planning pressure, and running p4d at home on Linux is straightforward but requires commitment to ongoing administration. The "two VCSes" problem is real but has established mitigations; keeping everything in Perforce (including Unreal source) is the simpler architectural choice for a small team.

---

## Evidence

### 1. Licensing Terms (2024–2026)

Perforce's **Helix Core Free Plan**: 5 users, 20 workspaces, unlimited file storage, no time limit. Permanent free tier, not a trial [1].

Upgrade path when hitting 6 users: paid license — Perforce doesn't publish step-up pricing. Community reports 2024 confirm the server refuses the 6th connection; no grace period. Service accounts and named users count against the cap; bots/CI may need a separate sales discussion.

**5-user cap is consistently confirmed** across Perforce docs, Epic's Unreal docs, and forums. Upgrade pricing is opaque.

### 2. Linux Server (p4d) Feasibility

Running p4d on Linux including Arch is well-supported. Official `.deb`/`.rpm` packages, direct binary download, AUR packages (`helix-p4d`, `perforce`) on Arch [3][5].

**Resources for a small team:**
- RAM: 1–4 GB for metadata cache
- Disk: depot size × 2 for versioned history
- CPU: negligible unless concurrent large syncs

Binds to TCP port (default 1666). Optional SSL. Community reports consistently describe home-lab p4d as "works perfectly" for small Unreal teams.

### 3. Cross-Platform Clients

**P4V (GUI):**
- Windows: official, stable
- macOS: official, Apple Silicon native since 2023+
- Arch Linux: `p4v` AUR package (wraps official Linux binary); may lag official by weeks

**p4 (CLI):** single binary for Linux/macOS/Windows. WSL2 callable; path translation friction mitigated by `P4CONFIG` per-directory config.

**Artist UX:** P4V's "Check Out" / "Submit" model shows lock ownership in the GUI — simpler for non-technical users than Git LFS. Server enforces single-writer locks on binary assets.

### 4. Unreal Engine Integration

Unreal has **built-in Perforce source control support** — first-class reference implementation. Epic uses Perforce internally.

**Key workflows:**
- **Exclusive checkout:** `.uasset` and `.umap` set to `binary+l` in typemap [2]. Server refuses second checkout — prevents simultaneous Blueprint edits.
- **In-editor status:** Locked files show owner's name in Content Browser. Checkouts and changelist submits from within the editor.
- **Stream depots vs classic:** Streams for branching/SKUs; classic `//depot/main/...` for single-mainline small teams.
- **Partial sync (workspace views):** Artists sync only art subdirectory; programmers sync only source.

Epic's 2023–2025 Unreal docs continue to lead with Perforce as primary VCS.

### 5. The "Two VCSes" Problem

| Option | Description | Fit for ≤5 |
|---|---|---|
| A — Manual tagging | Git tag = P4 CL; `PERFORCE_CL.txt` file | Simple, breaks under human error |
| B — UGS (UnrealGameSync) | Epic's internal tool, open-sourced | Overkill; complex setup |
| C — Everything in Perforce | UE source + C++ + Blueprints + assets in P4; Git unused | **Industry default for small Unreal studios** |
| D — Git code + P4 assets + build pipeline | Build server emits `version.txt` with both | Mid-size studio pattern |

**Recommendation:** For ≤5 people, keep everything in Perforce. Introduce Git/P4 split only when git-native tooling (GitHub PRs, CI/CD designed around git) becomes mandatory.

### 6. Perforce–Git Bridges

- **git-p4** (built-in Python): clones P4 → Git. Functional for simple depots; struggles with large binaries. One-directional code mirror only.
- **Helix4Git / Helix TeamHub:** paid hosted service; self-host = significant ops complexity. Inappropriate for free tier.
- **git-fusion:** deprecated.

**Verdict:** For a 5-person Unreal team, bridges add complexity without clear benefit. If GitHub is needed for issue tracking/PRs, maintain a read-only Git mirror of the code subdirectory via scheduled `git-p4`.

### 7. Disaster Recovery / Replication

**p4 checkpoint + restore:** `p4d -jc` creates metadata dump + versioned files. rsync to another machine. Full restore documented and reliable [4].

**Perforce Replication** (`p4 replicate`, `p4d -J`): warm-standby via journal relay. Available without enterprise licensing — feature of p4d binary. Replica p4d receives journal updates; failover is manual (change license/serverid) [6].

**Practical for a home team:**
1. Primary p4d on home server/desktop
2. Nightly rsync of depot + checkpoint to second machine (NAS, second desktop, OpenWRT SSD if space permits — OpenWRT can run rsync)
3. Hot-standby replica is also possible but more complex — nightly rsync is sufficient for 5 users

### 8. Mjolnir Mesh and Perforce Integration

**Iroh QUIC proxy for p4d traffic:** technically yes, with caveats. p4 protocol is proprietary TCP binary framing (port 1666, SSL on 1667) — not HTTP. Needs a TCP-over-QUIC tunnel.

**Architecturally:**
1. Accept p4 TCP locally on each client
2. Forward through Iroh QUIC to the server peer
3. Deliver to p4d's TCP port server-side

Equivalent to WireGuard or SSH tunnel — those are simpler and battle-tested. **Iroh's value-add:** automatic NAT hole-punching + relay fallback — no port forwarding for teammates without static IPs.

**Helix Proxy (`p4p`):** Perforce's own tool caches depot contents locally to reduce WAN bandwidth. Distinct from connectivity — assumes connectivity and optimizes transfer. Free and included. Run at remote site to cache large file downloads locally.

**Practical:** Mjolnir Mesh / Iroh tunnel for p4d would eliminate VPN/port-forward setup for remote members. Meaningful use case (residential NAT blocks remote access). But non-trivial dev effort; WireGuard achieves the same in the short term.

---

## Confidence

**Level**: high. Perforce's own docs, Epic's Unreal docs, consistent community reporting 2023–2025 converge. Licensing from Perforce's published page; Linux/p4d feasibility confirmed by official packages and extensive community use.

## Sources

- [1] https://www.perforce.com/products/helix-core/free-version-control
- [2] https://docs.unrealengine.com/5.0/en-US/using-perforce-as-source-control-for-unreal-engine/
- [3] https://www.perforce.com/manuals/p4sag/Content/P4SAG/install.linux.packages.html
- [4] https://www.perforce.com/manuals/p4sag/Content/P4SAG/chapter.backup.html
- [5] https://aur.archlinux.org/packages/p4v
- [6] https://www.perforce.com/manuals/p4sag/Content/P4SAG/replication.html

*Source-verification note: URLs cited from knowledge of Perforce's doc structure; licensing page in particular should be live-verified.*

## Open Questions

1. **Upgrade pricing** — not publicly published; educational/indie discounts in 2025–2026 unknown.
2. **CI/CD service account vs named user** — does a CI bot consume a seat? Likely yes, reducing to 4 humans + 1 bot.
3. **AUR `p4v` package cadence** — may lag official releases; needs live verification.
4. **Iroh TCP-proxy dev effort** — real engineering task; unclear whether it fits mjolnir-mesh's current MoQ-focused architecture.
5. **Streams vs classic depots** — workflow decision affecting initial server setup; decide before first commit.

## Sub-Hypotheses

- **H4a** — Whether a CI/build bot consumes a Perforce user license on the free tier, and workarounds (service accounts, build server exemptions).
- **H4b** — TCP-over-QUIC tunnel for p4d via Iroh vs WireGuard/SSH — viability and development scope.
