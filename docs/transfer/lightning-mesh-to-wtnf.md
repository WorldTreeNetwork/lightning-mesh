# Transfer: Lightning Mesh → World Tree Network Foundation

Moving this project out of **Identikey Inc.** and into the **World Tree Network
Foundation** (WTNF). New home: **`github.com/WorldTreeNetwork/lightning-mesh`**.
Licensing / CLA / security contact: **duke@worldtree.io**.

Legal mechanism: the [CLA](../../CLA.md)'s *"successors and assigns"* clause
already runs the existing contribution grants to the Foundation — no
contributor needs to re-sign, and existing signatures in the `cla-signatures`
branch remain valid.

---

## 1. In-repo license / ownership sweep — ✅ done

Swept in the transfer commit. Every ownership/licensing surface now names the
Foundation and `duke@worldtree.io`:

| File | What changed |
|------|--------------|
| `NOTICE` | Copyright holder → World Tree Network Foundation; commercial-license contact email |
| `CLA.md` | "the Project" = World Tree Network Foundation (2 spots); individual + corporate contact email |
| `CONTRIBUTING.md` | Commercial licensor + SPDX header template + security-contact + relicensing line |
| `COMMERCIAL-LICENSE.md` | Licensing contact email |
| `README.md` | Commercial-license contact email |
| `crates/mjolnir-hello/src/main.rs`, `crates/mjolnir-mesh/src/lib.rs`, `crates/mjolnir-mesh/src/radio.rs`, `crates/mjolnir-meshctl/src/main.rs` | SPDX `Copyright (C) 2026 …` header |
| `.github/workflows/cla.yml` | `remote-organization-name` → `WorldTreeNetwork`; CLA-document URLs → new org |
| `.beads/issues.jsonl` (bd memory) | Copyright-holder memory updated to the Foundation |

Note on the copyright line: history was rewritten to the Foundation rather than
appended (pre-1.0, effectively single-author). If a lawyer prefers to *preserve*
the historical Identikey line and add the Foundation alongside, that's a
one-line change to the header string — flag it and I'll redo it that way.

**No action, cosmetic only:** `deploy/openwrt/l23-port/*` and
`docs/**` prose reference `IdentiKey` as the *identity subsystem / product name*
(e.g. "IdentiKey key-based auth"), not the corporate owner — these are correct
and stay. Local build paths like `/home/dorje/work/IdentiKey/openwrt-l23` are a
contributor's working directory, not ownership.

## 2. GitHub repository transfer

- [ ] **Create / confirm the `WorldTreeNetwork` org** and that Duke has owner rights.
- [ ] **Merge the transfer commit** (this sweep) to `main` on the old
      `identikey/lightning-mesh` *before* transferring, so the new org's default
      view is already correct.
- [ ] **Transfer the repo**: old repo → Settings → *Danger Zone* → *Transfer
      ownership* → `WorldTreeNetwork`, repo name `lightning-mesh`. GitHub keeps
      issues, PRs, stars, and sets up automatic redirects from the old URL.
- [ ] **Update local remotes** (redirects work, but make it explicit):
      ```bash
      git remote set-url origin git@github.com:WorldTreeNetwork/lightning-mesh.git
      git remote -v   # confirm
      ```
- [ ] Anyone else with a clone updates their remote the same way.

## 3. Post-transfer wiring (things that do NOT travel with a transfer)

- [ ] **Actions secrets** — re-create under the new repo/org. The CLA workflow
      needs `PERSONAL_ACCESS_TOKEN` (repo scope, owned by someone in
      `WorldTreeNetwork`) to write to the `cla-signatures` branch. Without it the
      CLA bot silently fails.
- [ ] **`cla-signatures` branch** — travels with the repo; confirm it arrived
      and that CLA Assistant still resolves signatures against the new org.
- [ ] **Branch protection / rulesets** on `main` — re-apply (protections do not
      transfer).
- [ ] **Actions enablement** — confirm Actions is enabled and the org's Actions
      policy permits the third-party `contributor-assistant/github-action` and
      `dependabot`.
- [ ] **`allowlist` in `cla.yml`** — currently `dukedorje,dependabot[bot],*[bot]`;
      confirm the maintainer handle is right for the new org.
- [ ] **Webhooks / integrations / deploy keys** — none known in-repo; audit the
      old repo's Settings before it's gone.

## 4. Identity & contact plumbing

- [ ] **`duke@worldtree.io` mailbox is live and monitored** before publishing —
      it's now the licensing, CLA-corporate, and security-disclosure address.
- [ ] Confirm the `worldtree.network` / `worldtree.io` domains are Foundation-held
      (README already references `vm.worldtree.network`).

## 5. The talk beat — ✅ drafted

Added to `docs/talk/dweb-2026-talk-script.md`, Beat 11 ("Locked open"), as a
quiet ownership admission right before the closing network callback: *"This
project used to belong to my company. As of this week, it doesn't."* It's the
thesis enacted at the ownership layer (no throat — the speaker removes himself
as center) and sets up "it isn't mine to turn off." Delivery note added
alongside. Easily moved or cut if you'd rather keep it out of the stage script.

## 6. Announce

- [ ] Foundation announcement / README badge once the transfer lands ("as of
      this week, Lightning Mesh belongs to the World Tree Network Foundation").
- [ ] Optionally tag a release at the transfer commit as the provenance marker.
