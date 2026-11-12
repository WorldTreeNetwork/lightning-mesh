# Build Tasks: Lightning Admin home

Generated from: .design/lightning-admin/DESIGN_BRIEF.md
Date: 2026-11-03

Visual direction for the first slice: a Ubiquiti-style operator console. Full-width header, left section bar under it, tight padding. Plain tokens from `admin/src/app.css`. Related items use `--space-1` and `--space-2`. A larger gap only between the header, the bar, and the page.

## Foundation

- [x] **App shell and section bar**: Build the frame every page sits in: a header with the group name and the Appearance menu, and a left bar under the header with Home, Group, Map, Nearby, Services, and You. Each row has a Lucide icon and a label. The current section is marked. Collapsing leaves the icons, keeps the mark, and persists. Stub routes render a title and, where they are not sections, a back control. Done when you can open each section from any other and the bar stays put. _New: shell, section bar. Reuses: Appearance menu, Plain tokens. Adds Lucide as part of this slice._

## Core UI

- [x] **Home**: Inside the shell, Home shows the `3/5` status line and expands it into a sideways row of heard-router buttons, icon plus router name. The header switcher lists saved groups with their own `active/total` and choosing one replaces the count and the buttons. Done when a scan with routers is readable at a glance and a button opens `/router/[id]`. _New: status line, router button, group switcher. Depends on: app shell._
- [x] **Nearby**: Move the current discovery table, filters, and address expand onto `/nearby` inside the shell. Home no longer shows that table. Done when Nearby still scans, filters, and expands an address. _Modifies: discovery page. Reuses: discovery table. Depends on: app shell._
- [x] **Group**: Build `/group` with separate Group name and Wi-Fi name fields, a local-network settings block, and the roster. Heard rows look present. Missed rows say "Not heard" and stay quiet. Applying the Wi-Fi name uses the current network-name apply and does not change the group name. _New: group page. Reuses: network-name store, role chip. Depends on: app shell._
- [x] **You and Become the admin**: Build `/you` with the actor and their role, and move the current claim flow to `/you/become` with a back control to You. `/become` lands on `/you/become`. When this person is not an admin, Home's status line offers You. _Modifies: become page. New: You page. Depends on: app shell, home._
- [x] **Map, Services, and Router stubs**: Fill `/map`, `/services`, and `/router/[id]` with the selected group's name where it applies, a short statement of what the page will become, and a working back control on the router page. The section bar remains the way into Map and Services. _New pages. Depends on: app shell, home._

## Interactions & States

- [x] **Empty, busy, and quiet states**: Home with nothing heard, a group with a placeholder count, a scan in progress, a Wi-Fi apply in progress, an apply error, and a roster of only "Not heard" rows. Each state is text plus the same icons, in the same tight layout. _Modifies: home, group switcher, group page._
- [x] **Narrow column**: Around 847px wide, the section bar starts collapsed unless they have expanded it. The router row scrolls sideways instead of wrapping. Header and bar stay fixed while the page scrolls. _Modifies: app shell, home._

## Responsive & Polish

- [x] **Accessibility pass**: Focus rings on sections, the collapse control, router buttons, switcher rows, and back. Collapsed icons keep their accessible names. The count is text. "Routers heard" names the expanded row. Group name and Wi-Fi name are separate labels. Quiet rows stay in the tree. Reduced motion makes the collapse and the expand instant. _Modifies: shell, home, group, you._

## Review

- [x] **Design review**: Run /design-review against the brief. Capture the shell, Home expanded, the collapsed bar, Group, and Nearby with /admin-screenshot.
