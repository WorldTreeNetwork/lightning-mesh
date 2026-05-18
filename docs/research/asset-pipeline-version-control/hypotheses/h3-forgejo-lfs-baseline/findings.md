# Hypothesis: H3 — Boring-but-correct baseline: Forgejo + self-hosted git-lfs on an OpenWRT-attached SSD (or any dev-box host), with LFS file locks

## Summary

The hypothesis is **largely confirmed with important caveats**. Forgejo/Gitea + Git LFS + file locks is a well-trodden path for small indie/Unreal teams and clearly beats GitHub LFS on cost and LAN latency. However, OpenWRT itself is not a viable direct host — a Raspberry Pi 4 (aarch64) or any x86 dev box is the realistic host. LFS lock scalability is the primary failure mode at scale (8,000+ locks reportedly took 12–24 hours on GitHub; smaller on self-hosted but the architectural risk is real). For a 3–5 person LAN team with ~50 GB LFS, this setup is genuinely workable with modest operational discipline.

---

## Evidence

### 1. OpenWRT as a Forgejo Host

**Verdict: Not directly viable as a managed service; barely viable as a manual binary install.**

Forgejo/Gitea publishes no official `opkg` package for OpenWRT. Gitea issue #5674 requesting an OpenWRT `.ipk` was closed as "wontfix" [5]. One documented community case (goozenlab, 2020) ran Gitea 1.12 on a Pi under OpenWRT by manually installing the Go binary [4]:
- Required a non-login `gitea` user, `/opt/gitea/`, `/srv/git/`
- Required `git`, `git-http`, `openssh-keygen`, `bash`
- init.d script "starts but crashes within seconds" — resorted to a custom shell script
- System "at its limit" alongside Syncthing, InfluxDB, Adblock

**Realistic host targets for a small studio:**

| Host | RAM | Cost | Verdict |
|---|---|---|---|
| Raspberry Pi 4 (2 GB) | 2 GB | ~$45 | Minimum viable |
| Raspberry Pi 4 (4 GB) | 4 GB | ~$60 | Comfortable for 3–5 |
| Intel NUC / mini-PC | 8–16 GB | ~$150–300 | Preferred with Postgres |
| Dev laptop (always-on) | 8+ GB | $0 | Common small-studio pattern |

Gitea idles at ~120 MB as a single Go binary; 1 GB+ recommended with PostgreSQL; 50 GB+ SSD for LFS objects. Local filesystem storage (`STORAGE_TYPE = local`) is simplest.

### 2. Forgejo LFS Storage Backends

```ini
# Local (default, simplest)
[server]
LFS_START_SERVER = true
[lfs]
STORAGE_TYPE = local
PATH = /path/to/lfs
```

```ini
# MinIO / S3-compatible (SSD-on-router pattern)
[lfs]
STORAGE_TYPE = minio
MINIO_ENDPOINT = 127.0.0.1:9000
MINIO_BUCKET = forgejo
MINIO_BASE_PATH = lfs/
MINIO_USE_SSL = false
```

S3 compatibility tested against MinIO and Garage v0.8.2 [7]. There is no native "point Forgejo at an SMB share" option.

**SSH LFS bug:** Pure SSH LFS is disabled by default due to a git-lfs client bug. HTTP/HTTPS LFS is the recommended and default protocol; apply `git config --global lfs.ssh.automultiplex false` on all clients if SSH LFS is attempted [8].

### 3. SMB Mount for LFS Storage vs. HTTP LFS

**Verdict: HTTP LFS via one shared Forgejo instance is correct. SMB mounts have known pathologies.**

Git LFS supports only HTTP/HTTPS and `file://`; it cannot push over SSH natively (as of git-lfs 2.13.3+). git-lfs issue #4902 documents that mounting a network share as `lfs.storage` and attempting to clone fails on Ubuntu 20.04 — read-only network-mounted storage is not reliably supported [14].

**Correct pattern:** Run Forgejo on any always-on host, point `[lfs] PATH` at the attached SSD (ext4/btrfs), serve all LFS over HTTP. SSD attachment is transparent to clients.

### 4. Small-Studio Narratives

**Steve Streeting (2020, updated Sept 2022) [1]:** Gitea in Docker on LAN. Rejected plain Git + NFS-LFS (no locking) and GitLab (too heavy). Uses ProjectBorealis UEGitPlugin with UE5.
- `lockable` files become read-only after checkout; UE "Checkout" button runs `git lfs lock`
- **Stale lock failure mode:** pushing from CLI (e.g., merge commits) does not auto-unlock — requires `git lfs unlock <file>` or Gitea web UI
- Force-unlock risks losing someone's changes; read-only attribute can desync
- "Storage much cheaper vs. GitHub's $5/month per 50GB" on self-hosted LAN

**Rime.red (2024) [2]:** Production analysis of the worst-case LFS lock scaling:
- 8,000+ locks on GitHub: each unlock took multiple seconds; projected 12–24h to clear all → "completely unacceptable"
- Personal Gitea: "I've never encountered this issue"
- Locks are global across branches — friction in feature-branch workflows
- For non-technical artists, recommends Perforce free tier (< 5 seats) for native UE integration

**Anchorpoint.app (2023–2024) [3]:** Game-dev-focused Gitea guide using cloud VPS + MinIO object storage:
- 2 GB RAM VM at $10–14/month (Vultr) + MinIO ~$6/month for 1 TB
- **Avoid built-in UE Git plugin** (perf degrades from post-save file scanning); use ProjectBorealis
- "File locking via CLI only propagates after `git fetch`"
- Recommends World Partition to split levels and reduce lock contention

### 5. LFS Lock Workflow in Unreal

**Plugin options:**
- **SRombauts/UEGitPlugin** — original "Git LFS 2"; UE4.7–UE5.2
- **ProjectBorealis/UEGitPlugin** — production-hardened refactor, UE5, actively maintained

**Happy path:**
1. Source Control toolbar → "Checkout" → LFS lock acquired, file becomes writable
2. Edit + save
3. "Submit Content" → commit + push + auto-unlock
4. Other artists see lock cleared after their next `git fetch`

**Failure modes:**
- **Stale lock from non-UE push:** manual `git lfs unlock`
- **Read-only attribute desync:** artist must `chmod +w` or delete and re-pull
- **Lock perf at scale:** 8,000+ locks problematic; small teams rarely hit this
- **GUI support gap:** SourceTree/Fork/GitKraken don't expose LFS lock status — UE editor + CLI only
- **Admin force-unlock:** `git lfs unlock --force <path>` (needs push access) or Forgejo web UI

### Canonical `.gitattributes` (ProjectBorealis/PBCore, real production) [6]

```gitattributes
[attr]lock filter=lfs diff=lfs merge=binary -text lockable
[attr]lockonly lockable
[attr]lfs filter=lfs diff=lfs merge=binary -text
[attr]lfstext filter=lfs diff=lfstext merge=lfstext -text

# Unreal Engine file types.
*.uasset lock
*.umap lock
*.locres lfs
*.locmeta lfs

# Steam Audio
*.phononscene lfs
*.probebox lfs
*.probebatch lfs
*.bakedsources lfs

# Binaries
*.exe lfs
*.dll lfs
*.rcc lfs

# Audio
*.bank lfs
*.wav lfs
*.mp3 lfs
*.ogg lfs
*.flac lfs

# Images
*.png lfs
*.ico lfs
*.icns lfs

# Movies
*.bk2 lfs
```

`.uasset` and `.umap` use `lock` (`lockable`); audio/texture/binary assets use plain `lfs` without mandatory locking — locking everything causes unnecessary contention.

### 6. History Hygiene

| Config var | Default | Effect |
|---|---|---|
| `lfs.fetchrecentrefsdays` | 7 | Branches/tags with commits within window are "recent" |
| `lfs.fetchrecentcommitsdays` | 0 | Extra commits per recent branch; 0 = tip only |
| `lfs.pruneoffsetdays` | 3 | Buffer before pruning |
| `lfs.fetchrecentremoterefs` | false | Include remote refs |

**`git lfs prune` keeps:** current checkout, all stashes, unpushed commits, files on branches within `fetchrecentrefsdays`, other worktrees.

**Critical caveat:** Orphaned commits in reflog NOT considered — LFS objects only referenced by orphaned commits are always deleted. Run `git lfs push --all origin` before pruning [9][10].

**Recommended `.lfsconfig` for a small studio:**
```ini
[lfs]
    fetchrecentrefsdays = 14
    fetchrecentcommitsdays = 3
    pruneoffsetdays = 7
    pruneverifyremotealways = true
```

**"Warm peer" pattern:** No built-in designation. In practice:
1. Nominate an always-on host (Forgejo server itself)
2. Set `lfs.fetchrecentrefsdays = 36500` on its local clone, OR
3. Keep LFS storage on the Forgejo server and rely on its lack of automatic pruning
4. Use `git lfs fetch origin --all` on the warm peer after each push

**Partial clone interaction:** `--filter=blob:none` + LFS can cause confusing checkout failures — safer pattern is LFS-only without partial clone and rely on `fetchrecentrefsdays`.

### 7. Why Teams Leave GitHub LFS / Does Forgejo Solve It?

| Pain point | GitHub LFS | Self-hosted Forgejo |
|---|---|---|
| $5/month per 50 GB pack | Expensive for 500 GB+ | SSD cost only |
| Bandwidth throttling | Throttles on bursts | LAN gigabit, no throttle |
| Re-clone cost | Whole LFS history | Same risk, LAN-fast |
| CI/CD | Every runner downloads LFS | Self-hosted CI avoids egress |
| Lock perf at scale | 8,000+ locks = 12–24h | Smaller teams don't hit |
| **Availability** | 99.9%+ uptime | LAN host down = team blocked |

**Solves:** per-GB cost, bandwidth cost, small-scale lock perf, retention control.
**Does NOT solve:** Lock UX for non-technical artists, stale lock management, **availability** (the most significant regression), global lock scope.

---

## Confidence

**Level**: high. Multiple independent sources converge; OpenWRT finding supported by both the wontfix issue and community experience.

## Sources

- [1] https://stevestreeting.com/2020/08/09/my-unreal-engine-vcs-setup-gitea-git-lfs-locking/
- [2] https://blog.rime.red/git-lfs-or-perforce-for-unreal-in-2024/
- [3] https://www.anchorpoint.app/blog/install-and-configure-gitea-for-lfs
- [4] https://goozenlab.github.io/blog/2020/05/openwrt-gitea/
- [5] https://github.com/go-gitea/gitea/issues/5674
- [6] https://github.com/ProjectBorealis/PBCore/blob/main/.gitattributes
- [7] https://forgejo.org/docs/latest/admin/storage/
- [8] https://docs.gitea.com/administration/git-lfs-setup
- [9] https://manpages.debian.org/testing/git-lfs/git-lfs-prune.1.en.html
- [10] https://proinsias.github.io/til/Git-Git-lfs-fetch/
- [11] https://miltoncandelero.github.io/unreal-git
- [12] https://about.gitea.com/resources/tutorials/game-development-on-gitea-cloud
- [13] https://stackoverflow.com/questions/32927704/how-to-specify-where-git-lfs-files-will-be-stored
- [14] https://github.com/git-lfs/git-lfs/issues/4902

## Open Questions

1. Pi 5 (8 GB) benchmark for Forgejo + LFS + PostgreSQL + CI runners for a 3–5 person team.
2. Does Forgejo's "Git Repositories GC" prune unreferenced LFS server-side, or does storage grow unboundedly?
3. Partial clone + LFS interaction in UE 5.3+ (engine source uses partial clone).
4. Lock visibility in Unreal's native source control UI post-UE 5.1 — has Epic added LFS lock support?
5. Availability SLA: secondary bare Git clone fallback vs. Forgejo built-in mirroring — which works for LAN teams?

## Sub-Hypotheses

- **H3a** — Pi 4 (4 GB) Forgejo + Postgres throughput for 3 concurrent users pushing 500 MB LFS objects.
- **H3b** — Forgejo server-side GC: does it reclaim deleted LFS storage?
- **H3c** — UE 5.3+ built-in Git source control plugin LFS-lock support status.
