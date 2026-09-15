# Future exploration: open backhaul and a right to pass

> PARKED direction requested 2026-09-14. Outside initial household implementation.
> Open backhaul is tentative; no live radio or payment changes are authorized.

## Intent and boundaries

Resident Wi-Fi remains private, with optional isolated guest Wi-Fi. Separately,
explore an open node backhaul where unfamiliar nodes can discover/connect but
forwarding traffic requires an explicit **right to pass**. A future payment method
could move tokens on a locally synchronized EVM. This does not assert that an EVM
or payment integration currently exists in this repo.

Radio participation, transit permission, house membership, and administration are
separate. Payment must not confer resident access, house secrets, routing authority,
or admin rights. Initial resident internet and optional hello.mesh identity flows
do not acquire a wallet or payment requirement.

## Possible architecture

```text
Discover transit offer → accept terms → submit payment
  → verify settlement → issue right-to-pass capability → enforce forwarding
```

An owner-authorized verifier could check settlement and issue a bounded,
holder-bound IdentiKey/Biscuit transit grant. Forwarding nodes enforce destination,
expiry, quota/rate and revocation limits on the data path. A transaction hash or
token balance alone is not authorization. Free, reciprocal, sponsored, and paid
passage are possible policies; no commercial terms or wire protocol are chosen.

## Resolve before activation

- Scope: one relay, a multi-hop path, or internet egress; who receives payment and
  compensates intermediate nodes; how service failure and refunds work.
- Local EVM: chain/asset identity, validator ownership, consensus and finality.
  Synchronization alone does not establish an authoritative settlement history.
- Partitions: refuse new grants, use bounded provisional credit, or pre-funded
  allowances? Define reconciliation and exposure; do not assume independently
  spent balances in disconnected partitions are valid after merging.
- Replay, duplicate redemption, reorgs, expiry, clock rollback and revocation;
  distributed usage accounting without double charging or trusting false reports.
- Bootstrap access to discover offers and reach settlement before paying, with
  bounds preventing that access from becoming unrestricted free transit.
- Trust boundaries: current setup places mesh in the LAN firewall zone and uses
  an 802.11s forwarding island. It is not a safe public-transit admission system
  merely because capabilities are added to an application API.
- Protection from untrusted routing advertisements and management requests;
  payload encryption above an open radio; abuse limits and hardware budgets.

## Future UX

An operator explicitly enables “Offer transit,” selects terms and resource limits.
A connecting party views scope, price and expiry before accepting. Show settlement
pending separately from transit active. No silent charging on discovery or roaming.
Local recovery must not depend on obtaining paid access first.

Activate this exploration independently after the initial household setup work.
