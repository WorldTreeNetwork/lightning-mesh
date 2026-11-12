# Lightning Admin page critique

Looked at the running debug window and the route source. Bound to the Admin brief: home shows this group's routers, Backhaul is the radio and the client network name, Guild is membership and the roster, You holds Become the admin. Map, Services, and Router may stay stubs.

## Home

Actions possible now: read `active/total`, read whether the scan is running or failed, open each assigned router (heard or not), follow a link to You when this person is not an admin, follow a link to Guild when the group has no routers.

Actions that should be possible: start another scan from Home. The count is this page's subject, and the only Scan control lives on Nearby. The app already scans on open through `discovery.scan()`.

Design and feel: the house is a row of router tiles, quiet when not heard. That matches the later correction. The page is still mostly empty when the group has one router, which is honest. Required: a Scan control in the lead row.

## Backhaul

Actions possible now: read the radio explanation, type a network name, Apply when the field is non-empty, see applying / error / halted / updated.

Actions that should be possible: know whether a name is already loaded. Nothing in the app reads the SSID back, so the page must not pretend it has one. Required: while the field is empty and idle, say that no network name is loaded, that the operator types the client SSID, and that this is not the guild name.

Design and feel: the explanation and the apply bar are the right density. The blank field looks like a missing value. Required: that empty-state sentence.

## Guild

Actions possible now: read the membership explanation, see identity, switch guilds, rename, assign a node, admit, open nodes / roster / ops / id, read Heard and Not heard for assigned routers.

Actions that should be possible: those actions already exist. No new guild wire op.

Design and feel: the panel is the dense console. The identity id wraps so the last characters sit alone (`overflow-wrap: anywhere` in a half-width card). Required: keep the id on one line and scroll it inside the card.

## Map

Actions possible now: read the selected group name and the stub sentence. The section bar is the way back.

Actions that should be possible: drawing the network. Deferred. The brief allows this stub.

Design and feel: one sentence in the shell. Acceptable until a map exists. No required change.

## Nearby

Actions possible now: scan, filter ULA / link-local / all, copy the list, open the canvas, filter by interface, sort columns, expand a resource that has more than one address.

Actions that should be possible: read State without it being crushed, and read Via without the same gateway repeated on several lines. Both already have the data.

Design and feel: this is the working neighbor table and it belongs here. Via joins every gateway with a comma, so "Gateway" stacks. State then loses its width. Required: show one via plus a count of the rest, keep the full list on the title, and keep State on one line with horizontal scroll instead of clipping.

## Services

Actions possible now: read the selected group name and the stub sentence.

Actions that should be possible: listing the service mesh. Deferred. The brief allows this stub.

Design and feel: same as Map. No required change.

## You

Actions possible now: read the actor id (or "no seed"), read the role in the selected group, open Become the admin.

Actions that should be possible: those three. Appearance stays in the header. No new settings.

Design and feel: the facts are a bare paragraph and an underlined link, thinner than Become and Guild. Required: put the id and the role in one surface card, keep the id on one scrolling line, and make Become the admin a real control.

## Become the admin

Actions possible now: go back to You, enter a LAN address, make an owner key, ask the box (disabled until a key and a gateway exist), prove after a challenge (disabled until the card fields are filled), read an error, read that the biscuit stays closed until the claim succeeds.

Actions that should be possible: issue the biscuit. Deferred. The page already says the grant stays closed until the claim succeeds, and issuance is the next slice. Do not add a grant control.

Design and feel: this is the strongest form in the window. Steps, disabled reasons, and the waiting last step are clear. No required change.

## Router

Actions possible now: read the selected group name, read a stub sentence, go back to Home. The section bar marks Home.

Actions that should be possible: a full router dashboard. Deferred.

Design and feel: the title is `Router` plus the raw id, including a 64-character hex, even when the group already knows the router's name. Required: title the page with that name when the id matches an assigned or heard router, and leave the id only when it does not.

## Sol tasks

Required gaps only. Each task names the files Sol may edit.

- [x] Home scan. Outcome: Home can run the existing discovery scan. Done when `admin/src/routes/+page.svelte` has a Scan button in the lead row that calls `discovery.scan()`, is disabled while `discovery.loading`, and reads Scanning while a scan is in progress. No other file.
- [x] Backhaul empty name. Outcome: an empty network-name field explains itself. Done when `admin/src/lib/network-name/NetworkNameBar.svelte` shows, only while the trimmed name is empty and the bar is not applying and has no report, the sentence "No network name is loaded on this computer. Type the client SSID to apply it. This is not the guild name." Apply stays disabled for an empty name. No other file.
- [x] Nearby via and state. Outcome: Via is one name plus a count, and State stays readable. Done when `formatVia` in `admin/src/lib/discovery/resources.ts` collapses duplicate vias to `Name` or `Name +N`, a unit test of that function fails if duplicates are not collapsed, the Nearby via cell uses it and keeps the full list in `title`, and State is `white-space: nowrap` with the table allowed to scroll sideways inside `.main`. Files: `resources.ts`, a new `resources.test.ts` beside it, `admin/src/routes/nearby/+page.svelte`.
- [x] Guild identity line. Outcome: the actor id does not break mid-token. Done when `admin/src/lib/guild/mock/IdentityCard.svelte` keeps `.addr` on one line (`white-space: nowrap`) and the card scrolls horizontally (`overflow-x: auto`). No other file.
- [x] You card. Outcome: You matches the shell's cards. Done when `admin/src/routes/you/+page.svelte` shows the actor id and the role in the selected group inside one surface card, the id does not wrap, and Become the admin is a control that links to `/you/become`. No other file.
- [x] Section focus. Outcome: keyboard users can see the section bar focus. Done when `admin/src/lib/shell/SectionBar.svelte` gives links and the collapse button a `:focus-visible` outline using `var(--focus)`. Hover and the current-section mark stay. No other file.
- [x] Router title. Outcome: a known router is titled by name. Done when `routerTitle` in `admin/src/lib/home/groups.ts` returns the assigned name, else the heard name, else the raw id, case-insensitive on the id, with assertions in `groups.test.ts`, and `admin/src/routes/router/[id]/+page.svelte` uses that string as the `SectionNote` title. The group name stays in the body. Back still goes to `/`.
