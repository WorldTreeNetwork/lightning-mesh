# captive-portal

The OS captive-portal sheet explains an offline mesh and points people to
hello.mesh. This is not a walled garden: the mesh never blocks traffic, and
user action never controls internet access.

Code: `crates/mjolnir-hello/src/portal.rs`.

## ADDED Requirements

### Requirement: Describe internet availability without gating it

The network SHALL allow internet use without a portal action whenever the router has a usable default route. In that state the CAPPORT API SHALL report `captive:false`, and intercepted operating-system connectivity probes SHALL receive their expected open-network success response. When the router has no default route, CAPPORT SHALL report `captive:true` and intercepted probes SHALL serve a self-contained page explaining that internet is unavailable while local mesh services remain reachable at `http://hello.mesh/`. A dismissal or identity action SHALL NOT override either state.

#### Scenario: Phone joins a mesh with internet routing

- GIVEN a client joins the mesh and the serving router has a default route
- WHEN its operating system checks whether the network is captive
- THEN the check succeeds immediately without user interaction

#### Scenario: Phone joins a mesh without internet routing

- GIVEN a client joins the mesh and the serving router has no default route
- WHEN its operating system checks whether the network is captive
- THEN the operating system receives an offline portal that links to the local front desk

#### Scenario: Internet routing returns

- GIVEN an offline portal was previously shown
- WHEN a usable default route appears and the operating system checks again
- THEN CAPPORT reports open and the probe receives its exact success response without a dismissal action

#### Scenario: Legacy pass endpoint is called

- GIVEN either internet-availability state
- WHEN a client calls the compatibility pass endpoint
- THEN the call succeeds without changing the state derived from routing
