## ADDED Requirements

### Requirement: Keep-IP e2e fails while A still announces proto 158

The sim e2e runner SHALL expose `keep-ip` that hops a Linux STA A→B.
The run SHALL fail if proto 158 `/32` for the STA IP remains on A
after the hop, even if B also has it. The run SHALL NOT close
`mjolnir-mesh-5wc`, `wvg`, `wvg.1`, or `sz9.1`. Dual-158 SHALL NOT
be marked expected-fail.

#### Scenario: Dual /32 is red

- GIVEN a STA that kept its IPv4 across A→B
- AND proto 158 `/32` is present on both A and B
- WHEN `deploy/sim/e2e/run.sh keep-ip` finishes
- THEN the command exits non-zero
- AND `5wc` remains open
