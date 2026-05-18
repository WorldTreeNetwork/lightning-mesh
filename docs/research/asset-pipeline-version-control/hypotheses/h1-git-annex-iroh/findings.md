# Hypothesis: H1 — git-annex with an Iroh-blobs "external special remote" is the best long-term fit for a p2p, content-addressed asset store

## Summary

git-annex is technically well-suited to content-addressed p2p asset storage and **has a native Iroh transport as of git-annex 10.20251103**, which sidesteps much of the custom-special-remote work. However, it falls significantly short of "best long-term fit" for Unreal game-studio workflows on two hard blockers: (1) Windows symlink handling is structurally broken for non-developer-mode users, forcing a degraded unlocked/pointer-file mode with rough edges for non-technical artists; (2) git-annex's lock/unlock semantics are fundamentally incompatible with Unreal-grade exclusive file locking — the maintainer himself confirmed this is architecturally impossible without a centralized server. The iroh-blobs special remote does not yet exist as a published artifact; it would need to be built, and the key-mapping problem (SHA256 annex keys → BLAKE3 hashes) is solvable in ~300–500 LoC with a sidecar index, analogous to what `git-annex-remote-ipfs` does for CIDs.

---

## Evidence

### 1. External Special Remote Protocol — Actual Wire Format [source 1]

Synchronous, line-oriented stdio. Remote binary is `git-annex-remote-$name` on PATH.

**Handshake:**
```
VERSION 2
EXTENSIONS INFO ASYNC GETGITREMOTENAME UNAVAILABLERESPONSE ...
PREPARE
PREPARE-SUCCESS   # or PREPARE-FAILURE msg
```

**Mandatory commands (minimal surface):**

| Command | Reply |
|---|---|
| `INITREMOTE` | `INITREMOTE-SUCCESS` / `INITREMOTE-FAILURE msg` |
| `TRANSFER STORE Key File` | `TRANSFER-SUCCESS STORE Key` / `TRANSFER-FAILURE STORE Key msg` |
| `TRANSFER RETRIEVE Key File` | `TRANSFER-SUCCESS RETRIEVE Key` / … |
| `CHECKPRESENT Key` | `CHECKPRESENT-SUCCESS` / `-FAILURE` / `-UNKNOWN` |
| `REMOVE Key` | `REMOVE-SUCCESS` / `-FAILURE msg` |
| `GETCOST` | `COST <int>` |
| `GETAVAILABILITY` | `AVAILABILITY GLOBAL|LOCAL|UNAVAILABLE` |

**Optional but useful:** `CHECKURL`, `CLAIMURL`, `WHEREIS`, `GETSTATE`/`SETSTATE`, `PROGRESS`, `SETURIPRESENT`, `DEBUG`/`INFO`. Extensions include `ASYNC` (concurrent transfers), `DELEGATE`, `UNAVAILABLERESPONSE`.

**Complexity:** Python `AnnexRemote` subclass exposes 6 methods [source 2]. `git-annex-remote-ipfs` shell implementation stores `ipfs:<CID>` via `SETURIPRESENT` and retrieves via `GETURLS` [source 3]. Minimal Rust impl covering the 6 mandatory commands + state persistence + progress: realistically ~350–500 LoC (excluding the iroh-blobs calls).

No published Rust external special remote exists as of April 2026.

### 2. git-annex Windows Reality (2024–2026)

**Symlinks structurally broken:** git-annex unconditionally assumes Windows filesystems are crippled [source 4]. Bug open since Aug 2021 notes NTFS has supported symlinks via Developer Mode for years but git-annex ignores it [source 5]. Consequence: Windows repos auto-enter "unlocked" mode (pointer + hardlinks):
- Files appear as regular files (artist-friendly)
- `annex.thin` (hardlink dedup) does not work on NTFS [source 6]
- After `git stash` or `git reset --hard`, artists must run `git annex smudge --update` manually
- `adjusted/master(unlocked)` branch handles this on init but is fragile across non-annex-aware GUIs

**Path length:** Windows 260-char MAX_PATH can be exceeded by deeply nested `.git/annex/objects/XX/YY/SHA256--<64>/SHA256--<64>` paths.

**Bottom line for artists:** Degraded-but-functional in unlocked mode; `smudge --update` footgun makes it unsuitable without a GUI wrapper. git-annex assistant has Windows support but reliability for non-technical artists is not verified by independent 2024+ sources.

### 3. git-annex + Unreal Engine

**No documented real-world use.** Dominant tools are Perforce, Git LFS + locking, Plastic SCM [sources 7, 8]. Atomic "commit code + asset" works via pointer-files-in-git + content-in-annex. But `.uasset` is binary, non-mergeable, and changes frequently — absence of exclusive locks is a practical blocker.

**Editor integration:** No Unreal plugin for git-annex exists. The community Unreal git plugin integrates git + git-lfs-locking only.

### 4. git-annex Lock Semantics vs. Unreal Needs

| Feature | git-annex | git-lfs | Perforce |
|---|---|---|---|
| Scope | Local repo only | Centralized advisory | Centralized enforced |
| Effect | Local read-only/writable | Server lock + push warning | Blocks others' checkout |
| Cross-peer enforcement | None | Advisory (`--force` bypass) | Enforced |
| Implementable p2p | **No (per maintainer)** | n/a | No |

Joey Hess (maintainer, 2021) explicitly stated LFS-style exclusive locking "is fundamentally incompatible with git-annex's architecture" — git-lfs is centralized, git-annex generally is not. Workaround: pre-receive hook on a centralized git server rejecting pushes when another user holds a lock claim in git-annex metadata. Requires centralized remote + custom hook + all clients to respect the protocol. Not atomic; race conditions. No existing implementation found [source 9].

### 5. Content-Addressing and Retention — Concrete Topology

`numcopies` + `preferred-content` produces the desired retention [sources 10, 11].

```bash
git annex numcopies 2

# Laptops: keep present, drop what group peers hold
git annex wanted alice   "standard"
git annex wanted bob     "standard"
git annex wanted carol   "standard"

# OpenWRT SSD warm peer: archive group, keep everything
git annex group openwrt-ssd archive
git annex wanted openwrt-ssd "groupwanted"   # archive group = anything
```

**Behavior:** When Alice runs `git annex drop hero.uasset`, it verifies 2 copies elsewhere (OpenWRT + Bob). Warm peer `preferred-content = anything` retains old versions; laptops stay lean. `git annex sync openwrt-peer --content` pushes/pulls per preferred-content expressions. Balanced distribution via `balanced=teamgroup:2`.

### 6. Iroh-Blobs as Special Remote Backend

**Native Iroh transport already exists:** git-annex 10.20251103 added `git annex p2p --enable iroh` using `dumbpipe 0.33+` and magic-wormhole pairing [source 12]. This is a **transport**, not a special remote — two git-annex repos sync directly over Iroh QUIC. Implications:
- `git annex sync peer --content` over Iroh works without custom code
- Location tracking is native
- No BLAKE3 ↔ SHA256 mapping needed — Iroh is just the tunnel
- Limitation: one-to-one pairing; N peers = N pairings; daemon must run continuously

**What a dedicated `git-annex-remote-iroh` adds vs. native transport:** A *store* — content in iroh-blobs' BLAKE3 store fetchable by hash by anyone with access to the iroh node, without pairing as a git-annex remote (IPFS analogy).

**Key-mapping sidecar:** git-annex keys are `SHA256--<64>`; iroh-blobs uses 32-byte BLAKE3 root hashes. Store mapping via `SETURIPRESENT Key iroh:<blake3-hex>` (same pattern as `git-annex-remote-ipfs`'s `ipfs:<CID>`).

**Rough LoC estimate for `git-annex-remote-iroh` (Rust):**

| Component | LoC |
|---|---|
| stdio protocol framing | ~150 |
| INITREMOTE / PREPARE | ~50 |
| TRANSFER STORE (BLAKE3 while uploading) | ~80 |
| TRANSFER RETRIEVE | ~60 |
| CHECKPRESENT | ~40 |
| REMOVE | ~30 |
| Key→BLAKE3 mapping (GETSTATE/SETSTATE) | ~80 |
| Progress + error handling | ~50 |
| **Total** | **~540** |

`iroh-blobs` Rust crate provides `blobs.add_bytes()` / `get()` / `has()` / `delete()` mapping cleanly onto the 4 core ops [source 13].

**Complication:** iroh-blobs uses BLAKE3 tree hashing with 16 KiB chunks and outboard metadata; the hash is the tree root, not single-pass. Dual-hash (SHA256 for git-annex key, BLAKE3 for iroh) is unavoidable unless git-annex is extended with a BLAKE3 key backend.

### 7. DataLad as Prior Art

DataLad wraps git-annex with dataset conventions and Python CLI/GUI [source 14]. Relevant prior art:
- `datalad-installer` solves cross-platform git-annex install including Windows (reusable)
- Dataset nesting (`subdataset`) maps to "project with asset library"
- Does NOT fix lock semantics or Windows symlinks
- Forgejo-aneksajo (git-annex/DataLad forge) presented at distribits 2025 — active ecosystem [source 15]
- Scientific, not game-industry user base; artist-unfriendly UX

### 8. Real-World Use in Media / Game Studios

No evidence of git-annex adoption by game studios [sources 7, 8, 16]. Known uses: scientific data (OpenNeuro, DataLad), personal media archives, academic paper+data bundles.

**Why not game studios?**
1. Windows non-negotiable; git-annex's Windows story is structurally degraded
2. Exclusive locking non-negotiable for binaries; git-annex cannot provide it
3. Perforce inertia
4. Git LFS captured the indie segment with a simpler model
5. Steep learning curve — ~40 subcommands [source 17]

---

## Confidence

**Level**: high. Primary sources (git-annex docs, maintainer forum posts, native Iroh integration page, IPFS remote, DataLad) converge. LoC estimate is analysis-based, medium confidence.

## Sources

- [1] https://git-annex.branchable.com/design/external_special_remote_protocol/
- [2] https://github.com/Lykos153/AnnexRemote
- [3] https://github.com/NII-DG/git-annex-remote-ipfs
- [4] https://git-annex.branchable.com/tips/unlocked_files/
- [5] https://git-annex.branchable.com/bugs/Windows__58___support_NTFS_symlinks/
- [6] https://git-annex.branchable.com/bugs/Symlink_support_on_Windows_10_Creators_Update_with_Developer_Mode/
- [7] https://github.com/getnamo/GitSourceControl-Unreal
- [8] https://www.anchorpoint.app/blog/git-with-unreal-engine-5
- [9] https://git-annex.branchable.com/forum/Git_LFS_lock_feature/
- [10] https://git-annex.branchable.com/git-annex-numcopies/
- [11] https://git-annex.branchable.com/git-annex-preferred-content/
- [12] https://git-annex.branchable.com/tips/peer_to_peer_network_with_iroh/
- [13] https://docs.rs/iroh-blobs/latest/iroh_blobs/
- [14] https://www.datalad.org/
- [15] https://www.distribits.live/talks/2025/risse-forgejo-aneksajo-a-git-annex-datalad-forge/
- [16] https://lwn.net/Articles/774125/
- [17] https://anarc.at/blog/2018-12-21-large-files-with-git/
- [18] https://github.com/n0-computer/iroh-blobs/blob/main/DESIGN.md

## Open Questions

1. Does the native git-annex Iroh transport use iroh-blobs as the blob store, or is Iroh purely a QUIC tunnel for git-annex's own p2p protocol? **Load-bearing for the architecture decision.**
2. iroh pinned to 0.96 here; git-annex's native integration uses `dumbpipe 0.33+` — compatibility unknown.
3. Can the exclusive-lock requirement be relaxed for a small team (2–3 artists, low collision probability)? Advisory metadata + pre-receive hook may suffice.
4. Windows artist adoption path — is there a maintained GUI hiding the `smudge --update` footgun?
5. DataLad installer reuse — adapt to distribute bundled `git-annex-remote-iroh` alongside git-annex?
6. BLAKE3 key backend for git-annex — proposed/prototyped? Would eliminate the sidecar.

## Sub-Hypotheses

- **H1a** — git-annex native Iroh transport internals (iroh-blobs vs QUIC tunnel).
- **H1b** — Exclusive-lock sidecar via git-annex metadata + pre-receive hook — feasibility for 2–3 artists.
- **H1c** — BLAKE3 key backend in git-annex — eliminates the hash impedance mismatch.
