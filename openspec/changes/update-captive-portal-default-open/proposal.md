# update-captive-portal-default-open

> **ACTIVE BUILD**

## Why

Clients already have unrestricted packet forwarding, but reporting `captive:true` and intercepting OS connectivity probes makes macOS and other clients wait in a captive-assistant flow until someone presses a special completion button. Internet use should begin as soon as DHCP succeeds.

## What

- Modify `captive-portal` so the CAPPORT API reports the network open by default.
- Return each operating system's expected connectivity-probe success response without requiring per-client release state.
- Keep `hello.mesh` available as the voluntary front desk.

## Impact

- Capabilities: MODIFIED `captive-portal`
- ADRs: none

## User journey & surfaces

A person joins the existing Lightning Mesh SSID and immediately gets normal internet connectivity without a captive sheet or completion action. They can visit `http://hello.mesh/` whenever they want the local front desk and IdentiKey flow. If there is no upstream internet, the mesh-local front desk remains reachable.

## Out of scope

- Changing SSID security or association policy.
- Adding an internet availability claim to the CAPPORT response.
- Changing gated private-community modes described outside this capability.
