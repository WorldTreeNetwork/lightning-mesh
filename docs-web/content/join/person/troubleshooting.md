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
verified_against: 2f6dc6b (2026-09-24)
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
| A sheet says the local mesh is available | Expected when the router has no internet route. Use its hello.mesh link for local services |

## A captive welcome sheet appeared

This is expected when the mesh has no internet route. It explains the offline
state and links to `http://hello.mesh`; no login or dismissal unlocks internet.
If a real internet site already loads while the sheet remains, close it and
rejoin. Tell the operator that CAPPORT and the router's default route disagree.

## My laptop dropped its working internet when I joined

Expected. One radio, one Wi-Fi. Leave the mesh SSID to get the old network
back. Operators who must poke a node without internet on the client SSID use
the project's mesh-errand script, not a manual default-route change.

## I don't see an Apps section

That hello.mesh is an older build. Use **Services**. Ask the operator to
update the hello binary (`build-hello.sh` then fleet update).

## Apps says "No apps on this mesh yet" but I published something

Operator publish without `--txt app=v1` is a **Service**, not an App. Add
the marker and a `/.well-known/mesh-app.json` — see
[Mini-apps](../publish/02-mini-apps.md).

## "Use GPS" fails when I stamp a router

Expected on `http://hello.mesh` (not a secure context). Type latitude and
longitude, or skip. You need an identity before Confirm will send.

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
[Find services](05-services.md#step-2-open-a-service-or-app). Camera and
mic will often stay blocked until trusted HTTPS names exist.

## The router map is missing some routers

Routers running older software, or ones that don't answer at their LAN
address, drop out of the map. Everything else still works. Two boxes that
never exchanged **peer ids** will not show as one mesh — see
[Add another router](../house/06-add-router.md).

## Moving between rooms dropped my call or download

Same SSID, often the same DHCP IP, but a live session can still break.
Reconnect the app. Details:
[Connect — roaming](01-connect.md#walking-around-roaming).

## LEDs don't match "ready"

Lamp names differ by box (Cudy M3000 red/white, TR3000 red/white, AP3000
red/green, WR3000S white). The WPS LED is the WAN SSH window, not mesh
health. If client Wi-Fi is on the air and `10.42.` works, the node is up.

## For agents

Checks you can run from a device on the mesh:

```sh
ip -4 addr | grep -E 'inet 10\.42\.'          # on the mesh? expect 10.42.x.n
ping -c 2 10.42.x.1                            # gateway answers? (x from the line above)
nslookup hello.mesh 10.42.x.1                  # .mesh resolution via the router
curl -s http://hello.mesh/api/health           # front desk up
curl -s http://hello.mesh/api/directory | head -c 400   # directory (node, identities, services)
curl -s http://hello.mesh/api/apps | head -c 200        # mini-app manifests (404/empty on old hello)
```
