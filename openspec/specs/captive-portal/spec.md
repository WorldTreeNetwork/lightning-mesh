# captive-portal

The OS captive-portal sheet is how a phone finds hello.mesh. This is not
a walled garden: the mesh never blocks traffic. The page offers IdentiKey
and one pass-through.

Code: `crates/mjolnir-hello/src/portal.rs`.

## ADDED Requirements

### Requirement: Offer IdentiKey, or just the internet

The portal page SHALL offer two actions only: create an IdentiKey (link
to `http://hello.mesh/`) and a pass-through labeled “Just the internet,
please”. The page SHALL NOT show a separate “No thanks” control. The
pass-through SHALL POST `/api/portal/pass` and then re-trigger the OS
probe so the sheet can close.

#### Scenario: Phone joins the mesh SSID

- GIVEN a client OS opens its captive-portal sheet on this node
- WHEN the stranger sees the page
- THEN they can create an IdentiKey or take “Just the internet, please”,
  and nothing is blocked either way

#### Scenario: Decline is one button

- GIVEN the portal HTML
- WHEN a reader searches for decline copy
- THEN “Just the internet, please” is present and “No thanks” is absent
