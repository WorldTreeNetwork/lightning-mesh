# Hypothesis: H2 — Self-hosted Forgejo/Gitea + git-lfs with a custom LFS transfer agent backed by Iroh-blobs

## Summary

This hypothesis is **largely confirmed as technically feasible but with important caveats**. The git-lfs custom transfer protocol is real, well-documented, and fully pluggable on Windows. Forgejo self-hosted can serve as the lock metadata and batch API server while a standalone client-side transfer agent routes blob bytes elsewhere — but Forgejo always stores blobs on its own configured backend; there is no "pointer-only" server mode. The iroh-blobs crate is suitable as a content-addressed blob store but uses BLAKE3 while LFS uses SHA256-OIDs, requiring a mapping layer. The Unreal Engine git plugin (ProjectBorealis fork) works with self-hosted Gitea/Forgejo and provides usable artist UX, but has documented Windows performance issues with locking. The full system is buildable in two phases with a well-defined seam.

---

## Evidence

### 1. LFS Custom Transfer Protocol — Reality and Windows Pluggability

The protocol is documented in full at `git-lfs/git-lfs/blob/main/docs/custom-transfers.md` [1]. It is a JSON-over-stdio protocol where git-lfs spawns a subprocess and exchanges newline-delimited JSON messages:

**Init (git-lfs → agent):**
```json
{ "event": "init", "operation": "download", "remote": "origin",
  "concurrent": true, "concurrenttransfers": 3 }
```
Agent responds `{}` on success or `{"error": {"code": N, "message": "..."}}`.

**Download event (git-lfs → agent):**
```json
{ "event": "download", "oid": "<sha256-hex>", "size": 21245,
  "action": { "href": "nfs://server/path", "header": {} } }
```

**Upload event (git-lfs → agent):**
```json
{ "event": "upload", "oid": "<sha256-hex>", "size": 346232,
  "path": "/path/to/file.png",
  "action": { "href": "nfs://server/path", "header": {} } }
```

**Standalone mode** (`lfs.standalonetransferagent`): the agent receives `null` for the `action` field and determines the transfer endpoint itself — no server round-trip needed. This is the critical configuration for an Iroh-backed agent [1][4].

**Git config wiring:**
```
lfs.customtransfer.lfs-iroh.path = /usr/local/bin/lfs-iroh-agent
lfs.standalonetransferagent = lfs-iroh
```

**Windows compatibility:** The protocol is process-spawning with stdio, which works on Windows. Known Windows caveats:
- Use forward slashes in path arguments [6]
- Files ≥ 4 GiB corrupt on Git for Windows < 2.34.0 unless smudge is disabled [4]
- `lfs locks` CLI has documented poor performance on Windows (issue #54 in UEGitPlugin) [7]
- Disabling smudge during pull + separate `git lfs pull` greatly improves Windows checkout perf [4]

**Production deployments of custom transfer agents confirmed:**
- `lfs-folderstore` (Go, shared NAS folder) [6]
- `lfs-dal` (Rust, OpenDAL-backed: S3, Azure, GCP, WebDAV) [9]
- `pyelfs` (Python, no HTTP server) [2]
- `lfs-s3` (Go) [3]

### 2. Forgejo/Gitea LFS Server — Storage Architecture

**Forgejo does not have a pointer-only mode.** The LFS subsystem separates concerns into a `lfs_meta_object` DB table and a configured blob backend (local filesystem or S3/MinIO) [8][10][11]:

```ini
[lfs]
STORAGE_TYPE = local
PATH = /var/lib/forgejo/lfs
; or MinIO/S3 credentials
```

There is no bypass to redirect blob transfers to a third party at the server level. Two viable architectures:

- **Architecture A (clean seam):** Forgejo with S3/MinIO backend. The iroh transfer agent on the client reads/writes blobs to the iroh mesh; a small sidecar on the server side syncs iroh-blobs ↔ MinIO. Forgejo never knows about iroh.
- **Architecture B (full bypass):** Use `lfs.standalonetransferagent` so the client agent routes blobs peer-to-peer via iroh, and Forgejo only stores lock metadata + pointer data. Requires Forgejo to not reject missing-blob state — which it does not enforce at lock time, only at download time.

**Lock API:** Forgejo implements full LFS Lock API (create, list, delete, verify) backed by a `lfs_lock` table with path, owner ID, repo ID, creation time [12]. This is independent of blob storage — the seam is clean.

### 3. Unreal Engine Git Plugin for LFS File Locking

**Two active plugins:**

**SRombauts/UEGitPlugin** — UE 4.27, 5.0, 5.1, 5.2. Wraps `git lfs lock`. Self-hosted Gitea documented as tested target [7].

**ProjectBorealis/UEGitPlugin** — more actively maintained fork with:
- Multi-threaded lock/unlock for bulk operations
- Local lock cache for fast status queries
- Auto-checkout on modification
- Visual indicators: red checkmark = own lock, blue = others'
- Folder-level lock, multi-select [17]

Both wrap `git lfs lock` / `git lfs unlock` CLI — works with any LFS server implementing the lock spec, including Forgejo.

**Artist UX from a production Gitea+UE setup [13]:**
- Save → editor prompts "check out" (lock)
- "Submit To Source Control" auto-commits, pushes, unlocks
- Lockable types set via `.gitattributes`: `*.uasset lockable`
- `r.Editor.SkipSourceControlCheckForEditablePackages = 1` in `DefaultEngine.ini` required

**UE 5.3/5.4/5.5 status:** ProjectBorealis README lists 5.0–5.2. The getnamo/GitSourceControl-Unreal fork may cover later versions. Both are community-maintained, not Epic-official. *Needs direct confirmation.*

### 4. Iroh-blobs as OID-Addressed Blob Store

**Hash function mismatch — the central technical challenge:**
- Git LFS OIDs are SHA256 hex (64 chars)
- iroh-blobs addresses by BLAKE3

**iroh-blobs API [15][16]:**
```rust
store.add_path(path).await?;         // returns Tag with Hash (BLAKE3)
store.has(hash).await?;
store.get(hash).await?;              // streaming
store.export(hash, path, ExportMode::Copy).await?;
```

BLAKE3 verified streaming authenticates each chunk against the root; interrupted/resumed transfers and range requests are first-class.

**Versioning:** `iroh-blobs 0.96.0` (released Oct 2024) matches the `iroh = "0.96"` pin in this workspace. The crate's docs note "not yet production quality — use 0.35 for production" but 0.35 predates the 0.96 API. API stability across 0.96 patch releases is the real risk.

**Required mapping layer:** A custom transfer agent must maintain a SHA256→BLAKE3 side-index.
- Upload: compute SHA256 (LFS OID) + ingest into iroh-blobs (BLAKE3) → save mapping
- Download: look up BLAKE3 by SHA256-OID → fetch from iroh mesh

At ~100 bytes per asset, 100k assets = ~10 MB index — trivial. Storage: redb, sled, or JSON file.

### 5. Partial Clone / History Retention / Lazy Hydrate

| Config | Default | Effect |
|--------|---------|--------|
| `lfs.fetchrecentrefsdays` | 7 | Fetch LFS objects for refs within N days; also prune threshold |
| `lfs.fetchrecentcommitsdays` | 0 | Also fetch LFS for N days beyond HEAD |
| `GIT_LFS_SKIP_SMUDGE=1` | off | Do not auto-download LFS blobs on checkout |
| `git lfs fetch --recent` | — | Pull blobs for recent refs per above config |
| `git lfs prune` | — | Delete local LFS blobs not in recent refs |

**`--filter=blob:none` + LFS open bug (#4335):** `git lfs prune` throws missing-object errors when cloned with `blob:none` + sparse-checkout. Workaround: use `GIT_LFS_SKIP_SMUDGE` for lazy hydration instead.

**Recommended Windows lazy-hydrate workflow:**
```bash
GIT_LFS_SKIP_SMUDGE=1 git clone <repo>
git lfs pull --include="Content/Characters/**"
# or bulk:
git lfs fetch --recent
git lfs checkout
```

**Prune safety:** Default `lfs.fetchrecentrefsdays = 7` is aggressive for art teams — recommend 30+ days or disable prune on workstations.

### 6. End-to-End Narrative

**Day 1 — Vanilla self-hosted LFS (no iroh):**

```bash
docker run -d \
  -e FORGEJO__lfs__START_SERVER=true \
  -e FORGEJO__lfs__CONTENT_PATH=/data/lfs \
  -v forgejo-data:/data \
  -p 3000:3000 codeberg.org/forgejo/forgejo:latest

git lfs install
git lfs track "*.uasset" --lockable
git lfs track "*.umap" --lockable
git lfs track "*.png" "*.jpg" "*.wav" "*.mp3"
```

Install ProjectBorealis/UEGitPlugin to `Plugins/UEGitPlugin`. Add `r.Editor.SkipSourceControlCheckForEditablePackages = 1` to `DefaultEngine.ini`. Configure source control provider "Git LFS 2" with the Forgejo URL.

**Day-1 cost:** 2–4 hours for a developer familiar with Docker + git.

**Phase 2 — Iroh-blobs custom transfer agent (2-week build):**

The seam: `lfs.standalonetransferagent` on the client. Forgejo keeps Lock API + git object store; blobs travel peer-to-peer via iroh.

| Component | LoC | Notes |
|-----------|-----|-------|
| Stdio JSON protocol handler (serde_json + tokio) | ~150 | |
| SHA256 → BLAKE3 mapping (redb or JSON) | ~100 | |
| iroh-blobs ingest (add_path, get, export) | ~200 | async + progress |
| iroh Endpoint + peer discovery via gossip | ~150 | reuse mjolnir-mesh |
| CLI parsing + config (clap) | ~80 | |
| Error handling + logging | ~80 | anyhow + tracing |
| **Total** | **~760** | Single Rust binary, cross-compiled |

**Risks:**
1. **Dual-hash cost:** SHA256 + BLAKE3 on a 4 GB texture: ~10–15s + ~3–4s on modern HW. Parallelizable but adds first-push latency.
2. **Forgejo blob orphans (Arch B):** If anyone bypasses the agent, Forgejo records broken pointers. Mitigate by committing `.lfsconfig` so the agent is mandatory.
3. **LAN peer discovery:** Handled by mjolnir-mesh gossip; WAN needs relays.
4. **Windows binary distribution:** Ship `lfs-iroh-agent.exe` in repo `Tools/` with relative path in `.lfsconfig`.

**Drop-in seam:**
```
# .lfsconfig (committed)
[lfs "https://forgejo.example.com/org/repo.git"]
  standalonetransferagent = lfs-iroh

[lfs "customtransfer.lfs-iroh"]
  path = Tools/lfs-iroh-agent
  concurrent = true
  concurrenttransfers = 4
```

Before Phase 2: file absent, vanilla HTTP LFS works. After Phase 2: committing `.lfsconfig` opts everyone in. Reverting is a one-line git revert.

---

## Confidence

- LFS custom transfer protocol: **high**
- Forgejo lock API independence from blob storage: **high**
- Forgejo "pointer-only" mode: **confirmed not available natively**
- UE plugin UE 5.3+ compatibility: **medium — needs direct confirmation**
- iroh-blobs 0.96 API stability: **medium**
- SHA256→BLAKE3 mapping requirement: **high**

## Sources

- [1] `https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md`
- [2] `https://github.com/pyelfs/pyelfs`
- [3] `https://github.com/nicolas-graves/lfs-s3`
- [4] `https://github.com/git-lfs/git-lfs/blob/main/docs/man/git-lfs-config.adoc`
- [5] `https://github.com/infrastlabs/lfs-caching`
- [6] `https://github.com/sinbad/lfs-folderstore`
- [7] `https://github.com/SRombauts/UEGitPlugin`
- [8] `https://deepwiki.com/go-gitea/gitea/4.3-git-lfs-integration`
- [9] `https://github.com/regen100/lfs-dal`
- [10] `https://forgejo.org/docs/latest/admin/config-cheat-sheet/`
- [11] `https://forgejo.org/docs/next/admin/setup/storage/`
- [12] `https://codeberg.org/forgejo/forgejo/src/tag/v1.21.2-1/services/lfs/locks.go`
- [13] `https://www.stevestreeting.com/2020/08/09/my-unreal-engine-vcs-setup-gitea--git--lfs--locking/`
- [14] `https://blog.rime.red/git-lfs-or-perforce-for-unreal-in-2024/`
- [15] `https://docs.iroh.computer/protocols/blobs`
- [16] `https://docs.rs/iroh-blobs/latest/iroh_blobs/`
- [17] `https://github.com/ProjectBorealis/UEGitPlugin`
- [18] `/Users/dukejones/work/Mjolnir/mjolnir-mesh/Cargo.toml:11-14` — workspace pins iroh 0.96, blake3 1

## Open Questions

1. Forgejo behavior when `lfs.standalonetransferagent` is set and no blob is ever POSTed — graceful or broken on fallback pulls? Needs a test instance.
2. UE 5.3/5.4/5.5 plugin compatibility (SRombauts / ProjectBorealis / getnamo) — needs direct confirmation.
3. iroh-blobs 0.96 API churn risk across patch releases.
4. Dual-hash perf on representative assets.
5. Peer availability for WAN teams (relay strategy).
6. `.lfsconfig` fallback UX when the binary is missing.

## Sub-Hypotheses

- **H2a** — Forgejo batch-API standalone mode (source-code investigation into whether upload validation can be skipped).
- **H2b** — iroh-blobs 0.96 store API stability and dual-hash indexing cost benchmark.
