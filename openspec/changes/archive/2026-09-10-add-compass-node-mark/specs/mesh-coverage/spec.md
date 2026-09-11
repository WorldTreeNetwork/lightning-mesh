## ADDED Requirements

### Requirement: A phone stamp writes last-known coordinates

hello.mesh on a node's LAN gateway SHALL let an associated phone pick a
node from a nearby-narrowed list (associated node default; others ranked
by radio strength when known) and enter GPS (geolocation or typed). A
confirmed stamp SHALL POST a signed claim that hello verifies and spools
for meshd. An empty list, a cancel, a missing GPS with nothing typed, or
a bad signature SHALL write nothing.

#### Scenario: Nearby pick

- GIVEN a phone associated to node A's client AP with GPS available
- WHEN the operator confirms node A (the default) and the GPS
- THEN node A's directory coordinate equals that GPS and names the
  phone key as stamper

#### Scenario: Pick a neighbor

- GIVEN nearby radio strength shows node B stronger or the operator
  is standing at B
- WHEN the operator selects B from the narrowed list and confirms GPS
- THEN node B's directory coordinate is written, not A's

#### Scenario: Miss

- GIVEN the operator cancels, or the list is empty, or GPS is missing
  and the typed fields are empty
- WHEN no stamp is confirmed
- THEN directory coordinates are unchanged
