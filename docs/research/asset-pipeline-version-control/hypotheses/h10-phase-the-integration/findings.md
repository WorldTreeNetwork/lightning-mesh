# Hypothesis: H10 — Building Mjolnir-native asset sync is premature; phase it

## Summary

This hypothesis is **strongly supported**. Phased adoption is correct for this team at this stage. The core argument: artists need a working workflow now, the team's value-add is Mjolnir Mesh (not LFS plumbing), and the custom-transfer seam is clean enough that Phase 3 can be added later without disrupting in-flight work. The real risk is **not** building Phase 1 too early — it is that Phase 3 never gets prioritized because Phase 1 is "good enough." That risk is manageable with deliberate phase triggers.

---

## Evidence

### 1. Steel-man: "Adopt now, integrate later"

**Artists need days, not weeks.** The friend on Windows is the forcing function. Git + Git LFS is documented, stable, Google-able on Windows. A custom Iroh-blobs transfer agent requires compiled binaries on PATH — an unknown quantity for a non-technical collaborator.

**Opportunity cost is asymmetric.** Mjolnir Mesh is the team's primary research output. Every week on LFS plumbing is a week not on the audio pipeline or OpenWrt integration. LFS work is infrastructural, not differentiating.

**80/20 is favorable.** Forgejo + LFS covers: versioned binary storage, access control, partial clone, Windows compat, standard Git clients. The remaining 20% — P2P transfer, offline resilience, no-single-host — are real needs but not yet demonstrated pain.

**The seam is clean.** Git LFS custom transfer agent is JSON-over-stdio. Phase 3 plugs into `.lfsconfig` without touching `.gitattributes` or any artist workflow. Migration: (a) write agent, (b) update `.lfsconfig` on server, (c) push. Artists notice nothing.

### 2. Steel-man: "Integrate now"

**Host availability is the dealbreaker.** If the designated Forgejo host (dev box with SSD) goes offline — suspended, rebooted, subnet change — the team is blocked. For a hobbyist project, this is not hypothetical.

**Dogfooding / research output.** If the team plans to publish Mjolnir Mesh, a working git-lfs custom transfer agent on top of Iroh-blobs is a compelling interop demo.

**Asset size outliers.** Multi-GB raw audio sessions, large textures, video exports — if a single SSD fills up, Forgejo LFS doesn't help. But this is "when," not "now."

### 3. Phase Triggers: Concrete Signals

| Phase | Trigger Signal | Threshold |
|-------|---------------|-----------|
| 1 → 2 assessment | Forgejo host unavailability | Host unreachable >1x/week, blocking commits |
| 1 → 2 assessment | Repo total LFS size | LFS store exceeds 150 GB on the SSD |
| 1 → 2 assessment | Clone time | Fresh clone takes >15 min on LAN |
| 2 → 3 build | Assessment confirms need | Pain real AND a second always-on peer available |
| 2 → 3 build | Dogfooding opportunity | Demo/paper benefits from the integration |
| Skip to 3 | Structural | Dev box is not always-on (laptop, not desktop) |

A second always-on peer is a **hard prerequisite** for Phase 3 to be worth the complexity. A two-node P2P where one node is frequently offline is worse than single-host — split-brain without redundancy.

### 4. Risk Inventory: Will Phase 3 Ever Happen?

**The real risk: Phase 1 becomes permanent.** Once artists have a working workflow, pressure to improve disappears. Classic "good enough" trap — accumulated friction absorbs the pain invisibly.

**Mitigations:**
- **Install a size meter now.** CI check or pre-push hook logs LFS store size. When it crosses threshold, trigger fires automatically.
- **Write the Phase 2 assessment plan before Phase 1 ships.** Two-page doc of what to measure and what constitutes pain.
- **Scope down Phase 3 to maintain momentum** (see §7). If the first integration is "gossip the Forgejo IP on the LAN" rather than "replace the LFS protocol," Phase 3 is a two-day project.
- **Budget a timebox.** "If Phase 3 not started by [date 6 months out], do a one-week spike to check triggers."

### 5. Minimal Phase 1 Setup — Forgejo + LFS on a Dev Box, Arch + Windows Today

**Host (Arch Linux):**

```bash
yay -S forgejo
sudo systemctl enable --now forgejo
```

**`/etc/forgejo/app.ini`:**

```ini
[server]
HTTP_PORT = 3000
DOMAIN    = forgejo.local

[lfs]
ENABLED   = true
PATH      = /mnt/assets-ssd/forgejo-lfs
MAX_FILE_SIZE = 0
```

**Windows client (Git for Windows 2.x bundles LFS):**

```powershell
git lfs version
git clone http://forgejo.local:3000/team/mjolnir-assets.git
```

**`.gitattributes` (committed):**

```
# Audio
*.wav   filter=lfs diff=lfs merge=lfs -text
*.flac  filter=lfs diff=lfs merge=lfs -text
*.aiff  filter=lfs diff=lfs merge=lfs -text
*.mp3   filter=lfs diff=lfs merge=lfs -text

# Images / textures
*.png   filter=lfs diff=lfs merge=lfs -text
*.psd   filter=lfs diff=lfs merge=lfs -text
*.exr   filter=lfs diff=lfs merge=lfs -text
*.tga   filter=lfs diff=lfs merge=lfs -text

# Video
*.mp4   filter=lfs diff=lfs merge=lfs -text
*.mov   filter=lfs diff=lfs merge=lfs -text

# Archives / blobs
*.zip   filter=lfs diff=lfs merge=lfs -text
```

**`.lfsconfig` (committed):**

```ini
[lfs]
    url = http://forgejo.local:3000/team/mjolnir-assets.git/info/lfs
```

**Hostname resolution:** `/etc/hosts` entry on each client (manual) or mDNS via `avahi-daemon` on the host (zero-config on Linux; Windows needs Bonjour).

**Total time to working pipeline: ~2–3 hours.**

### 6. First Genuine Pain After Phase 1

**Most likely: host availability.** Dev box reboots. Artist tries `git push`, gets `failed to push some refs`. Mitigation: `systemd` auto-restart + document "is Forgejo down?" diagnostic URL.

**Second: LFS fetch on slow connections.** Remote work = slow pulls over internet. `git lfs fetch --include`/`--exclude` patterns.

**Phase 2 Assessment Plan (2-week window):**

| Metric | Measure | Tool |
|--------|---------|------|
| Forgejo uptime | Admin panel or cron-curl hourly | `cron` + log |
| LFS store growth | `du -sh /mnt/assets-ssd/forgejo-lfs` weekly | manual/cron |
| Clone time (fresh) | `time git clone --no-local` per OS | bash |
| Artist friction events | "How many LFS errors this week?" | async chat |
| Bandwidth | Forgejo admin → repos → traffic | built-in |

If "was Forgejo the bottleneck more than once?" = yes after 2 weeks, Phase 3 moves up the backlog.

### 7. Low-Hanging Mjolnir Integrations (No LFS Protocol Interception)

The spectrum between "do nothing mesh-related" and "write a full LFS custom transfer agent" has cheap, immediately useful middle ground.

**Option A — Gossip the Forgejo host's address via existing Mjolnir gossip (~50 LoC).** Existing gossip layer (see `docs/architecture/p2p-resilience.md`). Add `ForgejoAnnounce { url: String, host_id: NodeId }`. Host broadcasts once/minute; peers update local `.lfsconfig` override. Artists never update config when LAN IP changes. **One afternoon.**

**Option B — Fallback mirror via `git remote` + LFS URL override (0 LoC, config only).** Second git remote pointing at cloud backup. `git lfs fetch --all` nightly pushes LFS to mirror. Down-Forgejo fallback via pure Git config. Costs cloud storage; cold backup only.

**Option C — Iroh-blobs as a CDN layer alongside Forgejo (~1 week, strictly additive).** Run iroh-blobs node alongside Forgejo. Post-receive hook re-exports blobs into iroh-blobs. Clients with iroh fetch from mesh; clients without fall back to HTTP transparently. LFS protocol unchanged.

**Option A is the right first integration** — solves real pain (dynamic-IP discovery on LAN), uses existing gossip substrate, ships in a day without distracting from the audio pipeline.

---

## Confidence

**Level**: high. Architectural reasoning over well-understood problem space + knowledge of existing codebase capabilities. Team constraints described in hypothesis context; phased approach is a standard infra adoption pattern.

## Sources

- [1] `docs/architecture/p2p-resilience.md` — structured gossip layer; Option A feasible on existing substrate
- [2] `docs/plans/initiatives/dual-layer-architecture.md` — Iroh + MoQ primary focus; supports opportunity-cost argument
- [3] Hypothesis context (H10 brief) — team constraints, artist profile, Windows requirement
- [4] https://forgejo.org/docs/latest/admin/config-cheat-sheet/
- [5] https://git-lfs.com — custom transfer agent protocol; confirms clean `.lfsconfig` seam

## Open Questions

- **Is the dev box always-on?** Single most important unknown. If it's a sleeping laptop, Phase 3 trigger fires immediately.
- **Windows artist's Git literacy?** Influences GUI choice (GitHub Desktop has native LFS).
- **Does the team have a second always-on peer today?** Phase 3 P2P only makes sense with ≥2 persistent nodes.
- **Paper or demo planned for Mjolnir Mesh?** If yes, "integrate now" strengthens significantly.
- **SSD capacity?** 150 GB trigger is a guess; depends on actual SSD + generation rate.

## Sub-Hypotheses

- **H10a** — Gossip-based Forgejo host discovery: extensibility of existing gossip message types + Windows-side consumption story.
- **H10b** — iroh-blobs as transparent CDN alongside Forgejo: does the iroh-blobs crate already have an HTTP gateway serviceable as LFS-compatible without a custom transfer agent?
