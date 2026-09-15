# Tasks

- [x] Make CAPPORT and intercepted OS probes report open-network success by default.
- [x] Remove the per-client completion requirement while retaining `hello.mesh` as a voluntary destination.
- [x] Update focused tests and run the `mjolnir-hello` test suite.
- [x] Build and deploy `mjolnir-hello` to the reachable active fleet, then validate a live CAPPORT response and probe payload.
  - Live on `m3000` and `m3000-b`; TR3000 recovery and deployment is tracked by `mjolnir-mesh-ri7v` because its backhaul dropped before the copy.
- [x] Fold the accepted behavior into the living spec and archive this change.
