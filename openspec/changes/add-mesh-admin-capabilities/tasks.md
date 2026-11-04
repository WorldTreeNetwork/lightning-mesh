# Work tracking

`mjolnir-mesh-ai0.1` is the grant library in `crates/mjolnir-admin-capabilities`.
`authorize` is a bounded decision. It returns a `ReplayObligation` and does not
execute the operation, reserve that obligation, or import meshd.

The library tests cover the acceptance contrast:

- A configuration grant attenuated to diagnostics succeeds. The same token used
  to apply a plan is refused, and a different node is refused.
- A changed holder, an appended right, an appended expiry, an unknown right,
  and a configuration grant with no expiry are refused.
- A Relaxed grant that carries a firewall change and guest Wi-Fi and claims
  30 days is valid for at most 24 hours. Network authority older than 24 hours
  is stale under Relaxed.
- An ownership right caps a mixed token at 5 minutes. A firmware right caps it
  at 15 minutes. Those names are not new wire operations.
- A diagnostics-only token may omit `expires_at`. Authority older than the
  profile stale window still refuses it.

Holder binding follows identikey-capability-v1 §7.1: the authority block
checks the fingerprint, and the verifier injects `holder(<hex>)`. It does
not assert a holder fact.

`mjolnir-mesh-b6j.2` reserves the replay obligation and carries the request.
Live radio apply stays `mjolnir-mesh-lpv`. This change stays open: the library
tests are not a fold.
