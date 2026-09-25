## ADDED Requirements

### Requirement: Keep-IP harness is a measured STA hop

The sim lab SHALL provide a scripted Linux STA (same sim image, STA-only)
that associates to node-a, hops to node-b, and records an ordered
pre-hop baseline plus post-hop observations. Passing SHALL NOT close
beads `mjolnir-mesh-5wc`, `mjolnir-mesh-wvg`, `mjolnir-mesh-wvg.1`, or
`mjolnir-mesh-sz9.1`. A roam command with no observed association
transition SHALL NOT be a pass.

#### Scenario: Scripted hop green

- GIVEN a STA associated to node-a with a vended `10.42` address
- AND a pre-hop baseline of MAC, IPv4, home node, LeaseBook LWW entry,
  `iw station dump` on A (present) and B (absent), usable `ip neigh` on A,
  proto 158 `/32` absent on both, and a running sequenced egress probe
- WHEN the harness hops it to node-b (log names `wpa_cli roam` and/or
  `vwifi-ctrl`)
- THEN IPv4 is unchanged
- AND the STA is in B's `iw station dump` and not in A's
- AND proto 158 `/32` is present on B only and absent on A
- AND LeaseBook still maps that MAC to the same IPv4 (current LWW)
- AND the probe logs last-success-before, first-success-after, loss
  count, and observed egress node B
- AND those metal roam beads remain open

#### Scenario: Command without transition is red

- GIVEN the harness issues a roam command
- WHEN B's `iw station dump` does not contain the STA or A still does
- THEN the harness exits non-zero

#### Scenario: Stale /32 on A is red

- GIVEN the hop
- WHEN A still has proto 158 `/32` for the STA IP
- THEN the harness exits non-zero even if B also has it

#### Scenario: Lease divergence is red

- GIVEN the hop
- WHEN LeaseBook maps the STA MAC to a different IPv4 than baseline
- THEN the harness exits non-zero

#### Scenario: Egress does not resume through B is red

- GIVEN a sequenced probe across the hop
- WHEN there is no first-success-after via B
- THEN the harness exits non-zero
