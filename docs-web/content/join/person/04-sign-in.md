---
id: person.signin
title: Sign in to mesh apps
description: Use your IdentiKey to sign in to other .mesh apps through hello.mesh, and what an app learns about you.
path: person
order: 4
audience: [person, agent]
status: built
time: 2 min
requires: [person.identity]
next_step: person.services
verified_against: 2f6dc6b (2026-09-24)
---

# Sign in to mesh apps

Apps on the mesh can ask hello.mesh who you are. You approve, and the app
gets a signed note with your name and public key. Your private key never
leaves hello.mesh.

> **Status: built** for two shapes:
> - an app in its **own tab** sends you to `/assert`
> - a mini-app **card** on hello.mesh asks over a message port; consent is
>   drawn *outside* the card

## Path A — the app opened in its own tab

### Step 1: Start signing in from the app

**Do:** in a mesh app, tap its sign-in button.

**Expect:** your browser goes to `http://hello.mesh/assert…` and shows
"Checking…".

### Step 2: Approve or deny

**Expect:** a screen titled **"Sign in to `<app address>`?"** that says:
- "`<app address>` will learn your public key and display name."
- "You are `<your name>` `<first 8 characters of your key>`…"
- "Only your public key and name are shared — never your private key or
  recovery phrase."

**Do:**
1. Check the app address. The **address** is what you're trusting, not the
   app's name.
2. Tap **Approve** to sign in, or **Deny** to refuse.

**Expect:**
- Approve: "Signing you in…", then you're back in the app and signed in.
- Deny: you're back in the app, not signed in.

**If not:**
- "Create your identity first": you don't have one yet. Tap **Go to the
  front desk**, [create your identity](03-identity.md), then start signing in
  from the app again.
- "This sign-in request is invalid": the app sent a broken request. Tell the
  app's owner; your identity is fine.

## Path B — a card on the Apps shelf

**Do:** on `http://hello.mesh`, under **Apps**, tap **Open here**. If the app
asks who you are, a consent sheet appears **on hello.mesh**, not inside the
card.

**Expect:** the same facts as path A (audience origin, your name, short
key). Approve or deny. After approve, the card may show "Identity shared
with …".

**If not:** the card is a **link-out** tile (no **Open here**). Use path A
in the new tab. Apps on raw IP addresses always link out.

## Next time

hello.mesh remembers apps you approved, in this browser, for both paths.
An app can then sign you in without showing the screen again. There's no
screen yet to review or revoke those approvals. To reset all of them, clear
hello.mesh's site data in your browser. That also removes your key, so save
your [recovery phrase](03-identity.md#step-2-save-your-recovery-phrase-now)
first.

Next: [Find services](05-services.md).
