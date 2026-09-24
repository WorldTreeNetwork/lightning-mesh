---
id: house.first-hour
title: First hour on the mesh
description: Confirm you are on the house mesh, open hello.mesh, and decide what to do next.
path: house
order: 2
audience: [person, operator, agent]
status: built
time: 10 min
requires: [house.unbox]
next_step: house.how-it-works
verified_against: 2f6dc6b (2026-09-24)
---

# First hour on the mesh

Once the client Wi-Fi has an address, the mesh is already a network. The
front desk is `http://hello.mesh`. Identity is optional.

> **Status: built.**

## Step 1: Confirm the address

**Do:** look at the Wi-Fi details (phone: tap the network name; Mac:
`ipconfig getifaddr en0`; Linux: `ip -4 addr`).

**Expect:** something like `10.42.61.23`. The router you landed on is
`10.42.61.1` — same first three numbers, last number `.1`.

**If not:** you are on a different network. Forget it and rejoin the name
from [Unbox](01-unbox.md).

## Step 2: Open the front desk

**Do:** in Safari, Chrome, or Firefox, type exactly:

```
http://hello.mesh
```

Include `http://`. There is no public `https` for this name yet.

**Expect:** a page that says you are connected to a named router, with
People, **Apps**, Services, and Routers. The page refreshes itself.

**If not:**
- The browser searched — type the full `http://hello.mesh`.
- "Server not found" — turn off Private DNS / secure DNS. See
  [Troubleshooting](../person/troubleshooting.md).
- Still nothing — open `http://10.42.x.1` using the `.1` from step 1.
  Come back to `http://hello.mesh` before creating an identity; keys made
  at the numeric address are stored separately.
- No **Apps** section — older hello on this box. Services still work.

## Step 3: Optional identity

**Do:** if you want a name other people on this mesh can see, tap
**Set up your identity** and follow
[Create your identity](../person/03-identity.md). Save the 24-word
phrase.

**Expect:** you appear under People. Apps can later ask you to sign in.
Stamp location (under Routers) also needs this identity.

**If not:** skip it. Joining the Wi-Fi did not create an account. You can
use internet (when a router has an uplink) and open published services
without an identity.

An IdentiKey is **not** permission to administer routers. House
administration is SSH or Lightning Admin, covered in
[Administer the house](04-administer.md).

## What "working" looks like

| You try | What should happen |
|---|---|
| `http://hello.mesh` | Front desk loads |
| A website on the public internet | Loads if **any** mesh router has a working WAN; otherwise only local mesh works |
| Apps / Services | Empty until someone publishes. That is normal on a new house |
| A second pre-flashed box, powered nearby, same client SSID | Radios are up; they are **not** one mesh until you exchange peer ids — [Add another router](06-add-router.md) |

Internet sharing is automatic (`gateway=auto`). If this box is on a
metered or captive network, do not share: see
[Administer the house](04-administer.md).

Next: [How the house mesh works](03-how-it-works.md).
