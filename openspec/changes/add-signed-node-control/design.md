# add-signed-node-control

Reconciled 2026-11-04 after `update-grant-holder-rules` landed and steer pinned the four open choices.

## Slice

One change carries:

1. Reservation of the grant library's `ReplayObligation` (target challenge, request id, authority epoch, token identity) in a caller-supplied durable ledger before the operation runs. A missing, corrupt, or unreadable ledger refuses. There is no production in-memory allow.
2. A hello HTTP adapter that forwards the signed envelope. The destination verifies. Possession of the HTTP request is not authorization.
3. An authenticated iroh path for the same typed operation, verified at the destination.

Argument bytes are the grant library's canonical request. The entry hop may forward. It may not widen the operation, the target, or the arguments.

## Pinned

- Receipt: canonical CBOR array, domain `lightning-admin/receipt`, version 1. The destination node key signs the exact canonical bytes. The array carries the `ReplayObligation` (target challenge, request id, authority epoch, token identity), the operation, and the outcome. This is not a gossip `SignedRecord`.
- Ledger: a caller-supplied trait. The node stores reserved obligations in one file. A missing, corrupt, or unreadable file refuses. There is no production in-memory allow. Tests use a temp file.
- Hello: `POST /api/control` forwards the signed envelope. It does not authorize.
- Iroh: ALPN `mjolnir/admin/1` carries the same envelope bytes. It does not reuse `mjolnir/mesh/v0` or `mjolnir/capability-beacon/1`.
- Tests: the hello handler and a local iroh endpoint pair. The entry has no node key. No fleet node and no radio.

Advise before act. Reader: Astra (`astra-arch-review`, protocol and durability).

## Reservation

One writer owns the ledger file and takes an exclusive lock for check-and-reserve. The reserved key is the token identity plus the request id, stored with the full obligation. Creating the file is an explicit initialization, separate from handling a request. A request that finds no file, a corrupt file, or a file it cannot lock or write refuses and does not execute.

Once the reservation has been persisted, it stays consumed across executor failure, cancellation, a lost reply, and restart. A failed persist does not execute. This is at-most-once dispatch. A crash after reserve does not release the obligation and does not promise that the effect completed.

The high-water floor lives in the same file and is loaded on open. An initialized file with no floor refuses. The floor only moves forward.

## Envelope and receipt

The control envelope is a definite 6-element CBOR array: domain `lightning-admin/control`, version 1, the canonical request bytes, the exact raw token bytes, the 32-byte holder key, and the 64-byte holder signature. Those four inputs are what `verify_holder_proof` already requires. The body is at most 64 KiB, rejected before parse when larger, and rejected when trailing or non-canonical.

The receipt body is a definite CBOR array: domain `lightning-admin/receipt`, version 1, target challenge (32 bytes), request id (32 bytes), authority epoch, token identity (32 bytes), the operation code from the closed grant enum, and an outcome code. Outcome 0 means the executor returned success after the reservation was durable. Outcome 1 means the destination refused, including an executor error after a consumed reservation. The destination node key signs the exact canonical receipt body. On the wire the receipt is a definite 2-element CBOR array whose first element is a byte string of that canonical body and whose second is the 64-byte signature. A receiver accepts a receipt only when the signature matches the expected destination key and the body matches the expected obligation and operation. A signature that is valid for a different obligation or operation is not acceptance. Unknown outcome codes, non-canonical bytes, and trailing bytes are refused.

Hello `POST /api/control` and iroh ALPN `mjolnir/admin/1` carry the same envelope bytes. One iroh stream is one exchange. The request is a big-endian u32 length, at most 65536, then that many envelope bytes. The response is a big-endian u32 length then either the receipt frame or an error frame. The stream ends after that response. Bytes after the length-delimited body are a protocol error.

Hello reads the node id from the canonical request inside the envelope and connects to that node on `mjolnir/admin/1`. That read does not authorize. If the envelope is not the pinned array, hello returns an error frame and does not open iroh. The forward stops at 5 seconds. The entry does not hold the destination signing key and returns the destination's response bytes unchanged.

An error frame is a definite CBOR array: domain `lightning-admin/control-error`, version 1, and a code. Code 0 is malformed input, 1 is no destination context, 2 is timeout, 3 is oversize. Those are not signed receipts. Outcome 1 on a receipt means this attempt did not report success. It does not mean the effect never happened, and the reservation stays consumed.

Adding the admin ALPN does not change `mjolnir/mesh/v0`, the tunnel ALPN, gossip, or the capability beacon.

`target_challenge` stays an opaque 32-byte replay component copied from the signed request. This slice does not add a challenge service.

## Destination context

The destination installs its own context before it serves: local node id, owner-verified authority for that node, authenticated time, the persisted high-water floor, and the node signing key. A missing context refuses before `authorize`. After `authorize`, a request whose node is not the local node refuses without execution. Transport identity and anything the entry asserts are not authority.

This slice dispatches only operations the grant library already encodes. The executor receives the verified operation and its signed arguments, not the raw envelope. `ApplyNetworkPlan` carries a plan digest and a revision, not the plan body. If that operation runs, the executor's plan bytes must match that digest and revision. A test executor records the verified operation and does not touch a radio.

## Kept from the accepted control contract

- F-A: a missing or unreadable ledger refuses privileged operations.
- F-B: the signed argument digest is the canonical request bytes from the grant library.
- Name publish is not this change.
