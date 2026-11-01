## ADDED Requirements

### Requirement: An admitted foreign prefix is a TUN device route

Admin SHALL install a route for a foreign house `/48` only when `admitted_pairs` contains a pair that includes this house and an app TUN session is up. The route SHALL point at that TUN and SHALL NOT assign the `/48` on the interface. Omitting either admit op, so the pair is absent, SHALL remove the route. There SHALL be no withdrawal op. A stock OpenWrt prefix, this house's own `/48`, and a pair that does not include this house SHALL NOT gain a route. This SHALL NOT change Babel or the Iroh bind set.

#### Scenario: Both directions install the foreign prefix

- GIVEN this house is guild `11` repeated and the presented set admits guild `22` repeated
- WHEN the app TUN session syncs routes
- THEN `fd8f:78a9:2caa::/48` is a route on that TUN and `fd81:2b22:8aca::/48` is not assigned on it

#### Scenario: Dropping one direction removes the route

- GIVEN the foreign `/48` route is installed
- WHEN the caller presents no pair
- THEN that `/48` route is gone

#### Scenario: A stock prefix is not routed

- GIVEN the presented pair admits only the foreign house
- WHEN routes are listed
- THEN `fd12:3456:789a::/48` is absent
