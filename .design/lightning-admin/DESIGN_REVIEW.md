# Design Review: Lightning Admin home

Reviewed against: DESIGN_BRIEF.md
Philosophy: Functional admin console, Ubiquiti Network
Date: 2026-11-03

Walked the running debug window (Plain, Light, about 1720×2030) with the Tauri MCP bridge. Group was Strawberry Creek, then Admin, then Strawberry Creek again. Did not press Apply or New owner key.

## Screenshots Captured

| Screenshot | What it shows |
| --- | --- |
| `screenshots/review-home.png` | Home, `0/1`, Nothing heard |
| `screenshots/review-home-expanded.png` | Status line expanded, still nothing heard |
| `screenshots/review-switcher.png` | Group menu: Strawberry Creek `0/1`, Admin `0/–` |
| `screenshots/review-backhaul.png` | Backhaul copy and Network name |
| `screenshots/review-guild.png` | Guild panel and Not heard roster |
| `screenshots/review-map.png` | Map stub |
| `screenshots/review-nearby.png` | Nearby while a scan is running |
| `screenshots/review-nearby-expanded.png` | Nearby after the scan, State column tight |
| `screenshots/review-services.png` | Services stub |
| `screenshots/review-you.png` | You, actor id, admin role, Become link |
| `screenshots/review-become.png` | Become the admin, inside the shell |
| `screenshots/review-collapsed.png` | Section bar collapsed to icons |
| `screenshots/review-appearance.png` | Appearance menu, Plain + Light |
| `screenshots/review-dark.png` | Same page in Plain Dark |
| `screenshots/review-router.png` | Router stub, Home marked, Back present |

This is a desktop window. The brief's narrow case is a tiled column, not a phone, so there are no 375px captures.

## Summary

The shell works and it looks like the console in the brief: header, icon bar, tight page, Plain warm field, steel selection. Guild, Nearby, Become, the group switcher, collapse, and Dark all behave. Home is honest about `0/1` and too empty once you open it. Nearby is the page that feels unfinished in this frame.

## Must Fix

None. Nothing clicked failed, and no page left the shell.

## Should Fix

1. **Home says "Nothing heard" twice.** The status line already says it. Opening the line adds the same words again and no router buttons. See `screenshots/review-home-expanded.png`. _Fix: when the heard list is empty, leave the row closed or show one quiet line, not a second copy._
2. **Nearby clips the right edge.** After a real scan the table is dense and useful, then Via repeats "Gateway" and State is cut (`this comp`, `reach`). See `screenshots/review-nearby-expanded.png`. _Fix: let the page scroll sideways, and keep State on one readable line. Show one via, with a count when there are more._
3. **The actor id breaks mid-token.** On Guild the id ends with `42` alone on the next line. See `screenshots/review-guild.png`. _Fix: keep the id in the mono address style and let it wrap on a character boundary, or give that card the width it needs._
4. **Network name looks unset.** Backhaul's field is empty, so the page does not show the name that is on the air. Apply stays disabled until someone types. See `screenshots/review-backhaul.png`. _Fix: fill the field from the last known client name, or say that this computer has not read one yet._
5. **Section focus is only a hover.** Current section and hover are clear. There is no visible focus ring on the bar, so a keyboard move does not show where you are. _Fix: add `:focus-visible` using the same accent soft as the current section, plus an outline._

## Could Improve

1. **You is thinner than the other pages.** Role and the Become link work, and they read as a note under a heading. Become itself is the strongest form in the window. See `screenshots/review-you.png` and `screenshots/review-become.png`. _Suggestion: set the id and the role in the same card language Guild already uses._
2. **Collapsed icons have no name until you learn them.** The bar collapses cleanly and You stays marked on Become. See `screenshots/review-collapsed.png`. _Suggestion: a title on each icon while collapsed._
3. **Map and Services are one sentence in a large field.** That matches the stub allowance. They will feel blank until the pages exist. Home has the same quiet when nothing was heard.

## What Works Well

- The bar is how you move. Home, Backhaul, Guild, Map, Nearby, Services, and You each take the page, mark themselves, and keep the header.
- The header names the house. Switching to Admin changed the count to `0/–`. Switching back restored Strawberry Creek `0/1`.
- Guild shows membership, the workplace, and `ap3000-outdoor-b` as Not heard. That matches the home count.
- Nearby is a real scan: resources, interface counts, names, addresses, and a scan that finishes.
- Become stays under You, has Back, and leaves Ask the box disabled until the card fields are filled.
- Collapse persists, and expanding it returns the labels.
- Plain Dark is the same layout with the warm dark field and a light accent button. Appearance stayed Plain + Light after the check.
- The router stub marks Home and offers Back to Home.
