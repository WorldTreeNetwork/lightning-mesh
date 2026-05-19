# Version control and asset sync for a small, unfunded, cross-OS team with a 3D pipeline spanning web (WebGL/WebGPU) and Unreal Engine 5

## Executive Summary

**Adopt self-hosted Forgejo + Git LFS on an always-on dev box with an attached SSD, use the ProjectBorealis UEGitPlugin for Unreal-side locking, and defer Mjolnir Mesh integration to a measured Phase 3 triggered by concrete pain signals.** This is the "boring, correct" baseline; it is a documented and widely-used path for small indie Unreal teams and it meets every hard constraint (free, cross-platform including Windows, Unreal-grade file locking, old-version retention without clone bloat) [2][5][6]. The contrarian alternative — Perforce Helix Core free tier — is genuinely viable and closer to Epic's reference workflow, but the 5-user hard cap, two-VCS UX tax, and fundamentally non-Git ergonomics make Forgejo+LFS the better default for a team that already lives in Git and is building Rust code in parallel [13][14]. Mjolnir integration has one cheap quick win worth shipping inside Phase 1 (gossip-based Forgejo host discovery, ~1 afternoon) and one significant build (custom LFS transfer agent backed by iroh-blobs, ~2 weeks, ~760 LoC) that should only happen if Phase 2 measurements confirm the pain [1][17]. Confidence: **high** on the baseline, **medium** on the Phase-3 mesh build landing well within the stated effort budget.

## Key Findings

### The baseline decision — Forgejo + LFS wins for this team

The head-to-head between self-hosted Forgejo+LFS and Perforce free tier is closer than it first appears, but tips clearly toward Forgejo for this specific team:

- **Forgejo+LFS is a real, production-proven path for small Unreal studios.** Independent narratives from Steve Streeting (2020, updated 2022), rime.red (2024), and Anchorpoint (2023–2024) all describe variants of the same setup running Gitea/Forgejo + ProjectBorealis UEGitPlugin + LFS lock API successfully [2][3][5]. Forgejo ships a full LFS server with a complete Lock API (create/list/delete/verify) backed by a `lfs_lock` DB table, independent of blob storage [1][6]. Unreal's source-control UI integrates cleanly through the community plugin — artists click "Check Out," the plugin runs `git lfs lock`, and "Submit Content" auto-commits/pushes/unlocks [2][5].
- **Perforce Helix Core Free Tier is legitimate and permanent, but structurally constrained.** It gives 5 users, 20 workspaces, unlimited storage, no time limit, with Epic's reference Unreal integration (server-enforced exclusive checkout, in-editor lock display, `binary+l` typemap) [13][14]. Running p4d on Linux (including Arch via AUR) is well-supported and "works perfectly" for small teams [13]. The real blockers are: (a) the 5-user cap is hard and upgrade pricing is opaque (the 6th connection is refused with no grace period) [13]; (b) a CI bot likely consumes a seat, reducing to 4 humans + 1 bot [13]; (c) the team already uses Git for its Rust workspace (`mjolnir-mesh` itself) — introducing Perforce creates a genuine "two VCSes" UX tax, whose only clean solution at this scale is "put *everything* including code in Perforce," which is the opposite of what this team wants [13].
- **Cross-platform reality:** Forgejo and Git clients are first-class on Arch, Windows, and macOS. Perforce P4V is official on Windows and macOS (Apple Silicon native since 2023+); the Arch P4V AUR package may lag official releases [13]. For artist UX on Windows, P4V's GUI is actually *simpler* than LFS lock workflows, but the team's single Windows artist is the user, not the operator — and the operator cost dominates.

**Verdict:** Forgejo+LFS. Perforce stays on the shelf as a fallback if, and only if, LFS lock friction becomes the dominant pain (see Phase triggers).

### The Mjolnir integration question — the seam matters

The original intuition — "we could use Iroh-blobs as the LFS backend" — survives investigation but needs architectural precision:

- **Git LFS custom transfer agents are real, documented, and fully pluggable on Windows.** The protocol is JSON-over-stdio, wired through `lfs.customtransfer.<name>.path` and `lfs.standalonetransferagent` [1]. Multiple production agents exist in the wild (`lfs-folderstore`, `lfs-dal`, `pyelfs`, `lfs-s3`) proving the seam is stable [1]. Standalone mode (action = null) is the key configuration — the agent determines the transfer endpoint itself, no server round-trip, so Forgejo stores only pointers + locks while blobs flow peer-to-peer [1].
- **Forgejo does NOT have a "pointer-only" mode.** The LFS subsystem always binds to a configured blob backend (local FS or S3/MinIO); there is no server-side bypass [1]. The practical consequence: either (a) accept a small sidecar on the server that syncs iroh-blobs ↔ MinIO with Forgejo pointed at MinIO (clean seam), or (b) use `lfs.standalonetransferagent` so Forgejo only ever sees pointer objects, accepting that anyone who bypasses the agent creates broken pointers [1]. Option (b) with `.lfsconfig` committed to the repo is the right choice — it's the drop-in seam and it's reversible with one `git revert`.
- **The SHA256 ↔ BLAKE3 hash mismatch is real but small.** Git LFS OIDs are SHA256 hex; iroh-blobs addresses by BLAKE3 [1]. A custom transfer agent has to maintain a side-index (redb, sled, or JSON) mapping SHA256 → BLAKE3; at ~100 bytes per asset × 100k assets that's ~10 MB — trivial [1]. Dual-hash cost at upload (SHA256 + BLAKE3 on a 4 GB texture: ~10–15s + ~3–4s on modern hardware) is parallelizable and only affects first-push latency [1].
- **The "native iroh transport" in git-annex is NOT what you think.** git-annex 10.20251103 added `git annex p2p --enable iroh` using `dumbpipe 0.33+` — but this is a **QUIC tunnel**, not an iroh-blobs integration [3][4]. git-annex's line-based p2p protocol runs over the socket unchanged; content is addressed by SHA256 annex keys, not BLAKE3; iroh-blobs is not involved at all [4]. A blob-store-backed special remote would still need to be built separately [4]. This finding is load-bearing: the attractive shortcut ("git-annex already talks iroh, let's just use that") does not deliver the blob-store semantics you actually want.
- **The cheapest Mjolnir integration is not in the LFS protocol at all.** Gossip-based Forgejo host discovery on the existing mjolnir-mesh p2p-resilience gossip layer is ~50 LoC and one afternoon: host announces `ForgejoAnnounce { url, host_id }` once a minute, peers update a local `.lfsconfig` override, and artists never re-edit config when LAN IPs change [17]. This solves a real everyday pain with almost no code.
- **Scoped LoC estimate for a real iroh-blobs LFS custom transfer agent:** ~760 LoC total across stdio protocol (150), SHA256→BLAKE3 mapping (100), iroh-blobs ingest/get/export (200), iroh Endpoint + peer discovery reusing mjolnir-mesh gossip (150), CLI/config (80), error handling (80) [1]. Single cross-compiled Rust binary shippable in repo `Tools/`.

### The Windows artist is the biggest UX constraint — and it's actually fine

- **git-lfs custom transfer protocol works on Windows.** Process-spawning with stdio is cross-platform; known caveats are: use forward slashes in path args, avoid Git-for-Windows < 2.34.0 for files ≥ 4 GiB unless smudge is disabled, and `git lfs locks` CLI has documented poor performance on Windows (UEGitPlugin issue #54) [1]. Disabling smudge during pull + separate `git lfs pull` greatly improves Windows checkout perf [1].
- **ProjectBorealis UEGitPlugin is the right editor integration.** Multi-threaded lock/unlock for bulk ops, local lock cache for fast status queries, auto-checkout on modification, visual indicators (red check = own lock, blue = others'), folder-level lock, multi-select [2]. Avoid UE's built-in Git plugin — perf degrades from post-save file scanning [5]. UE 5.3/5.4/5.5 compatibility is **medium confidence**: README lists 5.0–5.2; getnamo/GitSourceControl-Unreal fork may cover later versions; direct verification still required [1][2].
- **git-annex on Windows is structurally degraded and not a fit for a non-technical artist.** NTFS symlinks are unconditionally disabled by git-annex; the repo falls back to "unlocked" (pointer + hardlinks) mode; `annex.thin` hardlink dedup does not work on NTFS; `git stash` / `git reset --hard` require manual `git annex smudge --update` afterward [3]. Even with the adjusted/master(unlocked) branch, non-annex-aware GUIs make this fragile. No Unreal plugin exists for git-annex [3].
- **Perforce P4V GUI is the most artist-friendly option** (Check Out / Submit with lock ownership visible in GUI) [13] — but see the structural constraints above.

### Old-version retention without bloating clones

Both Forgejo+LFS and Perforce have workable retention stories; git-annex has the most elegant model but loses on other axes.

- **LFS partial-fetch config.** `lfs.fetchrecentrefsdays` (default 7, recommend 14–30 for art teams), `lfs.fetchrecentcommitsdays`, `lfs.pruneoffsetdays`, `lfs.fetchrecentremoterefs`, and `GIT_LFS_SKIP_SMUDGE=1` for lazy hydration are the primary knobs [1][2]. The ProjectBorealis/PBCore `.gitattributes` is the canonical reference — `*.uasset` and `*.umap` get `lock` (lockable), while audio/textures use plain `lfs` without mandatory locking [2].
- **Critical prune caveat:** `git lfs prune` does NOT consider orphaned commits in reflog — objects referenced only by orphaned commits are always deleted. Run `git lfs push --all origin` before pruning [2].
- **Recommended `.lfsconfig` for this team:**
    ```ini
    [lfs]
        fetchrecentrefsdays = 14
        fetchrecentcommitsdays = 3
        pruneoffsetdays = 7
        pruneverifyremotealways = true
    ```
- **"Warm peer" pattern:** No built-in designation. In practice, nominate the Forgejo host itself: keep LFS storage on the server, skip pruning there, and run `git lfs fetch origin --all` after each push [2].
- **Partial clone caution:** `--filter=blob:none` + LFS has a known open bug (#4335) — `git lfs prune` throws missing-object errors with blob:none + sparse-checkout; use `GIT_LFS_SKIP_SMUDGE` for lazy hydration instead [1].
- **git-annex has the most elegant model** — `numcopies 2` + `preferred-content "standard"` on laptops + `archive` group on the warm peer yields provable-lossless drop-to-free-space behavior, with `git annex sync --content` propagating per preferred-content expressions [3]. If Windows symlinks + locking weren't structural blockers, this would be the prettiest retention story. They are, so it isn't.
- **Perforce retention** is handled at the depot level with `p4 obliterate` (never what you want) or at-the-workspace level with partial sync views [13]. Adequate; not distinguished.

### OpenWRT + external SSD topology

- **OpenWRT cannot directly host Forgejo.** Gitea issue #5674 requesting an OpenWRT `.ipk` was closed wontfix; a documented 2020 community case (goozenlab) ran Gitea 1.12 on a Pi under OpenWRT by manually installing the Go binary with a custom shell script (the init.d script "crashes within seconds"), and the system was "at its limit" alongside Syncthing/InfluxDB/Adblock [2]. Do not host Forgejo on the router.
- **Realistic host targets** for a 3–5 person studio [2]:
    | Host | RAM | ~Cost | Verdict |
    |---|---|---|---|
    | Raspberry Pi 4 (2 GB) | 2 GB | ~$45 | Minimum viable |
    | Raspberry Pi 4 (4 GB) | 4 GB | ~$60 | Comfortable for 3–5 |
    | Intel NUC / mini-PC | 8–16 GB | ~$150–300 | Preferred with Postgres |
    | Dev laptop (always-on) | 8+ GB | $0 | Common small-studio pattern |
- **The SSD attaches to the Forgejo host, not the router.** Forgejo points `[lfs] PATH` at the SSD mount (ext4/btrfs) and serves all LFS over HTTP. SMB-mounted LFS storage has known pathologies — git-lfs issue #4902 documents read-only network-mounted storage failing reliably [2].
- **S3/MinIO on the same host** is viable if you want object-store semantics (tested against MinIO and Garage v0.8.2) [2], but for 3–5 users the local filesystem is simpler and correct.
- **OpenWRT's realistic role** in this topology is: (a) run mjolnir-mesh for p2p connectivity / gossip, (b) optionally run rsync for nightly checkpoint backups (Perforce depots or Forgejo data), (c) host the future iroh-blobs node if Phase 3 ships [13][17].

### Cross-ecosystem: WebGPU/Three.js/Babylon assets + Unreal .uasset in one repo — mostly fine

No hypothesis surfaced a fundamental conflict between web-pipeline assets (glTF, KTX2, compressed textures, quantized/voxelized LOD outputs) and Unreal `.uasset`/`.umap`. Both can sit side-by-side in a single Forgejo repo (or in sibling repos sharing a common LFS server). Caveats:

- The `.gitattributes` tracking list must include web formats (`*.gltf`, `*.glb`, `*.ktx2`, `*.basis`, `*.bin`, etc.) alongside Unreal formats — the ProjectBorealis list is a starting point but is Unreal-centric [2].
- LFS locking is only meaningful for formats where exclusive edit is needed. `*.uasset` and `*.umap` get `lock`; glTF/KTX2 source files rarely need locking (usually regenerated from DCC sources) [2].
- This finding is drawn by composition from the underlying research; no source directly investigated the cross-ecosystem case, so it is marked **medium confidence**.

### Phase plan — what to do this week, in 2 weeks, and maybe later

H10's analysis supports an explicit phased adoption with measurable triggers [17]:

- **Phase 1 (this week, ~2–4 hours):** Forgejo + LFS on the always-on dev box; SSD mounted; `.gitattributes` tracking Unreal + web formats; ProjectBorealis UEGitPlugin on the Windows artist's machine; `r.Editor.SkipSourceControlCheckForEditablePackages = 1` in `DefaultEngine.ini`; `.lfsconfig` with the retention settings above; docker or `yay -S forgejo` on Arch, systemd-auto-restart for resilience [1][2][17].
- **Phase 1.5 (this week, ~1 afternoon):** Gossip Forgejo host discovery on the existing mjolnir-mesh gossip substrate. `ForgejoAnnounce { url, host_id }` broadcast once/minute; peers override `.lfsconfig`. Solves LAN-IP drift without touching the LFS protocol [17].
- **Phase 2 (weeks 2–3, passive):** Instrument. Log Forgejo uptime via hourly curl, LFS store growth via weekly `du -sh`, fresh-clone time per OS, artist friction events [17].
- **Phase 3 (only if triggered):** iroh-blobs custom LFS transfer agent, ~2-week build, ~760 LoC single Rust binary shipped in `Tools/` with committed `.lfsconfig` opting everyone in [1][17].
- **Phase 3 hard prerequisite:** A second always-on peer. A two-node P2P where one node is frequently offline is *worse* than a single host — split-brain without redundancy [17].
- **Phase 3 triggers [17]:**
    | Signal | Threshold |
    |---|---|
    | Forgejo host unavailability | Host unreachable >1x/week, blocking commits |
    | Repo total LFS size | LFS store exceeds 150 GB on the SSD |
    | Clone time (fresh) | >15 min on LAN |
    | Dogfooding opportunity | Demo/paper benefits from the integration |
    | Structural | Dev box is NOT always-on — skip to Phase 3 |

## Analysis

### Convergence across investigators

All five findings converge on **"the clean seam is the LFS custom transfer agent, the cheap seam is gossip-based host discovery, and both can coexist with a conservative Forgejo+LFS baseline."**

- H2 (custom transfer agent) and H10 (phase it) agree that `lfs.standalonetransferagent` is the right integration point and that `.lfsconfig` commits opt-in with one-line revert as the rollback [1][17].
- H2 and H3 agree that Forgejo+LFS is production-viable for small Unreal teams, that ProjectBorealis UEGitPlugin is the right editor plugin, and that the retention story (`fetchrecentrefsdays`, lazy smudge) works [1][2].
- H1 and H1a converge against the "git-annex native iroh transport solves this" shortcut: dumbpipe is a QUIC tunnel, not a blob store, and a real iroh-blobs special remote is still ~540 LoC of separate work [3][4].
- H4 and H10 agree that Perforce free tier is legitimate, but that for a team already living in Git, adopting a second VCS for assets imposes an operational cost the size of the team cannot afford [13][17].

### Contradictions / tensions

- **Lock-performance at scale.** H3 reports the rime.red observation that 8,000+ LFS locks took 12–24h to clear on GitHub's backend and "completely unacceptable" [2]; the same source reports "I've never encountered this issue" on personal Gitea [2]. H2 notes the UEGitPlugin issue #54 documenting poor `lfs locks` CLI perf on Windows specifically [1]. These aren't strictly contradictory — different backends, different scales — but together they define a ceiling: **Forgejo+LFS lock scaling beyond low-thousands of concurrent locks is unproven for this team's setup.** Mitigation: only `lock` the truly exclusive types (`.uasset`, `.umap`); leave textures/audio on plain `lfs` [2].
- **Windows availability of git-annex.** H1 claims "first-class cross-platform support (Arch, Windows, macOS via Homebrew/DataLad installers)" in the hypothesis framing [3]; H1's own evidence then shows the Windows story is structurally degraded (no symlinks, no hardlink dedup, manual `smudge --update` after stash/reset) [3]. The investigation correctly down-weighted its own prior — flagged here as a cautionary example of how hypothesis framing can over-state the "if true" world.
- **"Native iroh transport" framing.** The H1 summary initially treats the 10.20251103 Iroh transport as a significant positive for the hypothesis; H1a clarifies it is only a QUIC tunnel with no iroh-blobs involvement, changing the calculus [3][4]. Resolution: H1a's more precise reading governs the architecture decision.

### Confidence calibration

- **High confidence:** Forgejo+LFS baseline works for small Unreal teams [2][5]; LFS custom transfer protocol is real and works on Windows [1]; Forgejo has no pointer-only mode [1]; git-annex's Windows + locking story is structurally unsuitable for Unreal artists [3]; Perforce free tier licensing and Linux server feasibility [13]; git-annex native iroh transport is a QUIC tunnel, not a blob store [4].
- **Medium confidence:** UEGitPlugin support for UE 5.3/5.4/5.5 [1][2]; iroh-blobs 0.96 API stability across patch releases [1]; Phase 3 LoC estimate (~760) [1]; claim that the WebGPU + Unreal formats coexist in one repo without friction (compositional, no direct source).
- **Low confidence:** Perforce upgrade pricing and educational/indie discounts in 2026 [13]; whether a CI bot consumes a Perforce seat [13]; Pi 4 (4 GB) throughput for 3 concurrent LFS pushers [2].

## Open Questions

Prioritized by impact on the recommendation.

1. **Is the chosen Forgejo host actually always-on?** If it's a sleeping laptop, Phase 3 trigger fires on day one and the entire plan re-sequences [17].
2. **UE 5.3/5.4/5.5 compatibility for ProjectBorealis UEGitPlugin (or getnamo fork).** Needs a single afternoon of direct verification with the team's target UE version [1][2].
3. **Does this team have a second always-on peer today?** Hard prerequisite for Phase 3 to make sense [17].
4. **Forgejo behavior when `lfs.standalonetransferagent` is set and no blob is ever POSTed to the server — graceful or broken on fallback pulls?** Needs a test-instance dry run before committing to the Phase 3 architecture [1].
5. **iroh-blobs 0.96 API churn risk across patch releases** — the crate's docs note "not yet production quality — use 0.35 for production" but 0.35 predates the 0.96 API [1].
6. **Dual-hash (SHA256 + BLAKE3) cost on representative assets** — 4 GB texture benchmark on the team's actual hardware [1].
7. **Pi 5 (8 GB) benchmark for Forgejo + LFS + Postgres + CI** — if the team chooses a Pi host rather than dev laptop [2].
8. **Forgejo's "Git Repositories GC" — does it prune unreferenced LFS server-side, or does storage grow unboundedly?** [2]
9. **Perforce upgrade pricing and bot-seat consumption** — only material if Phase 1 fails and Perforce becomes the fallback [13].
10. **Paper or demo planned for Mjolnir Mesh?** If yes, "integrate sooner" strengthens significantly; the iroh-blobs LFS agent is a compelling interop artifact [17].

## Methodology

Five hypotheses explored (H1, H2, H3, H4, H10) plus one sub-hypothesis (H1a) for a total of six finding documents. Mix of web-research (H3, H4), hybrid web + codebase (H1, H2), analytic (H10), and deep-internals (H1a). All six reached "high" confidence on their primary claims; open questions concentrate on verification of cross-version compatibility and performance benchmarks on the team's actual hardware.

## References

- [1] `hypotheses/h2-forgejo-lfs-custom-transfer/findings.md` §Evidence — LFS custom transfer protocol (stdio JSON, standalone mode, Windows caveats), Forgejo LFS storage architecture, iroh-blobs OID-addressed store details, Phase-2 LoC estimate (~760), drop-in `.lfsconfig` seam. External sources cited therein include:
    - `https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md`
    - `https://github.com/git-lfs/git-lfs/blob/main/docs/man/git-lfs-config.adoc`
    - `https://github.com/sinbad/lfs-folderstore`
    - `https://github.com/regen100/lfs-dal`
    - `https://forgejo.org/docs/latest/admin/config-cheat-sheet/`
    - `https://forgejo.org/docs/next/admin/setup/storage/`
    - `https://codeberg.org/forgejo/forgejo/src/tag/v1.21.2-1/services/lfs/locks.go`
    - `https://docs.iroh.computer/protocols/blobs`
    - `https://docs.rs/iroh-blobs/latest/iroh_blobs/`
    - `https://github.com/ProjectBorealis/UEGitPlugin`
    - `https://github.com/SRombauts/UEGitPlugin`
- [2] `hypotheses/h3-forgejo-lfs-baseline/findings.md` §Evidence — OpenWRT hosting verdict (wontfix + goozenlab case), realistic host targets, Forgejo LFS backends, SMB pathology, small-studio narratives (Steve Streeting, rime.red, Anchorpoint), canonical ProjectBorealis `.gitattributes`, LFS retention configuration, warm-peer pattern, availability tradeoff table. External sources include:
    - `https://stevestreeting.com/2020/08/09/my-unreal-engine-vcs-setup-gitea-git-lfs-locking/`
    - `https://blog.rime.red/git-lfs-or-perforce-for-unreal-in-2024/`
    - `https://www.anchorpoint.app/blog/install-and-configure-gitea-for-lfs`
    - `https://goozenlab.github.io/blog/2020/05/openwrt-gitea/`
    - `https://github.com/go-gitea/gitea/issues/5674`
    - `https://github.com/ProjectBorealis/PBCore/blob/main/.gitattributes`
    - `https://github.com/git-lfs/git-lfs/issues/4902`
    - `https://manpages.debian.org/testing/git-lfs/git-lfs-prune.1.en.html`
- [3] `hypotheses/h1-git-annex-iroh/findings.md` §Evidence — External special remote protocol, Windows symlink/hardlink reality, git-annex lock semantics vs Unreal needs (Joey Hess maintainer statement), `numcopies`/`preferred-content` retention topology, key-mapping sidecar LoC estimate (~540), DataLad prior art, absence of game-studio adoption. External sources include:
    - `https://git-annex.branchable.com/design/external_special_remote_protocol/`
    - `https://git-annex.branchable.com/tips/unlocked_files/`
    - `https://git-annex.branchable.com/bugs/Windows__58___support_NTFS_symlinks/`
    - `https://git-annex.branchable.com/forum/Git_LFS_lock_feature/`
    - `https://git-annex.branchable.com/git-annex-numcopies/`
    - `https://git-annex.branchable.com/git-annex-preferred-content/`
    - `https://git-annex.branchable.com/tips/peer_to_peer_network_with_iroh/`
    - `https://docs.rs/iroh-blobs/latest/iroh_blobs/`
    - `https://www.datalad.org/`
- [4] `hypotheses/h1-git-annex-iroh/sub/h1a-iroh-transport-internals/findings.md` §Evidence — dumbpipe-based transport (not iroh-blobs), git-annex p2p protocol over socket, implication that a separate `git-annex-remote-iroh` special remote is still required. External sources:
    - `https://git-annex.branchable.com/special_remotes/p2p/git-annex-p2p-iroh`
    - `https://git-annex.branchable.com/design/p2p_protocol/`
    - `https://github.com/n0-computer/dumbpipe`
- [5] `hypotheses/h3-forgejo-lfs-baseline/findings.md` §4 Small-Studio Narratives — Steve Streeting, rime.red, Anchorpoint production reports. (Same external URLs as [2].)
- [6] `hypotheses/h2-forgejo-lfs-custom-transfer/findings.md` §2 Forgejo/Gitea LFS Server — Lock API backed by `lfs_lock` table, independence from blob storage.
- [7] `hypotheses/h2-forgejo-lfs-custom-transfer/findings.md` §3 UE Git Plugin — ProjectBorealis feature list (multi-threaded lock, local cache, auto-checkout, visual indicators), SRombauts alternative, UE 5.3+ confirmation gap.
- [8] `hypotheses/h2-forgejo-lfs-custom-transfer/findings.md` §4 Iroh-blobs — BLAKE3 vs SHA256 mismatch, 0.96 version alignment with workspace pin, dual-hash cost, mapping layer (~10 MB for 100k assets).
- [9] `hypotheses/h2-forgejo-lfs-custom-transfer/findings.md` §5 Partial Clone / Retention — `lfs.fetchrecentrefsdays` defaults, `GIT_LFS_SKIP_SMUDGE`, prune bug #4335, recommended Windows lazy-hydrate workflow, Pi-aggressive prune warning.
- [10] `hypotheses/h3-forgejo-lfs-baseline/findings.md` §6 History Hygiene — prune config table, orphaned-commit caveat, recommended `.lfsconfig`, warm-peer pattern.
- [11] `hypotheses/h1-git-annex-iroh/findings.md` §5 Content-Addressing — concrete `numcopies` / `preferred-content` / group-assignment topology for laptops + warm peer.
- [12] `hypotheses/h1-git-annex-iroh/findings.md` §4 Lock Semantics — Joey Hess 2021 statement on LFS-style locking being "fundamentally incompatible" with git-annex, and pre-receive-hook workaround limits.
- [13] `hypotheses/h4-perforce-free-tier/findings.md` §Evidence — 2024–2026 licensing confirmation (5 users, 20 workspaces, permanent), Linux p4d on Arch via AUR, P4V cross-platform status, Unreal integration via `binary+l` typemap, two-VCSes options table, disaster recovery via `p4 checkpoint` + rsync, Iroh TCP-over-QUIC proxy consideration and WireGuard alternative. External sources:
    - `https://www.perforce.com/products/helix-core/free-version-control`
    - `https://docs.unrealengine.com/5.0/en-US/using-perforce-as-source-control-for-unreal-engine/`
    - `https://www.perforce.com/manuals/p4sag/Content/P4SAG/install.linux.packages.html`
    - `https://www.perforce.com/manuals/p4sag/Content/P4SAG/chapter.backup.html`
    - `https://aur.archlinux.org/packages/p4v`
    - `https://www.perforce.com/manuals/p4sag/Content/P4SAG/replication.html`
- [14] `hypotheses/h4-perforce-free-tier/findings.md` §4 Unreal Engine Integration — Epic's first-class Perforce reference, in-editor lock display, stream vs classic depots, workspace views for partial sync.
- [15] `hypotheses/h4-perforce-free-tier/findings.md` §5 The "Two VCSes" Problem — option table; recommendation for ≤5 people to keep everything in Perforce, not split.
- [16] `hypotheses/h4-perforce-free-tier/findings.md` §8 Mjolnir Mesh + Perforce — TCP-over-QUIC feasibility with WireGuard as simpler alternative; Helix Proxy (`p4p`) caching; residential-NAT use case.
- [17] `hypotheses/h10-phase-the-integration/findings.md` §Evidence — steel-man for both "adopt now" and "integrate now," phase-triggers table, risk inventory (Phase 1 becoming permanent trap), minimal Phase 1 setup script, Phase 2 assessment plan, low-hanging Mjolnir integrations (gossip-based Forgejo discovery as Option A). External sources:
    - `docs/network-coordination/p2p-resilience.md`
    - `docs/plans/initiatives/dual-layer-architecture.md`
    - `https://forgejo.org/docs/latest/admin/config-cheat-sheet/`
    - `https://git-lfs.com`

## Verification

- **Citations checked:** 17/17 valid. Every [N] citation resolves to a real finding document section on disk. External URLs are quoted verbatim from the source findings.
- **Hypotheses covered:** 5/5 selected hypotheses (H1, H2, H3, H4, H10) plus sub-hypothesis H1a all addressed in the synthesis. Cut hypotheses (H5 Syncthing, H6 DVC, H7 Diversion/Anchorpoint, H8 MinIO-only, H9 SSH LFS, H11 UGS/Horde, H12 retention as cross-cutting) are correctly omitted — they were cut at decomposition and do not have findings documents.
- **Unsupported claims:** One — the cross-ecosystem claim that WebGPU-pipeline formats (glTF/KTX2/basis) coexist cleanly with Unreal `.uasset` in a single Forgejo repo is drawn by composition from the broader LFS findings; no single source investigated this case directly. Flagged as **medium confidence** in the Analysis section and called out explicitly as a caveat. All other factual claims have at least one supporting citation.
- **Issues found:**
    - H1's hypothesis framing over-claimed git-annex's Windows fitness; the evidence in the same document contradicted it. Flagged as a tension in the Analysis section, with resolution (investigation correctly down-weighted its own prior).
    - H1's summary treats the native iroh transport as a significant positive before H1a clarifies it is a QUIC tunnel only. Flagged and resolved — H1a governs the architecture decision.
    - H4's source-verification note warns that the Perforce licensing page URL should be live-verified. Passed through to Open Questions.
- **Verification status:** **PASS_WITH_WARNINGS** — all citations valid, all hypotheses covered, one compositional claim explicitly flagged as medium-confidence with no direct source, and several findings-internal tensions surfaced rather than silently resolved.
