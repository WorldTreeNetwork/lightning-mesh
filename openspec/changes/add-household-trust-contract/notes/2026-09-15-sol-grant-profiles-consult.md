# Consult: admin grant security profiles and token lifetimes

- **Reader:** Sol (`gpt-5.6-sol` via Codex, read-only, high effort)
- **Date:** 2026-09-15
- **Requested by:** Duke ("too strict; want an option for unlimited or a week for less security-needful networks; default to a middle level")
- **Status:** second opinion only (not an advise verdict). Decision recorded in `steer.md` when made.
- **Verbatim output below.**

---

> **CONSULT:** caution

Duke’s direction is sound if “relaxed” means longer but still bounded mutation authority. I dissent from unlimited configuration-changing or delegated Admin grants. Preserve the two-timer model: long token lifetime improves convenience; a shorter stale-authority window limits partitioned-node exposure.

### Recommended profiles

These values apply to ordinary low/medium-risk configuration grants.

| Profile | Token default | Hard ceiling | Stale-authority maximum | Renewal |
|---|---:|---:|---:|---|
| Strict | 15 min | 1 hour | 15 min | Explicit renewal |
| **Standard — default** | **24 hours** | **7 days** | **24 hours** | Transparent local renewal by an unlocked owner signer; delegates require owner-authorized reissue |
| Relaxed | 7 days | 30 days | 7 days | Same renewal rules; explicit opt-in warning and persistent risk indicator |

Owners may shorten either timer. Lengthening the stale window requires switching profile; advanced controls should not silently exceed profile ceilings. Profile changes apply only to newly issued grants, preserving the existing non-extension rule ([trust design:117](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:117)).

“No wall-clock expiry” should exist only for non-sensitive, read-only diagnostics on the owner’s registered, holder-bound device. It must still terminate on holder revocation or authority-epoch change and must exclude logs containing secrets, personal data, key export, or configuration mutation. Never allow unlimited delegated Admin grants. A durable owner credential may be long-lived, but it is an issuer identity—not a standing execution capability.

### Default: Standard

Use Standard. Twenty-four hours comfortably covers self-installation and maintenance without turning a stolen or ex-housemate device into a month-long control key. A locally present owner signer can renew without Internet, so ease of setup should come from seamless local issuance—not unlimited stale authority.

The network must define “fresh” as locally verifiable signed authority state, not cloud connectivity. Ordinary Internet and hub use already remain outside these timers ([trust design:121](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:121)). Relaxed should warn plainly: “A removed administrator may retain control of an isolated router for up to 7 days.”

### Never relax

- Initial claim requires reviewed device-specific proof plus a request-bound physical ceremony ([trust design:28](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:28)).
- Recovery requires an enrolled credential plus physical presence; losing both leads only to destructive reset. Recovery rotates authority epochs ([trust design:38](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:38)).
- Complete Biscuit verification, approved issuer, holder binding, target/operation/arguments/epoch binding, narrow-only attenuation, and byte-exact canonical requests.
- Durable one-use acceptance and replay refusal. Do not promise exactly-once hardware success; retries may truthfully return an unknown outcome ([trust design:67](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:67)).
- Verified revocation applies immediately. Reconnected nodes synchronize epoch/revocation state before processing administrative requests; the UI never claims global enforcement without node acknowledgements ([trust design:82](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:82)).
- Lost/corrupt time, authority, or replay state fails closed for privileged operations. Clock rollback cannot restore authority.
- No reusable bearer authority or protected keys in HTTP, URLs, logs, or routers ([trust design:87](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:87)).
- Profiles must never make ordinary connectivity, owner key export, or owner data portability depend on a vendor service.

### Grant-class rules

- **Harmless public diagnostics:** no administrative grant needed.
- **Private read-only diagnostics:** owner devices may use no-wall-clock-expiry, epoch-bound grants; delegates get at most 30 days.
- **Guest Wi-Fi and constrained device management:** profile table applies. Delegation is acceptable when restricted to the guest network and named targets.
- **Primary Wi-Fi, backhaul, radio, routing, firewall, DNS, or uplink:** never unlimited. Hard cap 24 hours under Standard/Relaxed and 1 hour under Strict; stale authority never exceeds 24 hours.
- **Firmware/software installation:** single transaction, 15-minute maximum, artifact digest bound into the request; owner approval by default.
- **Ownership, issuer, recovery-policy, or protected-key changes:** owner-only, single-use, five-minute maximum. Claim/recovery ceremonies remain mandatory where applicable.

A token spanning classes inherits the shortest applicable ceiling.

### Main threats and cheapest mitigations

- **Stolen unlocked device:** OS-backed non-exportable holder key, Admin-app lock/biometric presence, quick device removal, and epoch bump.
- **Ex-housemate:** one-action revoke-and-bump-epoch; display which nodes acknowledged it.
- **Partitioned node:** finite stale window, process authority updates before queued control requests, and show worst-case remaining exposure.
- **Lost owner device:** separate enrolled recovery credential plus physical ceremony; rotate epochs afterward—no cloud backdoor.
- **Clock rollback/state loss:** persisted monotonic floor and fail-closed privileged access, matching finding F-A ([review:128](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/reviews/2026-09-15-advise.md:128)).

### Exact document changes

1. Append a new dated decision to [steer.md:13](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/steer.md:13) that explicitly supersedes the current “FINAL” values; preserve the history.
2. Replace the universal fixed values in [trust design:94](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/design.md:94) with the profile table, class overrides, no-expiry boundary, and local-renewal semantics.
3. Revise [authority spec:3](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/specs/household-authority/spec.md:3) to require a finite stale window for every mutating grant, Standard as default, class ceilings, future-only policy changes, and most-restrictive-cap composition. Add scenarios for Relaxed partition exposure, no-expiry read-only access, epoch revocation, and high-risk operations.
4. Replace both the “FINAL” notice and time interface in [admin design:3](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-mesh-admin-capabilities/design.md:3) and [admin design:69](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-mesh-admin-capabilities/design.md:69); align [admin spec:32](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-mesh-admin-capabilities/specs/mesh-admin-capabilities/spec.md:32).
5. Parameterize issuer-revocation-under-partition tests across profiles, extending F-D ([review:154](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/reviews/2026-09-15-advise.md:154)).

This is a policy-parameter revision, not a repudiation of the accepted trust architecture: the review explicitly classified the magnitude as a human policy decision ([review:161](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-household-trust-contract/reviews/2026-09-15-advise.md:161)). Because normative SHALLs change, run a targeted timing/profile re-review with the time-evidence design. Unlimited mutating authority or removal of the stale bound would instead change the trust contract and require full re-review.

_Process note: the `intention:consult` dispatcher could not create its scratch directory under the read-only sandbox; no files were modified._