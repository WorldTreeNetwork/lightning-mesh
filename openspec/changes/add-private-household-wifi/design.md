# One SSID, three classes

Decision 2026-10-22, confirmed by Duke: priority only, no hard cap,
one client SSID. The guest-SSID contract reviewed through r7 is
superseded. Those reviews stay in `reviews/` as history. They are
not authority for this contract.

## Classes

The plan puts every client flow in one class.

- **Published service.** A box on a node's LAN publishes a `.mesh`
  name. Every client on the SSID may open that name. The plan does
  not hand the client a route to every address in `10.42.0.0/16` or
  `10.254.0.0/16`. The management plane stays closed: overlay
  `10.254.0.0/16`, WAN SSH, and the recovery address.
- **Member.** The client holds a verified IdentiKey membership in a
  guild or group. When the link is busy, member flows are scheduled
  ahead of unidentified flows. There is no rate cap. Association to
  the SSID is not membership. A directory label is not the mark.
  Issuing the credential is `mjolnir-mesh-ai0.1`. Until that verifier
  exists, the plan states the rule and does not invent a mark.
- **Unidentified.** Everyone else on the same SSID. Internet and
  published names at best effort. Creating an identity is not
  required.

Priority is a schedule order, not a reservation. On one radio a busy
unidentified client still uses airtime. A floor under member
throughput would be a rate cap, and this change refuses that cap.

## What a plan is

An uncommitted plan records the house intention and leaves the live
SSID unchanged (`radio: unchanged` until `commit: true`). The builder
is a pure function. It returns a type `OpenWrtAdapter` cannot submit.
It does not render the scheduler. A later apply slice does that when
live probes exist.

The one SSID stays the deployed open SSID until a separate
owner-committed plan encrypts it. This change does not write a
passphrase and does not add a `wifi-iface`.

## Not owed

- A second SSID, a guest CIDR, roam exclusion of a guest class, or a
  fleet INPUT reject keyed on one.
- Cake versus WMM. The plan names the class. The scheduler
  implementation is later.
- Guild admin SSH. A member grant is a biscuit the destination
  verifies. It is not a line in `authorized_keys`.
- Encrypting the SSID, live radio writes, CAPPORT, and paid transit.
