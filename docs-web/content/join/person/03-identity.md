---
id: person.identity
title: Create your identity
description: Make an IdentiKey on hello.mesh, save its recovery phrase, and understand what it protects and what it does not.
path: person
order: 3
audience: [person, agent]
status: built
time: 3 min
requires: [person.hello]
next_step: person.signin
verified_against: 2f6dc6b (2026-09-24)
---

# Create your identity

An **IdentiKey** is a name plus a cryptographic key that you hold, not an
account on someone's server. It lets people on the mesh recognize you, and
lets mesh apps know it's you. It works with the internet unplugged.

> **Status: built.** Deployed on the fleet.

## Before you start: what kind of key this is

hello.mesh shows this notice, and it's worth reading:

> **Soft custody.** A browser-created identity's key lives in this browser
> and can be read by whichever node is currently serving this page. It is
> *not* equivalent to an app or hardware key you alone hold. Fine for a
> disposable, low-value identity — never for anything you'd regret someone
> else touching.

Use it to be known on the mesh and to sign in to mesh apps. Don't use it to
guard anything valuable.

## Step 1: Introduce yourself

**Do:**
1. Open `http://hello.mesh` in your normal browser.
2. Tap the **Set up your identity** chip (top of the page).
3. Under **Introduce yourself**, type a name in **Your name** (up to 48
   characters).
4. Tap **Join**.

**Expect:** the status changes to "Introducing you to the mesh — this takes
up to about 20 seconds.", then "Syncing your name across the mesh…", then
**"You're `<name>` here"**.

**If not:**
- "Couldn't reach this node to announce you. Try again.": tap Join again.
- "Still introducing you — retrying." after a minute: the mesh is busy.
  Leave the page open; it keeps trying.
- A warning that you're "here by IP address": open `http://hello.mesh`
  instead of the numeric address, then start again.

## Step 2: Save your recovery phrase now

The recovery phrase **is** your identity. If you clear your browser data,
use private browsing, or lose the phone, the phrase is the only way back.

**Do:**
1. In the identity panel, tap **Show recovery phrase**.
2. Write the 24 words down on paper, in order.

**Expect:** 24 numbered words and the warning "**Anyone with these words is
you.** Write them down offline. Never paste them into anything you don't
trust."

Keep the phrase offline. Anyone who has it can act as you.

## Step 3: Check that others can see you

**Do:** look at the **People** section, or ask someone nearby to look at
theirs.

**Expect:** your name with a **you** badge on your screen, and your name with
"here now" on theirs. It usually shows up everywhere in under a minute.

## What gets shared

| Shared with the whole mesh | Stays in your browser |
|---|---|
| Your chosen name | Your private key |
| Your public key | Your recovery phrase |
| When you were last seen | Which apps you've approved |

Your name and public key are visible to anyone on the mesh. Any web page on
the mesh can read the directory. Pick a name you're happy to show.

## Change or replace it later

| To… | Do |
|---|---|
| Rename yourself | Edit **Your name**, then tap **Rename** |
| See or copy your full public key | **Show full key**, then **Copy key** |
| Use your identity in another browser or device | [Take your identity with you](06-leave.md) |
| Switch to a different identity | **Restore a different identity**. This replaces the current key; save its phrase first |

You can't delete an identity from the mesh yet. Your name stays in the
People list (folded under "Not seen recently" after 24 hours). If you stop
using a key, that entry simply goes quiet.

Next: [Sign in to mesh apps](04-sign-in.md).
