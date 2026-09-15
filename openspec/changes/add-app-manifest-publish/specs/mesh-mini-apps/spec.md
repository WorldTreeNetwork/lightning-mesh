## ADDED Requirements

### Requirement: App marker accepted on every publish path

The system SHALL accept the mini-app marker (version `1` and an optional entry
path) on every path that publishes a name. That covers key-owned name claims
(`POST /api/name-claim`, as an optional `app` object outside the signed message)
and the node control API (`POST /v0/publish` and the `mjolnir-meshd publish`
command, as TXT `app` and `path`). All paths SHALL validate the marker with one
shared rule set:
- version equals `1`
- the path begins with `/`
- the path has no `//`, `\`, scheme, or control characters
- the path is at most 256 bytes

An invalid marker SHALL be rejected with an error before anything is spooled or
published, and SHALL NOT alter an existing lease or service. A published marker
SHALL appear in the directory as TXT `app=v1` and, unless the path is `/`,
`path=<path>`. Consumers SHALL still re-validate the marker.

#### Scenario: Self-serve mini-app without SSH

- GIVEN an app holding a valid key-owned lease on `walkie-talkie`
- WHEN it renews with a valid signature and `"app":{"v":1,"path":"/"}`
- THEN within one sweep the directory lists `walkie-talkie` with TXT `app=v1` and the name appears in `/api/apps`

#### Scenario: Invalid path on a name claim

- GIVEN an app holding a valid lease without an app marker
- WHEN it submits a validly signed claim with `"app":{"v":1,"path":"//evil.example"}`
- THEN the request is rejected with `400`, nothing is spooled, and the existing lease is unchanged

#### Scenario: Unsupported version on the control API

- GIVEN an operator on a router
- WHEN they run `mjolnir-meshd publish foo --port 80 --txt app=v2`
- THEN the publish is rejected with a reason naming the unsupported version and no service record changes

#### Scenario: Marker omitted on renewal

- GIVEN a lease whose last claim carried an app marker
- WHEN the owner renews without an `app` field
- THEN the directory entry no longer carries TXT `app` or `path`

#### Scenario: Signed ceremony unchanged

- GIVEN a client that signs `mjolnir-name-claim:v1\n<challenge>\n<name>\n<port>` exactly as before
- WHEN it adds or omits the `app` field
- THEN the same signature verifies in both cases
