---
id: person.leave
title: Take your identity with you
description: Restore your IdentiKey on another device or mesh, and what does and does not leave with you.
path: person
order: 6
audience: [person, agent]
status: partial
time: 2 min
requires: [person.identity]
next_step: null
verified_against: 2f6dc6b (2026-09-24)
---

# Take your identity with you

Your identity belongs to you, not to this network. You can walk away with it
and use it on another device or another Lightning Mesh, without anyone's
permission.

> **Status: partial.** Taking your key with you is built. Deleting your
> entry from a mesh isn't built yet.

## Step 1: Export

**Do:** on `http://hello.mesh`, open your identity and tap **Show recovery
phrase**. Write down the 24 words, or tap **Copy raw hex** for the 64-character
seed.

**Expect:** 24 words from the standard BIP39 word list. They encode your raw
32-byte Ed25519 key directly. They aren't tied to this mesh or this software,
but they're **not a crypto-wallet seed phrase**. A wallet app that derives
keys from BIP39 words produces a different key, so restore only into
hello.mesh or another tool that uses the same raw-key encoding.

## Step 2: Restore somewhere else

**Do:**
1. Join any Lightning Mesh and open `http://hello.mesh` in a browser.
2. Tap **Set up your identity**, then **Already have a recovery phrase? Restore
   it**.
3. Paste the 24 words, or the 64-character hex, and tap **Restore identity**.

**Expect:** "You're `<name>` here", with the same public key as before. People
and apps that know your public key recognize you.

**If not:** check the words are in order and spelled correctly. On a
browser that already holds a different identity, use **Restore a different
identity**. It replaces that key, so save its phrase first.

## What leaves with you, and what stays

| Leaves with you | Stays behind |
|---|---|
| Your private key (the phrase) | Your entry in this mesh's People list (name, public key, last seen). It can't be deleted yet |
| Your public key, so apps still recognize you | The apps you approved, remembered in that browser only |
| | Anything an app stored about you. Ask that app |

If a key has been exposed, there's no way to rotate it in place. Make a new
identity. The old entry goes quiet, and anything that trusted the old public
key needs to learn the new one.

Back to the [overview](../index.md), or [troubleshooting](troubleshooting.md).
