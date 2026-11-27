# Publish an EdgeBox service onto Lightning Mesh

An operator can already publish a LAN device through its router. This change lets an SSD-equipped Raspberry Pi EdgeBox advertise a service it hosts, under its own persistent identity. The observer from PR #12 provides an independent view of the announcements during bring-up.

## First working service

- Configure one service on the Pi with a name and an address and port reachable from a mesh client.
- Start the publisher and verify the other AP lists the service in hello.mesh. Open the actual service from a client on that AP.
- Reboot the Pi and verify that the same identity advertises the same service and a client can reach it again.

Use the existing service publication protocol and keep publication explicitly opt-in. Running the observer continues to originate zero application records. Select the ownership and refresh rules against the router implementation before building the publisher.

## Prototype evidence

The [original active-node commits](https://github.com/WorldTreeNetwork/lightning-mesh/commit/461826fc6b8052553596fc8cfc889fae62f3b276) remain available for reference. The prototype lists learned services but lacks a local publish operation. Its synthesized direct address list can be empty, and the saved service books are not restored on startup. The first implementation should verify the reachable endpoint and restart behavior through the client journey above.
