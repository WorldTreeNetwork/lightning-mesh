# Household configuration model and hardware inputs

> Proposed design, 2026-09-14; `add-household-network-management` is PENDING.
> This is not a live inventory or a claim that these modes are implemented.

## 1. Current evidence

| Evidence | What it establishes | What it does not establish |
|---|---|---|
| [Architecture](../../../ARCHITECTURE.md) | Symmetric nodes, per-node routed client /24, local management, canonical apply | Automatic port or radio reassignment |
| [Wireless setup](../../../deploy/openwrt/setup-wireless.sh) | 5 GHz mesh / 2.4 GHz AP default; inverse selectable | Wi-Fi upstream mode or wired dual-band AP profile |
| Same script, concurrent-AP warning | Mesh+AP on one mt76 radio broke joins in field tests; disabled by default | Universal impossibility on every driver / firmware |
| [Apply](../../../deploy/openwrt/files/usr/sbin/mjolnir-apply) | Snapshot and local baseline health gate | Durable fleet commit-confirm, client internet verification, or recovery after every crash |
| [Settings spec](../../specs/mjolnir-settings/spec.md) | AP identity/security settings in mjolnir UCI | Full policy schema proposed below |
| [Fleet snapshot](../../../deploy/openwrt/fleet.yml) | Historical models and observations dated 2026-08-31 | Current cabling, reachability, or optimal uplink node |
| [Gateway design](../../../docs/network-coordination/gateway-liveness.md), [egress code](../../../crates/mjolnir-mesh/src/crdt/egress.rs) | Egress detection, fail-open probe hysteresis, live-gateway observability; Babel owns routes | Strict fresh internet proof for setup; seamless NAT-session failover |
| [Browser key code](../../../hello-mesh-web/src/lib/identity/keys.ts) | HTTP identity uses extractable JS keys, explicitly soft custody | Safe storage of fleet owner credentials |
| [Desktop](../../../admin/README.md) | Discovery and SSID apply through SSH tooling | Consumer ownership or a browser admin API |

No secrets were read or live router changes made. Historical “current” wording
in older design notes should be checked against code and living specs.

## 2. Topology options

Each node has a resource allocation, not a single permanent “router mode.” The
following are proposed supported profiles subject to port/driver qualification.

| Installation | Internet ingress | Node connections | Client radios | Main consequence |
|---|---|---|---|---|
| Existing router + wired house | Ethernet WAN on one node | Dedicated Ethernet backhaul | 2.4 + 5 GHz on wired nodes | Preferred capacity; needs safe port separation |
| Existing Wi-Fi + wired house | 5 GHz station on best placed node | Ethernet to switch and peers | Ingress: 2.4 GHz; other wired nodes: both | One Wi-Fi hop limits internet; local wired traffic avoids it |
| Ethernet gateway + wireless satellites | Ethernet WAN | 5 GHz mesh | 2.4 GHz on mesh nodes | Matches current radio split; satellites consume shared airtime |
| Wi-Fi ingress + fully wireless distribution | 5 GHz station, 2.4 GHz mesh on ingress | Compatible 2.4 GHz mesh across island | Ingress: none; satellites: 5 GHz | No 2.4 GHz client service on that island; throughput bottleneck |
| Inverse fully wireless profile | 2.4 GHz station, 5 GHz mesh on ingress | 5 GHz mesh | Ingress: none; satellites: 2.4 GHz | Faster inter-node links possible, slower shared upstream |
| Partial Ethernet | Ethernet or Wi-Fi ingress | Wired core + wireless branches | Wired-only nodes: both; branch anchors: spare band | Keep mesh on every anchor needed by a wireless branch |
| Dedicated upstream receiver | Wi-Fi station on a separate box | Receiver Ethernet → gateway WAN, then mesh/wires | Main fleet keeps chosen profile | Useful staging topology; may add another NAT boundary |
| No external connection | None | Wired and/or mesh | According to available radios | Local apps/names continue; intentional offline state |
| Multiple internet connections | Ethernet, Wi-Fi, later modem | Any qualified topology | According to allocation | Priority/failover first; bonding is a separate feature |

For the Victorian, start planning with a wired core, strategically placed APs,
and the best measured upstream receiver. The AP3000's outdoor location or antenna
placement may help, but model name alone does not establish that it wins.

Illustrative proposed wiring; physical port labels must be verified per model:

```text
Existing Wi-Fi
    |
    | 5 GHz station
    v
Upstream receiver / mesh gateway
    |
    | dedicated node-connection Ethernet
    v
House switch ---- Hall node ---- client devices
    |             2.4 + 5 GHz
    +----------- Office node --- workstation / local server (client port)
    |             2.4 + 5 GHz
    +----------- Stairwell node ~~~ wireless satellite
                  5 GHz mesh         5 GHz mesh
                  2.4 GHz clients    2.4 GHz clients
```

“All Ethernet” can mean upstream LAN, node backhaul, or client LAN. Those are
different trust/routing domains. A single unmanaged switch can carry a dedicated
node-backhaul segment, but plugging resident clients into it must not bypass
client policy. Mixed upstream/backhaul/client use requires verified VLAN support
or physically separate segments. Never join all `br-lan` interfaces together;
that contradicts the current per-node L3 architecture and risks DHCP conflicts.

## 3. Configuration objects and scope

| Object | Scope / fields | Default / admin override |
|---|---|---|
| House | Stable ID, display name, trust epoch, policy version | Distinct from SSID and community/guild identity |
| Membership | Node ID, house grant, model/capability evidence | Physical claiming; nearby discovery is not adoption |
| InternetConnection | Kind, node/eligible-node set, credential reference, priority, metered policy, health | Wired DHCP first; Wi-Fi requires credentials and permission |
| NodeConnection | Peer/segment, Ethernet or mesh, band/channel, measured state | Prefer validated wire; retain required relay links |
| RadioAssignment | Physical radio ID, supported roles/combinations, active role, band/channel/width/power | Capability-gated; unsupported sharing unavailable |
| PortAssignment | Physical label, WAN / node link / client / disabled / advanced trunk | Per-model defaults; ambiguous ports require confirmation |
| WiFiProfile | SSID, resident/guest/device purpose, security reference, bands, schedule | Confirmed target: private residents; isolated guests optional |
| AccessPolicy | Client isolation, cross-node policy, local service exceptions, internet permission | Enforced on all paths, not just within one AP |
| OptimizationPolicy | Balanced / Coverage / Local performance / Custom; min service bands, fallback reserve | Balanced and stability-first; explain custom constraints |
| Authorization | Principal, role, node/house/service scope, credential bindings, expiry, version | Least scope needed; owner separate from network admin |
| ChangePlan | ID, actor, target revisions, prerequisites, impact, deadlines, stages | Same object across web/desktop; no silent last-writer-wins |
| Observation | Source node, time/age, test type, result, confidence | Never treat unknown/stale as healthy |

Settings show **House default → Node override → Actual state**, with a “Use
house default” action. A pinned role states what automatic optimization can no
longer do. Radio and port configuration expands the canonical mjolnir store;
secrets remain referenced, never returned in normal config responses.

Later expert options: static WAN, PPPoE, WAN VLAN, routed upstream with static
return routes, IPv6 PD/prefix changes, DNS overrides, maintenance schedules,
approved guest service exceptions, SQM/fairness, VPN egress, remote access,
per-service exposure, export/import and diagnostic bundles. Each needs its own
support declaration; Linux's capability is not proof the product supports it.

## 4. Automation contract

### Safe candidate selection

Discovery first identifies physical radios, valid interface combinations,
channel constraints, ports, link speeds, trusted peers, and configured policies.
Candidate nodes must have an upstream-capable radio and a surviving path to the
house. Exclude a candidate that would isolate dependants or violate required
client bands before comparing throughput.

Rank eligible candidates using observed end-to-end goodput, loss/latency under
load, upstream reliability, downstream bottleneck, and coverage cost. Signal
strength alone is insufficient. Show “suggested from scan” before a connection
test; show “tested” only after a dated test. Scans that interrupt an active radio
need an impact notice and should run sequentially. Enter credentials only after
candidate discovery; authorize exactly which candidates may test them.

### Automatic versus proposed changes

- Automatic: use an already-authorized healthy gateway; prefer a validated wired
  path; recover an existing link; retain local service through an internet outage.
- Propose first: remove a client band, remove a relay, change a mesh island's
  channel, move upstream credentials to a new node, alter trust/segmentation,
  incur metered usage, or scan disruptively without prior policy authorization.
- Never: claim an unknown node, join arbitrary open upstream Wi-Fi, infer internet
  from DHCP alone, or bridge trust domains based solely on cable carrier.

Hysteresis prevents flapping. Initial tuning hypothesis: require 60 seconds of
stable wire before a radio-reallocation suggestion and 10 minutes between
performance-driven reallocation suggestions. Recovery of an authorized failed
path need not wait for that cooldown. These are pilot constants to measure,
not existing behavior or guaranteed failover times.

### Cable loss and radio fallback

Releasing a mesh radio for clients removes immediate wireless fallback. Offer:

- **More client capacity:** use both client bands; a failed cable may isolate
  this node until a coordinated fallback profile or physical repair is possible.
- **Reserve wireless fallback:** keep the mesh radio allocated; preserve the
  spare-band client service, with its capacity tradeoff shown.

Do not promise instantaneous self-healing on dual-band hardware. A fallback
requiring other nodes to change channels or roles is a fleet plan with pre-staged
recovery, not a local toggle. Wired+wireless simultaneous paths need tested loop
prevention; the current 802.11s forwarding island makes naive bridging unsafe.

### Internet health

Model separately: associated / addressed / DNS working / internet verified /
upstream sign-in required / local only / unknown. Probe through the candidate
interface so another gateway cannot produce a false success. Verify from a
downstream client path as part of commissioning. IPv4 and IPv6 health are separate.
Preserve the existing runtime fail-open gateway policy until a scoped change
revises it; the stricter setup success indicator is additional evidence.

Failover can change the public address and break active calls/downloads or inbound
sessions. State this during multi-uplink setup. Load balancing does not combine
two connections into one faster flow; bonding and inbound reachability need
separate architecture and upstream cooperation.

## 5. Identity, ownership, and access

Confirmed 2026-09-14: one initial owner delegates to admins, members, and guests.
See [the identity and welcome refinement](identity-and-welcome.md) for the pulled
IdentiKey evidence, Biscuit application profile, FOKS-style keyspaces, and hub
invitation. This refines the generic grant model below; it does not add a second
token format. A second owner is optional, not a setup requirement.

Identity means a principal with credentials, not a display name, MAC address,
SSID password, or SSH key copied from the developer's machine. Keep four layers
distinct: network association; house membership; local-service permissions;
node administration. Reuse IdentiKey where appropriate without merging scopes.

| Role | Internet / services | Network settings | Invite admins / transfer / recovery |
|---|---|---|---|
| Guest | Guest internet; explicit service exceptions | No | No |
| Member | Member/resident policy; own service identities | No | No |
| Network admin | Resident permissions plus diagnostics | Assigned nodes/house; no owner grants | No owner transfer; delegation only if explicitly granted |
| Owner | Policy-controlled service access, not automatic access to residents' private data | All house network settings | Yes |
| Temporary technician | Explicit diagnostics/config scope, expiring | Only granted operations | No |

### Factory → claim → expand

Each shipped node generates a unique device key and has a unique onboarding
secret / verifiable identity tied to its label. No universal password, shared
factory owner, or developer SSH access. Claiming requires physical presence
(button window) plus the per-device proof, with rate limiting and replay
protection. Button timing alone must not let the fastest nearby attacker win.
Existing WPS WAN-SSH arming is an access window, not an ownership ceremony.

First claim creates a house authority and owner grant. A sealed kit may carry a
unique kit enrollment relationship so additional kit nodes adopt policy after
the kit is claimed. Never use a fleet-global factory secret. Separately bought
nodes require “Add node” proof of possession. Already-owned nodes cannot be
reclaimed by a stranger's discovery or ordinary button press.

### Control protocol and custody

Reuse `b6j.2`'s proposed typed, signed control request: target node/house, action,
canonical arguments hash, challenge, authority epoch, expiry, and idempotent
request ID. The target verifies authority and persistent replay state itself;
an entry node or proxy cannot expand permission. Return target-authenticated
receipts. Encrypt secrets to authorized target nodes; gossip carries signed
policy/grants and public status, never Wi-Fi passwords or private owner keys.

An owner list in a CRDT is a projection, not the root of authorization. Grant
and revoke events need authorized signatures, deterministic ordering, persistent
epochs, and an explicit conflict policy. Two concurrent ownership changes based
on the same revision must be rejected or reconciled through the defined authority
protocol. A partition cannot provide instantaneous fleet-wide revocation: show
pending nodes and use bounded-lived admin capabilities. Offline nodes accept no
fresh privileged mutation on stale/expired authority; reconnect syncs revocation
before new grants. Document the remaining bounded exposure rather than claim
global revocation is instantaneous.

Use installed signer / platform credential custody for owners in the first safe
implementation. A web control panel may request a typed approval from that signer;
the trusted approval UI independently displays the target and exact action.
Never load an owner seed into arbitrary `http://hello.mesh` JavaScript.

Browser-only admin is a release gate, not a solved detail. WebAuthn is scoped to a
relying-party identifier and secure context; changing arbitrary node origins is
not transparent credential portability. Choose and test a trusted HTTPS origin,
certificate bootstrap/renewal without current internet, and stable relying party
before promising phone-only owner setup. An installed app/signer is the explicit
initial fallback. See [W3C WebAuthn](https://www.w3.org/TR/webauthn-2/).

Align with the existing [secure-context proposal](../../../docs/network-coordination/secure-context-and-control-plane.md):
per-node key-qualified origins, no shared fleet TLS key, and installed-signer
portability rather than a shared browser keystore. Public certificates have
finite offline validity; bootstrap/renewal cannot promise indefinite offline
browser trust. HTTPS alone does not supply independent owner-signing custody.

### Lifecycle

- Invitations: single-use, expiring, scope-bound; accepting binds the recipient's
  credential. Human-readable name is editable independently of authority.
- New phone/laptop: existing owner authorizes a new credential; do not copy raw
  private keys through chat, clipboard instructions, or node-hosted pages.
- Lost credential: use verified offline recovery or an explicitly appointed recovery authority, revoke old
  credentials, rotate affected authority, and report pending enforcement.
- Last-owner removal: blocked until replacement/recovery is verified. Recovery
  material should be tested during setup without forcing cloud synchronization.
- Resale: explicit release erases house secrets, rotates operational identity as
  designed, revokes old membership, and returns to unclaimed state. Physical
  factory reset is a last-resort ownership reset, never recovery of old secrets.
- SSH: separately authorized, expiring diagnostic grants; remove developer keys
  in shipment image qualification. No undocumented support backdoor.
- Hardware compromise: software-only key storage cannot promise resistance to a
  physically determined attacker. Evaluate protected key storage separately.

## 6. Safe apply and shared control architecture

Share typed domain models, validation, change plans, receipts, permissions, and
UI components between Tauri and local web. Platform adapters handle discovery,
credential custody, and transport. The browser cannot directly use the desktop's
SSH or network-interface scan. Nodes remain symmetric; a temporary transaction
coordinator does not become permanent house ownership authority.

Plan state: Draft → Validated → Prepared → Applying → Verifying → Committed;
failure yields Reverting → Restored or Recovery required. Unknown outcome is
distinct from failure. Optimistic concurrency checks target config revisions;
double-click/retry reuses a request ID. Audit actor, targets, revisions, reason,
timestamps, and receipts without secrets or private browsing history.

Persist recovery and deadline state locally before disruptive work. Validate
package availability offline. Preserve a management path, then verify local
services, required peers, and intended client internet outcome. Existing
`mjolnir-apply` is the projector to extend, not a substitute for these new gates.

For fleet changes, compute the dependency graph. Do not reconfigure the sole
gateway/relay before its dependants have a viable path. Channel changes may need
coordinated activation because old and new mesh channels cannot talk. Require
all affected nodes prepared with timed rollback, or decline the plan and offer
a wired maintenance path. Report partial commit explicitly; do not claim atomic
fleet transactions across partitions. Retry must reconcile observed state.

## 7. Co-living and hacker-house option analysis

| Need | Product response | Boundary / validation |
|---|---|---|
| Calls during model downloads | Fairness / latency-under-load policy | Measure shaping CPU and actual uplink; radio headline rates are insufficient |
| NAS, GPU hosts, local inference | Prefer wired local paths; service discovery and scoped access | Do not route local transfers through the external Wi-Fi receiver unnecessarily |
| Guests during events | Expiring invitation or guest Wi-Fi; internet-only default | Enforce isolation across every node and IPv4/IPv6; captive portal is not a firewall |
| Printers, speakers, IoT | Optional Devices profile and selected discovery/service exceptions | Avoid broad multicast reflection or merging resident/guest networks |
| Residents moving out | Revoke member/service grants; rotate shared Wi-Fi secret if needed | Revoking identity alone does not revoke knowledge of a shared PSK |
| Development servers / demos | Explicit service publish and visibility | LAN presence is not authorization for internet exposure |
| Public webhook / remote SSH | Opt-in remote access proposal with authenticated endpoints | Upstream NAT/CGNAT may prevent inbound connections; no implicit port opening |
| Multiple houses | Separate trust domains, optional later federation | Nearby radio reachability never merges ownership |
| Internet outage | Show local apps available, upstream diagnosis | Keep local names and admin usable without cloud dependency |
| Cable not feasible | Placement test and mesh profile tradeoff | Do not infer wall loss from building age alone |
| Power interruption / update | Persistent config, recovery image, resumable status | Offline UI bundle and required packages must already be present |

## 8. Hardware selection inputs and qualification

These are requirements for a future search, not purchasing recommendations.

| Present limitation / uncertainty | Selection input | Acceptance evidence |
|---|---|---|
| Two radios cannot independently serve upstream + mesh + both client bands | At least the independently usable radio resources required by each target profile | Simultaneous role test; three radios still do not guarantee all four roles without sharing |
| Concurrent mesh+AP broke joins here | Driver/firmware combination qualified for needed roles | Sustained AP+mesh/STA test under load and channel changes, not merely advertised VIF support |
| Ingress may consume a scarce Ethernet port | Enough independently assignable ports, validated switch/VLAN layout | WAN/backhaul/client isolation test with physical labels; include one-port outdoor devices |
| Local AI/NAS traffic can exceed uplink needs | Evaluate multigig ports/switching and forwarding CPU | Bidirectional local throughput plus concurrent clients and shaping |
| Victorian walls / remote placement uncertain | Antenna placement options and practical powered cable routes | Room walk, loss/retry/latency/goodput tests at actual intended locations |
| Wired fallback conflicts with dual-band client service | Dedicated fallback radio or supported concurrent modes | Pull cable under load; record outage and which client bands disappear |
| Field installation / outdoor receiver | Appropriate power method, PoE compatibility, mounting and environmental rating | Verify exact SKU/revision, injector/switch requirements, sustained operation |
| No robust owner custody on current browser surface | Per-device bootstrap identity, physical control, protected storage option | Claim/race/reset/recovery tests; inspect firmware provisioning process |
| Flash/RAM and update safety unknown per revision | Headroom for daemon, UI, packages, logs, signed updates and recovery | Reboot/power-cut/update rollback, memory/storage stress with real workload |
| USB expansion might add a radio/modem | Stable Linux drivers, power budget, enclosure fit | Boot/hotplug/reconnect/load tests; no assumption a dongle is plug-and-play |

Linux supports multiple virtual interfaces only where the driver implements the
combination; qualifying the actual firmware remains essential. See
[Linux wireless VIF documentation](https://wireless.docs.kernel.org/en/latest/en/users/documentation/iw/vif.html).
OpenWrt's routed-client documentation describes routing/NAT as the general
station-uplink approach because ordinary client bridging is not generally
supported. Transparent/WDS mode remains an explicitly qualified advanced option.
See [OpenWrt routed client](https://openwrt.org/docs/guide-user/network/routedclient).
The search excerpt was available; direct page retrieval encountered a bot gate.

Bench comparisons should record SKU/revision, firmware/kernel/driver, regulatory
settings, radio roles, cable topology, placement, client type, and load. Measure
one-hop and multi-hop throughput, latency under load, retries, roaming and cable
failure. Repeat with simultaneous internet downloads and local server transfers.
Set numeric product thresholds after house size, concurrency, and upstream speed
are known. Do not turn link-rate estimates into a coverage or speed guarantee.

## 9. Recommended delivery boundaries

First validate one safe Wi-Fi receiver → Ethernet gateway path with the current
hardware and existing apply mechanism. Then deliver ownership and signed control,
shared setup/receipts, explicit wired/mixed profiles, and only then automatic
radio optimization. Qualification is necessary before automatic changes can
claim safety. Guest segmentation, full phone-browser custody, roaming claims,
and advanced multi-uplink policy each need their own acceptance evidence.
