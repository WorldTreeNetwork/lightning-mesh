# Information Architecture: Household network management

> Proposed extension of the [brief](DESIGN_BRIEF.md), 2026-09-14.
> Visible authenticated admin entry is confirmed and supersedes the older
> secret-knock entry. Other unconfirmed proposals remain under review.

## Site map

```text
Public front desk /
  Connect / local services / community identity (existing)
  Manage network /admin (confirmed visible, authenticated entry)
  Set up a node /setup (only offers eligible physical pairing)

Authenticated house /admin/houses/:houseId
  Overview /overview
  Nodes /nodes
    Node /nodes/:nodeId
      Connections /connections
      Radios & ports /hardware
      Placement /placement
      History /history
  Connections /connections
    Internet /internet
      Add /new
      Connection /:connectionId
    Node connections /node-links
    Wi-Fi networks /wifi
      Profile /:profileId
  People & access /access
    Person /people/:principalId
    Invitations /invitations
    Recovery /recovery
  Activity /activity
    Change /changes/:transactionId
  Settings /settings (utility)
    Policy, updates, diagnostics, export, advanced configuration
```

Desktop uses the same logical routes and task components with platform adapters.
Existing address discovery becomes the no-house entry and an Advanced diagnostics
view. Keep existing public identity/name flows separate from admin authentication.

## Navigation model

Five primary items: Overview, Nodes, Connections, People & access, Activity.
House switcher and account/signing credential are in the header. Settings, help,
and sign out are utility actions. Node detail uses secondary tabs. Common tasks
are Overview → task → result; advanced configuration is allowed deeper nesting.

Phone uses Overview / Nodes / More, with full-page tasks. Desktop has a side rail
and optional list/detail panes. Deep links preserve selected house/node and task.
The header names the entry node and data age when relevant, without forcing the
user to reason about proxy addresses.

## Content hierarchy

### Overview — primary admin destination

1. Actual outcome: internet status, local network status, nodes reachable; last checked.
2. One highest-priority action with a plain explanation of its effect.
3. Rooms/nodes needing attention, with client versus node-link diagnosis.
4. Current internet path and node topology; expandable technical evidence.
5. Recent changes, local services link, and secondary setup opportunities.

### Nodes

1. Human label / room, claim state, reachability, clients, connection type.
2. Attention or dependency: “Hall depends on this node” / “Ethernet disconnected.”
3. Radio allocation: 2.4 GHz purpose, 5 GHz purpose, port purpose.
4. Identify node, placement test, connections, and history.
5. Stable node ID, addresses, channel/width, driver, raw counters in details.

### Connections

1. Internet: active path, verified status, other permitted connections.
2. Node connections: wired/wireless links and their bottlenecks / fallback policy.
3. Wi-Fi networks: name, audience, security, bands and affected nodes.
4. Advanced: addressing, routing, VLANs, DNS, shaping, regulatory settings.

### People & access

1. Your role and scope; owners / recovery readiness.
2. Granted access, pending invites, expired grants, pending revocations.
3. Invite / modify / revoke according to actor permissions.
4. Recovery, credential rotation, transfer; separate from community PeoplePanel.

### Activity / change detail

1. Human outcome and affected nodes, including unknown/partial results.
2. Intended change versus observed result; actor and revision.
3. Recovery action or safe retry; reconnect instructions.
4. Per-node receipt and redacted diagnostics.

## CTA rules

Only the highest relevant action is primary. Other tasks remain reachable through
navigation. Disabled actions explain the missing permission or capability.

| State / audience | Message | Primary CTA | Secondary / result |
|---|---|---|---|
| No node found, installer | No Lightning nodes found on this connection | Find a node | Cable/button guidance; desktop interface picker |
| Unclaimed node, installer | This node is ready to set up | Set up this node | Verify device label and physical proof |
| Owned by someone else | This node belongs to another network | Ask the owner for access | No “claim anyway” shortcut |
| Local works, internet absent, admin | Local network works. Internet is unavailable. | Connect internet | Continue with local network |
| Intentional offline, admin | Internet is turned off for this house | Enable internet | Local services remain prominent |
| Upstream configured but failing | Wi-Fi connected; internet check failed | Check connection | Show association/address/DNS/probe evidence |
| Captive upstream | This internet connection requires sign-in | Open upstream sign-in | Explain session expiry; never proxy arbitrary credentials |
| Wrong upstream secret | Could not authenticate to the selected Wi-Fi | Update password | Choose another connection |
| Internet absent, resident | House internet is unavailable; local services still work | Open local services | View status / designated support info |
| Stable wired link newly found | Ethernet works; review how to use the freed radio | Review Wi-Fi improvement | Show fallback and dependent-node impact |
| Room complaint, admin | This room needs a connection test | Test this room | Placement guidance; no unsupported heatmap |
| Active change | Applying to Hall; other nodes are waiting | View change | No duplicate Apply; cancellation only where safe |
| Browser connection lost | Waiting to reconnect; outcome not yet known | Reconnect | Local recovery steps and transaction ID |
| Rollback verified | Previous settings restored | Review failure | Edit and try again |
| Partial result | 2 nodes updated, 1 restored, 1 unknown | Review affected nodes | Retry only after reconciliation |
| Healthy network | Internet and local network working | No urgent CTA | Add node / Invite person as ordinary actions |
| Recovery not verified | Prepare recovery for the owner's credentials | Set up recovery | One owner is supported; second owner not required |

## User flows

### 1. Purchaser: wired internet and a kit

1. Connect a labeled WAN port to the existing router. Power the first node.
2. Open local setup or discover it in Lightning Admin. No internet required for UI.
3. Prove possession using the device label plus physical setup window. If already
   owned, show access/transfer guidance rather than overwriting authority.
4. Establish house and first owner through the trusted signer. Browser without
   supported custody gets an explicit installed-signer path, not a fake login.
5. Show “Internet detected” only after a client-path test; otherwise diagnose the
   configured WAN. Do not require re-entering already detected network facts.
6. Choose resident Wi-Fi settings and optional guests; preview the change before
   switching away from the bootstrap connection. Provide new connection instructions.
7. Other kit nodes join only via unique kit enrollment; individual additions use
   the Add node flow. Show waiting / claimed / connected individually.
8. Name rooms and run an optional placement walk. Prompt to add recovery access.
9. Overview shows verified outcomes, remaining coverage gaps, and current path.

### 2. Existing Wi-Fi is the only internet source

1. Overview → Connect internet → Use existing Wi-Fi.
2. Discover eligible receivers. Show each node's scan freshness, signal, band,
   downstream path, and client-band cost; explain ineligible nodes.
3. Select network (or enter hidden SSID), security type, and secret. Password
   entry lives in a trusted surface and is redacted everywhere else.
4. Choose suggested receiver or compare candidates. “Find the best node” obtains
   permission for specified candidate tests and temporary disruption; does not
   copy the password to the whole fleet automatically.
5. Preview: “Office receives internet on 5 GHz; Ethernet serves the other nodes;
   Office provides 2.4 GHz Wi-Fi.” If no wired path exists, explain the alternative
   mesh-band allocation and any loss of client service before proceeding.
6. Test association → address → DNS → internet → downstream client path.
   Wrong password retains the non-secret draft; captive portal becomes “Sign-in
   required”; unsupported enterprise/security mode gives a concrete explanation.
7. Apply with persisted rollback. Return to Overview with tested status or the
   restored settings and failure reason. Never equate DHCP with internet success.

### 3. Add a node / add Ethernet

1. Add node → identify by label/button and verify possession → grant membership.
2. Label location; inherit house policy; show actual band availability.
3. If a cable is detected, identify whether it is upstream, trusted node link, or
   client attachment. Ask for intended role if evidence is ambiguous.
4. Validate the wired node path and loop policy. If a radio could be released,
   check downstream wireless dependants and fallback requirements first.
5. Review Wi-Fi improvement → compare capacity and fallback → Apply.
6. Cable loss later produces a precise degraded state; use only a previously
   qualified fallback. If isolated, instruct how to reconnect physically.

### 4. Diagnose slow video calls / a dead room

1. Overview → affected room/node → Test this room.
2. Identify associated AP, client band and link; separately measure node path and
   upstream. Offer bounded tests with expected duration and traffic cost.
3. Present evidence: “Room Wi-Fi weak,” “node link congested,” “upstream slow,” or
   “not enough information.” Never infer wall material from a single RSSI reading.
4. Recommend the relevant action: move node / add Ethernet / choose receiver /
   apply fairness policy / contact upstream operator. Compare before and after.
5. Record measurements and timestamps; preserve an accessible text report.

### 5. Resident / guest / local developer

The expanded [welcome flow](../../openspec/changes/add-household-network-management/identity-and-welcome.md)
defines the portal invitation, regular-browser handoff, consent before publication,
QR/manual discovery, dismissal, and offline states. Invite all visitors; do not
require identity to browse the hub. Member is the permission role; resident is
the person's relationship to the house.

1. Join allowed Wi-Fi. Internet access follows network policy, independent of
   creating an IdentiKey or publishing a name.
2. Public front desk shows internet/local status and allowed services. Existing
   “Just the internet, please” remains coherent when internet is available.
3. A developer may create an identity and publish their service under its own
   authorization. This does not grant router control or automatically expose it
   to guests/the internet.
4. Guest service exceptions are named and scoped. If internet is down, point to
   available local services rather than repeatedly offering an impossible login.

### 6. Delegate, revoke, and recover

1. The single initial owner opens People & access → Invite person → Admin, Member,
   or Guest → exact permissions, node/service scope, expiry. Role labels are
   presets; show the effective grant and whether further delegation is allowed.
2. Recipient accepts using their credential. Show issued / accepted / enforced
   states; an invitation string is not a permanent admin password.
3. Revoke shows where enforcement is confirmed and where a disconnected node
   remains pending. Removing a resident also explains shared Wi-Fi password
   rotation where relevant.
4. Lost admin laptop: owner or explicitly established recovery authorizes replacement,
   revokes lost credentials, and verifies fresh access.
5. Transfer: replacement accepts and proves access before the final old owner is
   removed. Factory reset/release is an explicit separate destructive procedure.

### 7. Fleet change while connected through an affected node

1. Draft change shows target set, changed bands/ports, dependants, and version.
2. Review impact includes entry-node interruption and a concrete reconnect path.
3. Prepare targets; if required nodes are missing, stop before disruptive changes.
4. Apply ordered plan; show target receipts. Browser disconnect does not cancel
   node-local recovery or reset deadlines.
5. Reconnect from another node/device and open the same transaction. Show committed,
   restored, partial, or unknown. “Undo” creates a new revision-aware plan; never
   restore an old snapshot over newer changes without checking conflicts.

## Naming conventions

| Concept | UI label | Notes |
|---|---|---|
| Administrative trust domain | House / house network | Independent of network name and guild |
| Physical managed router | Node | Human room label first, stable ID in details |
| External connection | Internet connection | Wi-Fi, Ethernet, or future cellular |
| Connections between nodes | Node connections | Technical detail may say backhaul |
| Client-facing SSID | Wi-Fi network / Network name | Never implies identity or admin rights |
| Person with authority | Owner / Network admin | Role is distinct from key representation |
| Candidate performance | Suggested / Tested | Evidence and age required |
| Save configuration | Review changes → Apply changes | Draft edits do not mutate hardware |
| Failed apply recovery | Previous settings restored | Only when verified |

## Component reuse map

| Shared unit | Surfaces | Platform differences |
|---|---|---|
| House shell, task forms, node cards | Web and desktop | Layout density only |
| Plans, capability validation, receipts | All configuration workflows | Transport adapter only; target validation authoritative |
| Discovery | Setup and Nodes | Desktop local scans; web entry-node directory / manual discovery |
| Trusted approval prompt | Pairing and privileged mutations | Installed signer initially; standalone browser path gated |
| Status and topology | Overview, node detail, public subset | Public data filtered; no private fleet diagnostics leakage |
| Existing People/Services components | Public front desk | Never silently upgraded into ownership controls |

## Content growth and URL strategy

Design for a few to dozens of nodes; filter by room, state, connection, and role.
People/invitations and activity get pagination and explicit retention controls.
House switching keeps permissions and drafts separate. Public topology need not
expose private room names, device associations, or admin audit logs.

Use stable house/node IDs in routes, never IP addresses. Query parameters carry
non-sensitive filters only. Credentials, invitation secrets, and owner seeds
never appear in URLs or analytics. If an invitation is exchanged out of band,
redeem it through an explicit protected input. “History” deep links identify a
transaction; receipts remain authorization-checked on every target.
