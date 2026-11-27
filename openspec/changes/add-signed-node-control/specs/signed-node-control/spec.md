## ADDED Requirements

### Requirement: The destination runs a signed request once
The destination SHALL verify the grant library's authorization itself and SHALL reserve that authorization's `ReplayObligation` in a caller-supplied durable file ledger before execution. A second presentation of the same obligation SHALL NOT execute. An entry node that only forwards the envelope SHALL NOT be able to widen the target, the operation, or the arguments. On success the destination SHALL return a receipt: a canonical CBOR array with domain `lightning-admin/receipt` and version 1, covering that obligation, the operation, and the outcome, signed by the destination node key over those exact bytes.

#### Scenario: One success through an untrusted entry
- GIVEN a valid signed request whose entry hop is not the destination
- WHEN the destination verifies it and the obligation is not yet reserved
- THEN the operation executes once and the destination returns a signed receipt for that obligation

#### Scenario: Altered target or arguments
- GIVEN a valid signed request
- WHEN the target, operation, or canonical arguments differ from the signed bytes
- THEN the destination refuses and does not reserve a successful execution

#### Scenario: Replay
- GIVEN an obligation that is already reserved
- WHEN the same obligation is presented again
- THEN the destination refuses to execute it again

### Requirement: An unreadable ledger refuses
The destination SHALL refuse a privileged operation when the replay ledger file is missing, corrupt, or unreadable, or when a reservation cannot be persisted. It SHALL NOT rebuild that state as an empty allow. Creating the ledger file SHALL be an explicit initialization, not a side effect of a request. One writer SHALL own the file and SHALL make check-and-reserve atomic. A persisted reservation SHALL stay consumed across executor failure, cancellation, a lost reply, and restart. The high-water floor SHALL live in that file, SHALL load on open, and SHALL only move forward.

#### Scenario: Ledger cannot be read
- GIVEN a valid signed request
- WHEN the replay ledger cannot be read
- THEN the operation does not execute

#### Scenario: First use of an initialized ledger
- GIVEN an initialized ledger that has no reservation for this obligation
- WHEN a valid signed request arrives
- THEN the destination may execute it once

#### Scenario: Crash after reserve
- GIVEN a reservation that was persisted
- WHEN the process restarts before a receipt is delivered
- THEN the same obligation does not execute again

#### Scenario: Two concurrent presentations
- GIVEN two presentations of the same obligation at the same time
- WHEN both reach an initialized ledger
- THEN at most one executes

### Requirement: HTTP is not a bearer admin credential
Hello SHALL accept the signed envelope at `POST /api/control` and SHALL forward those bytes. Iroh SHALL accept the same bytes on ALPN `mjolnir/admin/1`. Presenting that HTTP request without a grant the destination accepts SHALL NOT authorize the operation. The destination SHALL verify either way.

#### Scenario: HTTP without a destination-accepted grant
- GIVEN a `POST /api/control` body that is not a destination-accepted signed grant
- WHEN the adapter handles it
- THEN the operation does not execute

#### Scenario: The same envelope arrives over iroh
- GIVEN a signed envelope that hello would forward on `POST /api/control`
- WHEN that same byte string arrives on iroh ALPN `mjolnir/admin/1`
- THEN the destination applies the same verification, reservation, and receipt rules

### Requirement: The envelope and the receipt have one byte layout
The control envelope SHALL be a definite 6-element CBOR array: domain `lightning-admin/control`, version 1, the canonical request bytes, the exact raw token bytes, the 32-byte holder key, and the 64-byte holder signature. Input larger than 64 KiB, trailing bytes, and non-canonical encoding SHALL be refused before authorization. The receipt body SHALL be a definite CBOR array: domain `lightning-admin/receipt`, version 1, the 32-byte target challenge, the 32-byte request id, the authority epoch, the 32-byte token identity, the closed operation code, and an outcome code. Outcome 0 SHALL mean the executor succeeded after a durable reservation. Outcome 1 SHALL mean the destination refused, including an executor error after the reservation was consumed. The destination node key SHALL sign the exact canonical receipt body. The on-wire receipt SHALL be a definite 2-element CBOR array whose first element is a byte string of that canonical body and whose second is the 64-byte signature. A receiver SHALL accept it only when the signature matches the expected destination key and the body matches the expected obligation and operation. A valid signature for a different obligation or operation SHALL be refused. Unknown outcome codes, non-canonical bytes, and trailing bytes SHALL be refused. Outcome 1 SHALL NOT be evidence that an effect never happened. Hello SHALL return the destination response bytes unchanged. One iroh stream SHALL be one exchange: a big-endian u32 length, at most 65536, then the envelope, and a response of a big-endian u32 length then either the receipt frame or an error frame. Hello SHALL choose the destination from the node id inside the canonical request and SHALL NOT treat that read as authorization. Malformed input, a missing destination context, timeout, and oversize input SHALL be an error frame, not a signed receipt. The error frame SHALL be a definite CBOR array with domain `lightning-admin/control-error`, version 1, and a closed code: 0 malformed, 1 no context, 2 timeout, 3 oversize. The forward SHALL stop at 5 seconds.

#### Scenario: A receipt cannot be moved to another request
- GIVEN a receipt whose signature is valid for one obligation
- WHEN a caller expects a different obligation or operation under the same destination key
- THEN acceptance fails even though the signature still verifies over the original body

### Requirement: The destination uses its own context
The destination SHALL refuse before authorization when its installed context is absent. That context SHALL supply the local node id, the owner-verified authority for that node, authenticated time, the persisted high-water floor, and the node signing key. After authorization, a request whose node is not the local node SHALL be refused without execution. The executor SHALL receive only the verified operation and its signed arguments. This slice SHALL dispatch only operations the grant library already encodes. For `ApplyNetworkPlan`, the executor's plan bytes SHALL match the signed plan digest and revision. `target_challenge` SHALL remain the opaque 32-byte component copied from the signed request.

#### Scenario: The request names another node
- GIVEN a valid grant for a node other than the destination
- WHEN the destination handles it
- THEN the operation does not execute

#### Scenario: No destination context is installed
- GIVEN a valid signed envelope
- WHEN the destination has no installed context
- THEN the operation does not execute
