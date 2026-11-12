# add-signed-node-control

> **ACTIVE BUILD**

**Rigor:** instrument

## Why

A destination must run one signed admin request once, even when the entry hop is untrusted, and must refuse a changed target, changed arguments, a replay, or an unreadable replay ledger. The grant library returns a `ReplayObligation` and does not reserve it. Nothing in hello or iroh carries that request yet.

## What

The holder-rule engine has landed. The first landing is one change with three pieces:

- A caller-supplied durable ledger that reserves `ReplayObligation` before execution and fails closed when the record is missing, corrupt, or unreadable.
- A hello HTTP adapter that forwards the signed envelope and does not treat HTTP as a bearer admin credential.
- An authenticated iroh RPC for the same typed operation. The destination verifies. The entry node does not.

Capability: `signed-node-control`. Tracker: `mjolnir-mesh-b6j.2` and node `mjolnir-mesh-ai0.10.2`.

## Impact

- Capabilities: ADDED
- Architecture record: amend when the receipt and transport shape lands in the tree. The observe word is omitted here on purpose.

## User journey & surfaces

An owner, from Lightning Admin or from hello on a node that is not the target, submits one signed operation.

- Working: an initialized ledger has no reservation for this obligation, the destination accepts the grant, and it executes once and returns a signed receipt.
- Empty: there is no grant. The operation does not run.
- Failed: altered target or arguments, a second presentation of a consumed obligation, a missing or unreadable ledger file, or a failed persist. Each refuses. A ledger file that exists and simply lacks this obligation is first use, not a failure.
- Off this change: a fleet rollout, name publish, and any new admin wire operation.

## Out of scope

- Fleet deploy and live network writes. This change adds code and tests.
- Name publish. It stays its own scope on `b6j.2`.
- Folding `add-mesh-admin-capabilities` or closing `ai0`.
- Browser-only owner custody and WebAuthn. The installed signer remains the first signer.

## Preparation

- Upstream artifacts: `ReplayObligation` in `crates/mjolnir-admin-capabilities`. Holder-headed rules are refused as of `9cc39ed`, folded into `openspec/specs/mesh-admin-capabilities/spec.md` at `405face`. Hello has no control route yet. Mesh iroh ALPNs in use are `mjolnir/mesh/v0`, `mjolnir/mesh/tun/v0`, and `mjolnir/capability-beacon/1`.
- Dependency: `mjolnir-mesh-ai0.10.1` is closed. This node is dependency-ready.
- Outcome: reconciled from steer on 2026-11-04. Receipt is `lightning-admin/receipt` version 1 signed by the destination node key. Ledger is a caller-supplied file. Hello is `POST /api/control`. Iroh ALPN is `mjolnir/admin/1`. Tests are a local HTTP handler and a local iroh pair.
- Authority: user “activate and run” on 2026-11-04 for `mjolnir-mesh-ai0.10`, after steer. Bounds stay the intend non-goals: no fleet deploy, no name publish, no new admin wire operation. Advise before act. Reader: Astra.
- Rechecked 2026-11-05 at `43f1df3`. Outcome: still valid. Holder refusal is still `reject_asserted_holder` in `engine.rs`. Astra’s accept still covers this contract. `meshd` moved after that review (`c3092da` dials every learned Lightning address; `43f1df3` skips an onlink route with no gateway). The admin ALPN has to land beside those paths. `mjolnir-mesh-z7gl` holds an empty-path lease, so this node is deferred until that write-set is known.
