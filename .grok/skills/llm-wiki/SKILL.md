---
name: llm-wiki
description: >
  Ingest a source into the Lightning Mesh contributor wiki, query that wiki,
  lint it, or promote one accepted page into the shared notebook. Use when
  the user adds a paper, link, or note to the wiki, asks what the wiki knows,
  or says LLM wiki, Karpathy wiki, or "add this to my wiki". Writes only
  under the directory named by wiki/.contributor.
---

# LLM wiki

Compile sources into the Lightning Mesh notebook. The human picks sources and decides what is relevant. The agent maintains the pages.

This is a local derivative of the [Karpathy LLM wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) workflow, from the MIT skill by Yuhan Lei (`Astro-Han/karpathy-llm-wiki`). The license is `LICENSE` next to this file.

The directory rule lives in `wiki/SCHEMA.md`. Read it before writing.

## Where pages go

Read `wiki/.contributor`. Use the first non-empty line that does not start with `#`.

That line is `<name>`. The writable tree is `wiki/<name>/`. It has the same folders as the shared notebook (`concepts/`, `comparisons/`, `interop/`, `appendix/`, `research/`, `queries/`), plus `raw/` for immutable sources, and its own `index.md` and `log.md`.

If `wiki/.contributor` is missing, or the name violates `wiki/SCHEMA.md`, stop. Tell the user to copy `wiki/.contributor.example`. Do not invent a name. Do not write a shared page.

Ingest, query archival, and lint write only inside `wiki/<name>/`. The shared files (`wiki/index.md`, `wiki/log.md`, `wiki/concepts/`, and the other shared folders) stay as they are until the user asks to promote.

## Page shape

Follow `wiki/SCHEMA.md` frontmatter. A page that is not accepted into the shared notebook is `state: [FUTURE]`.

Immediately after the H1, keep the evidence header the linter reads:

```markdown
> Sources: Author, YYYY-MM-DD
> Raw: [short name](../raw/<category>/<file>.md)
> Updated: YYYY-MM-DD
```

Templates are in `references/` next to this file. A raw file is never rewritten after it is saved.

## Ingest

1. Fetch the source with the tools you have. If you cannot reach it, ask for a paste.
2. Save it as `wiki/<name>/raw/<category>/YYYY-MM-DD-slug.md`. Reuse a category folder from the schema. Slug from the title, kebab-case, at most 60 characters. Unknown published date: omit the date prefix and set Published to `Unknown`. On a name clash, append `-2`. Preserve the source text.
3. Search `wiki/<name>/` for the source's entities. Disposition is one of **New**, **Update**, **Disputed**, or **No material**. No material keeps the raw file, logs it, and stops.
4. Same thesis as an existing contributor page: merge, and add the raw link. New concept: a new page named for the concept, not for the source file. One source may update one page and create another.
5. Every number, date, and direct quote must be found in the raw file before it is written, and written as found. A value you cannot locate is omitted or stated without precision.
6. Search the contributor tree for claims this source changes. Mark a replaced claim **Outdated** and a disagreement **Disputed**. Do not edit `queries/` pages during a cascade (they are snapshots).
7. Update `wiki/<name>/index.md` with a list entry, the shared index's shape: `- [[slug]] — one line`. Refresh Updated only when the knowledge changed.
8. Append `wiki/<name>/log.md`:

```markdown
## [YYYY-MM-DD] ingest | <page title>
- Disposition: New
- Raw: wiki/<name>/raw/<category>/<file>.md
```

No material uses this heading, which the evidence check matches exactly:

```markdown
## [YYYY-MM-DD] ingest | no material: raw/<category>/<file>.md
- Disposition: No material
```

The raw path in that heading is relative to `wiki/<name>/`.

A research request that gathers several sources compiles one source at a time. A question that does not ask to store anything is a query, not an ingest.

## Query

Read `wiki/<name>/index.md` and the shared `wiki/index.md`, then search both trees. Say which tree a page is in. Prefer the pages over memory. Cite with links. Do not write a file.

When the user asks to archive the answer, write a new page under `wiki/<name>/queries/` using `references/archive-template.md`. Do not merge it into an existing page. Prefix the index summary with `[Archived]`. Log:

```markdown
## [YYYY-MM-DD] query | Archived: <page title>
```

## Lint

Check only `wiki/<name>/`.

Fix without asking: an index entry whose file or Updated date is wrong, a markdown link with exactly one matching file elsewhere in the contributor tree, a Raw link with exactly one matching file under `raw/`. A See Also target with no match is removed. Anything ambiguous is reported.

Then run:

```bash
python3 .grok/skills/llm-wiki/scripts/check_evidence.py wiki/<name>
```

Report the suspects. Do not change a fact to satisfy the script.

Also report, without fixing: contradictions, missing Status blocks, orphan pages, and concepts that need their own page.

Append:

```markdown
## [YYYY-MM-DD] lint | <N> issues found, <M> auto-fixed
```

## Promote

Only when the user asks to bring a specific concept, link, or paper into the shared notebook.

1. Read the contributor page and its raw file.
2. Write or update the matching shared page (`wiki/concepts/`, `wiki/comparisons/`, `wiki/interop/`, `wiki/appendix/`, `wiki/research/`, or `wiki/queries/`) using the schema frontmatter. Cite the contributor page or the raw file in `sources`.
3. Leave the contributor page and the raw file where they are. Note the shared path on the contributor page.
4. Append the shared `wiki/log.md` and the contributor log.

Do not promote a page because it looks interesting.
