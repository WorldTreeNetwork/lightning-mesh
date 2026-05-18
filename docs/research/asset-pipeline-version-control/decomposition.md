# Decomposition: Version control and asset sync for a small, unfunded, cross-OS team with a 3D pipeline spanning web (WebGL/WebGPU) and Unreal Engine 5

## Understanding

The team needs a version-control-plus-asset-sync solution that behaves like Perforce for Unreal-style workflows (file locking, large binary versioning, tight code-asset coupling) while being free, peer-to-peer friendly, cross-platform (Arch/Windows/macOS), and ideally layered on or alongside their own Mjolnir Mesh (Iroh 0.96 QUIC + MoQ) substrate. A good answer identifies one concrete primary stack plus a fallback, explains how binaries are stored vs. how code is committed, how old versions are pruned from local clones but kept recoverable, how locking is enforced, and a migration path (start-simple → grow-into-mesh). It must honestly assess the build-our-own vs. adopt-existing tradeoff.

## Sub-Questions

1. **Which existing VCS / asset-manager stack best fits a zero-budget, cross-OS, Unreal-plus-web team?** (axis a) — What do comparable small studios actually use when they can't afford Perforce Helix Core proper, Plastic/Unity VCS seats, or LFS bandwidth?
2. **What is the right storage substrate for the large binaries themselves on a LAN with an OpenWRT router + external SSD and no always-on NAS?** (axes b, d) — Content-addressed blob store, S3-on-LAN, SMB/NFS share, or a sync-based fabric like Syncthing/Resilio/Iroh-blobs?
3. **How is the binary store glued to Git commits so code + assets move as one atomic version, with locking for .uasset/.umap and shallow local history?** (axis c) — Git-LFS custom transfer agent, git-annex, DVC, Xet, or an Unreal-plugin-level integration.
4. **Is riding on Mjolnir Mesh (Iroh-blobs + MoQ) for asset distribution a credible near-term engineering path, or a distraction from shipping?** (axis e) — Specifically as a `git-lfs` custom transfer agent or git-annex special remote.
5. **What does old-version retention look like without bloating every clone?** (axes b, c) — Partial clone / shallow LFS / git-annex `get`/`drop` / content-addressed GC policy on a "warm" peer.

## All Candidate Hypotheses

### H1: git-annex with an Iroh-blobs special remote is the best long-term fit

- **Plausibility**: medium | **Info Value**: high | **Type**: hybrid
- **Rationale**: git-annex already supports arbitrary "special remotes" via a well-documented external protocol (stdin/stdout line protocol); its content-addressed model (symlinks to SHA256-keyed files under `.git/annex/objects`) maps almost 1:1 onto iroh-blobs' BLAKE3-addressed store. git-annex natively supports "get"/"drop"/"copy --to" commands so old versions are preserved on warm peers but not every clone, and it has first-class cross-platform support (Arch, Windows, macOS via Homebrew/DataLad installers). It also has a `lock`/`unlock` concept, though not Unreal-grade exclusive locking.
- **If true**: The team gets free, p2p-native, de-duplicated asset sync that reuses Mjolnir's substrate, with a ~few-hundred-line Rust external-special-remote as the only new code. Fallback is trivial: point the same annex at a SMB directory or MinIO bucket.
- **If false**: Either git-annex's Windows story is too rough for artists, or the special-remote protocol can't express the streaming/range semantics we need for large .uasset pulls, pushing us to LFS-based approaches.
- **Effort**: medium

### H2: Self-hosted Forgejo/Gitea + git-lfs with a custom LFS transfer agent backed by Iroh-blobs

- **Plausibility**: high | **Info Value**: high | **Type**: hybrid
- **Rationale**: Git LFS has a documented "custom transfer" protocol (`lfs.customtransfer.<name>.path`) that lets a client-side binary handle upload/download of OIDs. Unreal Engine has first-class git-lfs support via the official Git source-control plugin (including `git lfs lock`). Forgejo/Gitea self-hosted removes GitHub's bandwidth ceiling. A transfer agent that fetches/pushes to iroh-blobs would let LFS pointers resolve over the mesh while Forgejo only stores the tiny pointer files and lock metadata. This is probably the shortest path to "it works in Unreal today."
- **If true**: We keep Unreal's native Git+LFS workflow (artists click "Check out" in the editor), get locking for free via Forgejo's LFS lock API, and can swap the transfer backend between iroh-blobs, SMB, and MinIO without changing the Git side.
- **If false**: If Forgejo's LFS server requires blobs on its own disk (not just pointers), or if the custom transfer agent can't be installed easily on Windows artist machines, we fall back to a vanilla Forgejo + self-hosted LFS on the OpenWRT SSD.
- **Effort**: medium

### H3: Boring-but-correct stack — Forgejo + self-hosted git-lfs on the OpenWRT SSD (SMB-mounted or direct), with LFS file locks

- **Plausibility**: high | **Info Value**: medium | **Type**: web
- **Rationale**: This is what most small Unreal teams who left GitHub LFS actually do. Forgejo/Gitea ship a built-in LFS server with lock API; it can be hosted on any Linux box (including the router if it has enough RAM, or more realistically on a dev laptop with the SSD attached). Cost is zero, bandwidth is LAN-bound, Unreal's Git plugin already supports it, and partial-clone + `lfs.fetchrecentrefsdays` prune old versions from local clones. This is the baseline to beat.
- **If true**: Confirms there's a known-good zero-budget path; the Mjolnir-native approach only has to beat this on availability (no single host) or bandwidth, not on correctness.
- **If false**: Would mean even the conservative path has a blocker (e.g., OpenWRT can't run Forgejo comfortably; Windows SMB + LFS has pathological behavior), which reshapes the whole question.
- **Effort**: light

### H4: Perforce Helix Core free tier (5 users / 20 workspaces) is viable and the team should just use it

- **Plausibility**: medium | **Info Value**: high | **Type**: web
- **Rationale**: Helix Core is free for up to 5 users and 20 workspaces; it's what Epic itself recommends for Unreal; file locking, stream depots, and partial sync are native; Unreal's editor integration is the reference implementation. Cross-platform clients exist (P4V on Arch via AUR, Windows, macOS). The "zero budget" constraint is satisfied at current team size. The contrarian angle: maybe the right answer is "don't build anything, don't use Git for assets, use the tool the industry uses."
- **If true**: Dramatically simplifies the problem — code in Git, assets in Perforce, with well-documented bridges. The mesh question becomes "can Mjolnir serve as a proxy/replica for a P4 depot?" which is a smaller, separable problem.
- **If false**: The 5-user ceiling, Linux-server setup friction, or the fundamental "two VCSes" UX cost rules it out for this team.
- **Effort**: light

### H5: Syncthing (or Resilio) for the raw asset tree + Git for code + an out-of-band lock service

- **Plausibility**: medium | **Info Value**: medium | **Type**: hybrid
- **Rationale**: Syncthing is free, cross-platform, p2p, and "just works" for artists. Many indie teams use it as a shared drive. The hard parts are: (1) no atomic coupling between code commits and asset versions, (2) no locking, (3) no history beyond Syncthing's shallow versioning. It's the "disable the problem" answer — worth surfacing because its simplicity is seductive and failures are instructive.

### H6: DVC (Data Version Control) for assets, Git for code — cut, ML-oriented, no Unreal integration, no locking
### H7: Diversion or Anchorpoint as a hosted free tier — cut, third-party cloud, eventually squeeze
### H8: MinIO / Garage / SeaweedFS S3-on-LAN — folded into H1/H2/H3 as a backend choice
### H9: git-lfs-transfer SSH protocol — folded into H2/H3 as an implementation detail
### H10: Build-your-own is a distraction — phase Mjolnir integration after adopting an off-the-shelf baseline

- **Plausibility**: high | **Info Value**: high | **Type**: analysis
- Included explicitly as a discipline hypothesis so the synthesis addresses sequencing, not just choice.

### H11: UGS/Horde — cut, Epic's enterprise tooling assumes Perforce and is overkill at this scale
### H12: Partial clone + LFS prune + warm-peer retention — treated as a cross-cutting finding each investigator must address

## Selected Hypotheses (top 5)

1. **H2: Forgejo + git-lfs + custom LFS transfer agent (optionally Iroh-blobs-backed)** → hybrid
2. **H3: Baseline Forgejo + self-hosted LFS on OpenWRT SSD** → web
3. **H1: git-annex with Iroh-blobs special remote** → hybrid
4. **H4: Perforce Helix Core free tier** → web
5. **H10: Build-your-own is premature; phase it** → analysis

## Cuts

H5, H6, H7, H9, H11 cut — each either a known-weak fit or a sub-technique that naturally surfaces inside H2/H3 investigation. H8 and H12 absorbed into whichever primary hypothesis wins.
