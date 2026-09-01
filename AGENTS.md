# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for full workflow context.

Living specs and in-flight changes live in `openspec/`. Hard-won facts
go in `docs/LEARNINGS.md`. Shape notes go in `ARCHITECTURE.md`.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>         # Complete work
# Beads sync via the git-committed .beads/issues.jsonl (dolt is embedded, no
# remote) — do NOT run `bd dolt push`; a plain `git push` carries beads too.
```

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
   (Beads sync via the git-committed `.beads/issues.jsonl`; dolt is embedded with no
   remote, so do NOT run `bd dolt push`.)
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

## Live two-node event pair (AP3000 + DWeb m3000)

Hardware bring-up / radio matching lives in
[`deploy/openwrt/NOTES-dweb-ap3000.md`](deploy/openwrt/NOTES-dweb-ap3000.md).
Live RF/LAN snapshot (BSSIDs, WAN leases, what is actually on the air):
[`deploy/openwrt/fleet.yml`](deploy/openwrt/fleet.yml). `update-fleet.sh` still
walks `fleet-nodes.conf` only.

- Live pair client SSID `Lightning Mesh` is **OPEN** on both APs
  (`encryption=none`, no password). Factory default in `setup-wireless.sh`
  is `⚡` (U+26A1), still open. That string is a **network name**, not a
  guild; association is not membership; `mjolnir-apply` radio writes do
  not touch identikey-log. `mjolnir-mesh` is the 802.11s **backhaul** id
  (beaconed, not the phone SSID).
- Do not add `192.168.1.1/24` onto AP3000 `br-lan`. Do not strip AP3000's
  babel `0.0.0.0/0` export — that node is the house gateway (`gateway=auto`).
- Do not “fix” remaining diffs in the notes file unless asked.

## Mesh errand (no-internet client SSID)

Skill: `.grok/skills/mesh-errand` (`/mesh-errand`).

The agent laptop has **one** Wi-Fi radio. Joining the client SSID drops
Pirate Radio / Origami and the session cannot report. Never leave the radio
on the test SSID. Never `ip route replace default via 10.42.x.1` while the
chat uplink is the working one.

```bash
deploy/openwrt/mesh-errand.sh
ERRAND='ssh -o BatchMode=yes -o ConnectTimeout=5 root@$GW logread | tail -40' \
  deploy/openwrt/mesh-errand.sh
```

`ERRAND` is the on-box work. Empty = DHCP + ping `$GW`. Always restores
(`trap` EXIT/INT/TERM). `GATEWAY_BLACKHOLE` = lease but `.1` silent (dual
`192.168.1.1/24` on `br-lan`).

## Lightning Admin Windows build (morphist-win11)

Recipe: [`admin/scripts/windows-build/README.md`](admin/scripts/windows-build/README.md).

- Domain is libvirt `morphist-win11` (`sudo virsh start`), SSH alias `win11`
  → `192.168.122.214` user `A`. Guest is often shut off. Wait for
  `sudo virsh net-dhcp-leases default` before ssh.
- Do **not** run Papyrus `provision-vm.sh` (quickemu / `localhost:22220`).
- `admin/scripts/windows-build/release.sh` — `survey` / `smoke` / `bundle`
  (MSI+NSIS → gitignored `admin/dist-windows/`). `bootstrap` is idempotent.
- No scp. Fetch must strip the PowerShell SSH banner (marker in `release.sh`);
  a raw `ssh | tar` is not a tar archive.
- `git push` against the HTTPS origin prompts; push with
  `git push git@github.com:WorldTreeNetwork/lightning-mesh.git HEAD:main`.

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
