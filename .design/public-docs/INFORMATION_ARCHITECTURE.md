# Information Architecture: Lightning Mesh user guide

## Site Map

- Start `/` and `/join`
  - Use the mesh `/join/person/*`
  - Set up a home `/join/house/*`
  - Build a router `/join/node/*`
  - Share a service `/join/publish/*`
  - Contribute `/join/contribute`

The source of every public page is `docs-web/content/`. Repository-only
architecture, research, decisions, and maintainer notes remain in `docs/` and
have no public route.

## Navigation Model

- **Primary navigation:** six task-oriented sections in the left sidebar.
- **Secondary navigation:** pages within the current section. Only the current
  section starts expanded; readers may open other sections as needed.
- **Utility navigation:** the Lightning Mesh title returns to the guide start.
- **Mobile navigation:** the existing menu button reveals the same collapsed
  section list.

## Content Hierarchy

1. Start by choosing a goal, not by reading a complete table of contents.
2. Keep ordinary use and home setup ahead of router building and development.
3. Show deployment status on individual pages so unfinished behavior is clear.
4. Keep implementation notes and speculative design out of the public source
   tree.

## User Flows

### Join an existing mesh

1. Open the guide.
2. Choose **Use the mesh**.
3. Connect to Wi-Fi, open hello.mesh, and optionally create an identity.

### Set up or extend a mesh

1. Open **Set up a home** for a pre-flashed router.
2. Continue to **Build a router** only when installing onto new hardware.
3. Verify operation, then optionally open **Share a service**.

## Naming Conventions

| Concept | Label in UI | Notes |
|---|---|---|
| Everyday client tasks | Use the mesh | Plain-language, device-oriented |
| Household operator tasks | Set up a home | Starts with the pre-flashed path |
| Hardware installation | Build a router | Signals technical work |
| App/device publishing | Share a service | Describes the outcome |

## Component Reuse Map

| Component | Used on | Behavior differences |
|---|---|---|
| Docs layout | Every guide page | Opens the section containing the current page |
| Markdown renderer | Every content Markdown file | Rewrites guide links to site URLs |
| Status chip | Sidebar items | Shows built, partial, or coming-soon status |

## Content Growth Plan

Add public pages only under `docs-web/content/` and assign them to one of the
existing task sections with frontmatter. Add a new section only when it serves
a distinct user goal; the collapsed navigation prevents other tracks from
growing the reader's initial list.

## URL Strategy

- Preserve existing `/join/...` URLs during the source move.
- Derive routes from Markdown paths under `docs-web/content/`.
- Rewrite relative Markdown links inside the public tree to site routes.
- Send links that escape the public tree to their GitHub repository page.
