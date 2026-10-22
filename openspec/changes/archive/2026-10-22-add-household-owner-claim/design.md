# Household owner claim

## Ceremony pin

Selected for review before implementation. No TPM, no factory attestation,
and no custom unaudited key exchange.

A claim or recovery request is accepted only when every required factor
verifies against the same request, node id, and ownership epoch.

1. **Device proof.** The node's meshd identity key signs the request:
   owner public key, node id, epoch, ceremony nonce, and window expiry.
   The node id is the key this mesh already derives addresses from.

2. **One-time provisioning secret.** Printed on a card that ships off the
   device. The node never displays it, including after reset. The card carries the full node id. `k = HMAC-SHA256(salt, secret)`. The salt
   is printed on the card or carried in the node-signed challenge. The node
   stores `k`, not the card string. `k` is secret-equivalent for forging
   proofs; reading it off flash is physical compromise. The card secret has
   at least 80 bits of entropy (16 lowercase RFC 4648 base32 characters).

3. **Physical window.** The local setup button (the same button position as
   WPS on these devices) only arms presence. It opens a configurable window
   that defaults to 120 seconds, closes on success, reboot, or loss of
   trustworthy timing, and is consumed by one success. The WPS protocol is
   not used.

Secret without the button fails. Button without the secret fails. A remote
presentation of the secret fails because the window was never armed on the node.

The client verifies the node signature against the node id printed on the card
before it sends anything derived from the secret. That signature is the
purchaser's check that the entry router is the intended device. The node does
not treat its own signature as an acceptance factor.

The client then sends HMAC-SHA256(k, owner public key || node id || epoch ||
ceremony nonce). The nonce comes from the node-signed challenge. The raw
secret and `k` never reach hello. Card entry, owner key generation, and the
HMAC run only in Lightning Admin. hello.mesh may point at that app. It has
no secret field. A window allows 5 failed proofs and then closes. Spending
those attempts is denial of service only; the purchaser presses again. The
nonce is fresh per attempt, so a sniffed MAC does not replay.

Order is request, then press. The press consumes the window for that request
only. meshd compares the MAC, owns the window, and runs the compare-and-swap.
The rc.wps hook only signals meshd. hello only relays, with a 5 second wait,
and does not use the async name spool for this request.

Button dispatch has three states. With a verifier present and the node
unclaimed, or with a pending recovery whose credential has already verified,
the press arms the ceremony and does not toggle WAN SSH. With no verifier
(developer-owned, or reset before a provisioning image), the press keeps the
existing WAN-SSH toggle. On an owned node with no pending verified recovery,
the press toggles WAN SSH and does not arm a ceremony.

The first complete claim is persisted atomically against the node id and moves
the epoch from unclaimed to 1. A racing second claim cannot overwrite it.

A local reset clears the ownership binding and the stored verifier. It does
not restore the original card. The node stays unclaimed until a provisioning
image writes a new verifier. The node id (device key) is unchanged. Fleet
nodes that were installed without a card stay developer-owned until that
image is applied. They are not implicitly claimed.

## Recovery pin

Recovery requires an enrolled recovery credential and a fresh physical window
bound to that request, node, and current epoch. Neither factor is enough.
Success, and any completed ownership transfer, retires every prior recovery
credential and advances both the ownership epoch and the authority epoch, so
outstanding grants from the previous epoch stop authorizing once a node has
verified the new epoch. The new owner re-enrolls. Enrollment is epoch-scoped.
A node holding one of two forked heads refuses privileged operations until a
successor exists. Ordinary forwarding continues.

If two partitions each complete recovery, this change may persist two candidate
heads. It does not pick a winner. `add-control-record-integrity` refuses a fork
until one successor chains from a single parent under a new physical ceremony.

## Clocks

The 120 second figure is only the physical window. Configuration grants use
the household-authority profiles (Standard default: 24 hour token / 7 day
ceiling / 24 hour stale-authority maximum) and the class caps in that contract.
This change does not reintroduce a 900 second admin-grant default.

## Keys

hello.mesh HTTP handlers never receive the owner private key, the raw
provisioning secret, or a reusable admin bearer. The MAC comparison runs in
meshd.

## Not owed here

- Live button presses on fleet hardware (`mjolnir-mesh-lpv` qualifies hardware).
- Implementing the Biscuit engine.
- Verifying gossiped records on a non-writer. That is admin-record-integrity.
