# Add durable network apply

> **ACTIVE BUILD**

**Rigor:** instrument

## Why

The wireless-only pilot works, but repeatable household configuration needs more
than a detached shell and a final result file. The current applier survives SSH
loss, not a reboot; it reports restoration without checking restored connectivity.
A stale preview or interrupted radio change must not strand a house or claim success.

## What

Add `network-change-transactions`: revision-checked plans, durable per-node
transaction state, reboot recovery, and receipts that distinguish local health
from demonstrated downstream client internet. Rigor: **instrument**. Activation
inherits ai0; live tests or deployment require separate authorization.

## Impact

- ADDED capability: `network-change-transactions`.
- Integrates with `mjolnir-apply`, its launcher and a boot recovery service.
- Proposed architecture: one target-node transaction owner; shared receipts for
  future desktop/web administration, not separate competing appliers.

## User journey & surfaces

An administrator previews a change, sees which radio/client service it spends,
and applies against the observed revision. The controller may disconnect: on
reconnect it retrieves the same transaction by ID rather than resubmitting blindly.
Working shows committed configuration and separately measured connectivity;
empty shows no transaction; failed shows restored or recovery-required with a
physical reconnect path; local-only policy does not display a false internet failure.
No new UI because this slice supplies the receipt/control contract consumed by
`bf7.7`; implementation initially exercises the local helper and test adapter.

## Out of scope

- Uplink/wired/private/guest renderers: `bf7.2`, `ai0.6`, `ai0.7`.
- Cryptographic request admission: `b6j.2`, `ai0.1`, `ai0.9`; existing root SSH
  remains a maintenance path, not the shipped household authorization model.
- Hardware power-cycle/RF qualification: `lpv`, `z3th`; no live writes here.
