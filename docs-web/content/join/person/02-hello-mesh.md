---
id: person.hello
title: Say hello
description: Open the hello.mesh front desk — people, apps, services, and routers.
path: person
order: 2
audience: [person, agent]
status: built
time: 3 min
requires: [person.connect]
next_step: person.identity
verified_against: 2f6dc6b (2026-09-24)
---

# Say hello

Every router runs a small front desk at **hello.mesh**. It shows who's
around, which apps and services you can open, how the routers connect, and
it's where you make your identity.

> **Status: built.** An online mesh reports an open connection immediately.
> An offline mesh opens a local explanation and a hello.mesh link. No button
> controls internet access.

## Step 1: Confirm the connection

Right after you join, your phone or laptop checks whether the mesh currently
has an internet route. With one, it marks the Wi-Fi connected without a
sheet. With no route, it may open a sheet explaining that the local mesh is
still available.

**Do:** wait for the Wi-Fi indicator to show connected.

**Expect:** normal internet access without a button when any mesh router has
an uplink. With no uplink, use **Open hello.mesh** in the sheet for local
people and services. The sheet clears automatically after routing returns
and the device checks again.

**If not:** forget the network and rejoin it, then see
[Troubleshooting](troubleshooting.md).

## Step 2: Open hello.mesh

**Do:** in your normal browser (Safari, Chrome, Firefox…), type exactly:

```
http://hello.mesh
```

Type the `http://`. There is no public `https` for this name yet. Some
browsers treat a bare `hello.mesh` as a search.

**Expect:** a page that says **"You're connected to `<router name>`"** with
the subtitle "A community mesh running on the routers around you. No
internet required." On the right (or below, on a phone), a chip:
**Set up your identity** if you have none, or **You're `<name>` here**.

**If not:**
- Your browser searched instead: type the full `http://hello.mesh`.
- "Server not found": turn off Private DNS or secure DNS
  ([Troubleshooting](troubleshooting.md)).
- Still nothing: open `http://10.42.x.1`, using the router address from
  [Connect to the Wi-Fi](01-connect.md), step 3. Come back to
  `http://hello.mesh` before creating an identity. Identities made at the
  numeric address are kept separately.

If the page has People, Services, and Routers but **no Apps** section, that
node is on older software. Services still work. Ask the operator to update
hello.

## Step 3: A quick tour

From top to bottom on current software:

| Section | What it shows |
|---|---|
| **You're connected to…** | This router. Tap the identity chip to introduce yourself |
| **People** | Everyone who has introduced themselves: a name, a short key, and when they were last seen ("here now", "seen 5m ago"). No accounts |
| **Apps** | Mini-apps (`app=v1`): tap **Open here** for a card on this page, or **Open** for a new tab. Empty copy: "No apps on this mesh yet." |
| **Services** | Everything else you can open, as links. Mini-apps are *not* repeated here |
| **Routers** | The boxes in this mesh, a radio map, and **Stamp location** (optional; needs an identity) |

The page updates itself every few seconds.

### Stamp location (optional)

Under Routers, **Stamp location** lets you mark where a router sits, from
this phone.

**Do:** create an identity first. Pick the router (the one you are on is
labelled **you are here**). Tap **Use GPS**, or type latitude and longitude.
Confirm.

**Expect:** "Stamp sent. Last-known will show after the node ingests it."
Coordinates appear on that router in the list.

**If not:**
- "Create an identity first, then stamp" — do [Create your identity](03-identity.md).
- **Use GPS** fails with "GPS unavailable" — expected on `http://hello.mesh`.
  Browsers treat it as not a secure page, so they withhold location. Type
  the numbers, or skip. Trusted HTTPS names for hello are not deployed yet.
- Cancel writes nothing.

Next: [Create your identity](03-identity.md).
