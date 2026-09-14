---
id: person.troubleshooting
title: Troubleshooting
description: Symptoms new users hit on Lightning Mesh, their causes, and fixes.
path: person
order: 99
audience: [person, operator, agent]
status: built
time: reference
requires: []
next_step: null
verified_against: 503cd17 (2026-09-13)
---

# Troubleshooting

Find your symptom, then work through the fixes in order.

## hello.mesh or other `.mesh` names don't open

| Cause | Fix |
|---|---|
| Browser searched instead of opening | Type the whole address with `http://`, for example `http://hello.mesh` |
| Android **Private DNS** is on | Settings → Network → Private DNS → **Off** while on this network |
| Browser **secure DNS** (DNS over HTTPS) is on | Chrome: Settings → Privacy → Use secure DNS → off. Firefox usually turns it off by itself here |
| A VPN or iCloud Private Relay is on | Pause it for this network |
| Not actually on the mesh | Check your address starts with `10.42.` ([Connect](01-connect.md), step 3) |

Still stuck: open the router's numeric address, `http://10.42.x.1`.

## Connected, but nothing loads at all (the phone just spins)

You got an address, but the router at `10.42.x.1` doesn't answer.

| Cause | Fix |
|---|---|
| Your router has a known misconfiguration (a leftover second address on its LAN) | Tell the operator: "gateway blackhole on 10.42.x.1". Meanwhile, forget the network and rejoin, which may put you on a different router |
| The router is restarting after an update | Wait two minutes and rejoin |

## Mesh works, internet doesn't

| Cause | Fix |
|---|---|
| No router in the mesh has an uplink right now | Nothing to fix on your device. `.mesh` services still work |
| Your phone says "Connected" anyway | Expected after "Just the internet, please". The OS is told the network is fine |

## The welcome sheet never appeared

That's fine; it's optional. Possible reasons: Private DNS or VPN is on, you
already tapped "Just the internet, please" in the last 12 hours, or the
operator turned the sheet off. Go straight to `http://hello.mesh`.

## My identity disappeared

| Cause | Fix |
|---|---|
| You opened the numeric address instead of `http://hello.mesh` | Go back to `http://hello.mesh`. Identities are kept per address |
| Private browsing, or site data was cleared | [Restore from your recovery phrase](06-leave.md#step-2-restore-somewhere-else) |
| Different browser or device | Same: restore from the phrase |

No recovery phrase saved? That identity is gone. [Create a new one](03-identity.md).

## "Still introducing you — retrying"

The router is busy or briefly unreachable. Keep the page open; it retries.
If it never finishes, rejoin the Wi-Fi and tap **Join** again.

## People list shows odd "last seen" times

"Last seen" updates while someone has hello.mesh open. It isn't a live
online indicator. Closed tabs look stale even if the person is in the room.

## Certificate warning on an `https://….mesh` app

Expected. `.mesh` apps use self-made certificates. See
[Find services](05-services.md#step-2-open-a-service).

## The router map is missing some routers

Routers running older software, or ones that don't answer at their LAN
address, drop out of the map. Everything else still works.

## Moving between rooms dropped my call or download

Moving to another router can briefly interrupt connections, and your
address may change. Seamless roaming isn't field-proven yet. Reconnect the
app.

## For agents

Checks you can run from a device on the mesh:

```sh
ip -4 addr | grep -E 'inet 10\.42\.'          # on the mesh? expect 10.42.x.n
ping -c 2 10.42.x.1                            # gateway answers? (x from the line above)
nslookup hello.mesh 10.42.x.1                  # .mesh resolution via the router
curl -s http://hello.mesh/api/health           # front desk up
curl -s http://hello.mesh/api/directory | head -c 400   # directory (node, identities, services)
```
