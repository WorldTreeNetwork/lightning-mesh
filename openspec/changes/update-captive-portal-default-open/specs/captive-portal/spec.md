## MODIFIED Requirements

### Requirement: Offer IdentiKey without gating internet

The network SHALL allow internet use as soon as a client receives network configuration, without requiring a portal action. The CAPPORT API SHALL report `captive:false`, and intercepted operating-system connectivity probes SHALL receive their expected open-network success response by default. The voluntary front desk SHALL remain available at `http://hello.mesh/` for creating an IdentiKey and discovering local services.

#### Scenario: Phone joins a mesh with an uplink

- GIVEN a client joins the mesh SSID and receives network configuration
- WHEN its operating system checks whether the network is captive
- THEN the check succeeds without user interaction and ordinary internet use is available

#### Scenario: Person chooses to visit the front desk

- GIVEN a connected client whose operating system considers the network open
- WHEN the person opens `http://hello.mesh/`
- THEN the front desk remains available without making identity creation or dismissal a prerequisite for internet use

#### Scenario: Mesh has no upstream internet

- GIVEN a client joins a mesh that currently has no usable uplink
- WHEN the person opens `http://hello.mesh/`
- THEN the mesh-local front desk remains reachable even though external internet destinations are unavailable
