# Information Architecture: Lightning Admin home

The operator's window. One person, one selected group, two levels deep.
Home is where they spend the time. Other screens open off it and come back.

Structural calls, taken from the brief:

- Ranked by how often they happen: see who answered, switch group, open a router or the map, change the group's Wi-Fi and local network, look at Nearby, open Services, become the admin.
- Depth is two. The deepest page is `/you/become`.
- One entry. Being an admin changes what the status line suggests and what the group page will edit. It does not add a second app.
- Growing lists are saved groups, the router row, the roster, and Nearby. The page set is fixed.
- `/` and `/become` already exist. This map extends them. `/become` keeps working and lands on `/you/become`.

## Site Map

- Home `/`
  - Group `/group`
  - Map `/map`
  - Router `/router/[id]`
  - Nearby `/nearby`
  - Services `/services`
  - You `/you`
    - Become the admin `/you/become`

The group switcher is part of Home. It has no page of its own.

## Navigation Model

- **Header:** full width. The group switcher is on the left. Appearance is on the right. The header is not a second copy of the sections.
- **Primary navigation:** a left bar directly under the header, beside the page. Sections, in order: Home, Group, Map, Nearby, Services, then You after a gap. Each row is an icon plus a label. The current section is marked. The bar collapses to icons only, and that choice persists.
- **Secondary navigation:** sections on a page, not more routes. Group stacks group name, Wi-Fi name, local-network settings, and the roster. You stacks this person's settings, then Become the admin.
- **Utility navigation:** Appearance, in the header. Plain, Cyberpunk; System, Light, Dark. It is not a page.
- **Narrow window:** the same bar, collapsed to icons unless they have expanded it. The router button row scrolls sideways.

Router and Become the admin are not sections. They keep a back control. Back returns along history. The section bar stays visible on those pages.

## Content Hierarchy

### Home `/`
1. Selected group name, with the switcher -- this is the house they are looking at.
2. Status line `3/5` -- heard on the last scan, over routers assigned to this group. This is the sentence of the page.
3. Expanded router buttons -- one per heard router, left to right, router name on the button. Hidden until the status line is expanded.
4. Next action, only when they are not an admin -- one line that opens You. The sections themselves live in the left bar, not on this page.

### Group `/group`
1. Group name -- the membership name, editable.
2. Wi-Fi name -- the name phones join, editable on its own.
3. Local-network settings -- the other features of this group's network.
4. Roster -- every router assigned to the group. Heard rows look present. Rows this scan missed look quiet and read "Not heard".

### Map `/map`
1. The selected group's name and `3/5`.
2. The map of that group's routers.
3. A stub may say what will be drawn here. The back control is real.

### Router `/router/[id]`
1. Router name.
2. Heard or not heard on the last scan.
3. A stub body is enough. The back control is real.

### Nearby `/nearby`
1. Every machine this computer can hear on its links.
2. The existing filters: resources, Lightning nodes, interface kind, show all.
3. Address detail stays behind an expand.

### Services `/services`
1. The title Services.
2. A stub for the service mesh. The back control is real.

### You `/you`
1. Who this person is (the actor id already shown in the guild UI).
2. Their role in the selected group.
3. The link "Become the admin".

### Become the admin `/you/become`
1. The title Become the admin.
2. The claim instructions that live on `/become` today.
3. Back to You.

## User Flows

### Open the window
1. User lands on Home.
2. User sees the last selected group and `3/5` from the last scan.
3. User expands the status line and sees the heard routers as named buttons.
   - If a saved group has no roster yet, the count may be a placeholder and the button row shows whoever the last scan heard.
   - If they are not an admin, a next-action line offers You.

### Switch group
1. User opens the switcher in the header.
2. Each row is a group name plus that group's `active/total`.
3. User chooses one.
4. Home replaces the name, the count, and the router buttons. Group, Map, and Router pages follow that selection.

### Open a router
1. User expands the status line on Home.
2. User activates a router button.
3. User arrives at `/router/[id]`.
4. Back returns to Home.

### Open the map
1. User activates Map in the left bar.
2. User arrives at `/map` for the selected group.
3. The bar stays. Map is the current section.

### Configure the local network
1. User activates Group in the left bar.
2. User sees the group name, the Wi-Fi name, the local-network settings, and the roster.
3. User edits the Wi-Fi name. The group name stays as it was.
4. User edits the group name. The Wi-Fi name stays as it was.
5. The bar stays. Group is the current section.

### Look around the link
1. User activates Nearby in the left bar.
2. User sees every machine on the link, and can filter to Lightning nodes.
3. The bar stays. Nearby is the current section.

### Become the admin
1. User activates You in the left bar, or follows the next-action line on Home.
2. User activates Become the admin.
3. User arrives at `/you/become` and follows the claim instructions.
4. Back returns to You. A further back returns to Home.
5. Visiting `/become` lands on `/you/become`.

## Naming Conventions

| Concept | Label in UI | Notes |
| --- | --- | --- |
| Saved house / guild | Group | The switcher and the Group page. |
| Guild's own name | Group name | Membership name. Editing it leaves the Wi-Fi name alone. |
| Client SSID | Wi-Fi name | The name phones join. The field that today says "Network name". |
| Assigned router heard on the last scan | The router's name | A button on Home. |
| Assigned router missing from the last scan | Not heard | Quiet roster row. Off the home button row. |
| Heard count / assigned count | `3/5` | Active over total, as of the last scan. |
| Every machine on the link | Nearby | Includes this computer, phones, gateways, and routers. |
| Overlay services | Services | Its own page. |
| This person | You | Last section in the left bar, after a gap. |
| Claim the owner role | Become the admin | A section under You. |
| Theme switch | Appearance | Plain, Cyberpunk. System, Light, Dark. |

## Component Reuse Map

| Component | Used on | Behavior differences |
| --- | --- | --- |
| Header | All pages | Group switcher and Appearance. |
| Section bar | All pages | Icon and label. Collapses to icons. Current section stays marked on Router and Become. |
| Back | Router, Become | Returns along history. Section pages use the bar instead. |
| Count `3/5` | Home status, switcher rows, Map, Group roster summary | Same meaning everywhere: heard / assigned, last scan. |
| Router button | Home, when the status line is expanded | Icon plus router name. Opens `/router/[id]`. |
| Roster row | Group | Present or "Not heard". |
| Nearby table | Nearby | The current discovery table, including filters and address expand. |
| Appearance menu | Header | Unchanged. |
| Role chip | You, Group | Existing admin / member / absent / no-seed colors. |
| Become flow | `/you/become` | The current `/become` page, with back to You. |

## Content Growth Plan

- Saved groups accumulate in the switcher. The list scrolls. No pagination at house scale.
- Heard routers accumulate in one sideways-scrolling row. They do not wrap into a second cluster.
- The roster grows on the Group page as a vertical list. Quiet rows stay in the list.
- Nearby grows with every machine on the link. Filters stay on that page: resources, Lightning nodes, interface kind, and show all.
- Services starts as a stub. A later list can grow there without a new route.

## URL Strategy

SvelteKit routes under `admin/src/routes/`. The app is a static SPA (`ssr = false`).

- Pattern: `/`, `/group`, `/map`, `/router/[id]`, `/nearby`, `/services`, `/you`, `/you/become`.
- Dynamic segment: `[id]` is the router's node id.
- Selected group: `?group=<id>` on Home, Group, and Map. The id is the group's keyspace id. Missing `group` means the last selection, then the only saved group.
- `/become` redirects to `/you/become`.
- Nearby keeps its filters in the page. They are not query parameters in this pass.
- A stub page uses the same path it will use when it is real.
