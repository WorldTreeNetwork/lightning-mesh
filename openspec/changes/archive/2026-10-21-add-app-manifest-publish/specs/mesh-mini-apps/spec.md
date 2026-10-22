## ADDED Requirements

### Requirement: App marker accepted on every publish path

The system SHALL accept the mini-app marker (version `1` and an optional entry
path) on every path that publishes a name. That covers key-owned name claims
(`POST /api/name-claim`, as an optional `app` object outside the signed message)
and the node control API (`POST /v0/publish` and the `mjolnir-meshd publish`
command). On the control API the marker is present if and only if the TXT
key `app` is present. When `app` is absent, `path` and every other TXT key
SHALL be opaque and untouched (plain services may publish `--txt path=print`
today). When `app` is present it SHALL be `v1` byte-exact, and `path`, if
present, SHALL pass the shared rule set. All paths SHALL validate the marker
with one shared rule set, the same as the living `App marker on service records`
requirement and `contract.ts` `appPath`:
- version equals `1`
- the path begins with `/`
- the path has no `//`, `\`, scheme, or control characters
- an omitted path is `/`

There is no separate publish-time length ceiling. An invalid marker SHALL be
rejected with an error before anything is spooled or published, and SHALL NOT
alter an existing lease or service. A published marker SHALL appear in the
directory as TXT `app=v1` and, unless the path is `/`, `path=<path>`.
`{"app":{"v":1}}` SHALL be accepted and SHALL project like path `/`.
`--app-path` SHALL be rejected unless `--app` is also set. Consumers SHALL
still re-validate the marker.

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

#### Scenario: Omitted path is slash

- GIVEN an app holding a valid key-owned lease
- WHEN it claims with `"app":{"v":1}` and no `path` field
- THEN the request is accepted and the directory lists TXT `app=v1` without `path=`

#### Scenario: Plain service with path TXT and no app still publishes

- GIVEN an operator on a router
- WHEN they run `mjolnir-meshd publish printer --port 631 --txt path=print` with no `app` TXT key
- THEN the publish succeeds and the directory lists `printer` with TXT `path=print` and no `app`
- AND the service is not a mini-app

#### Scenario: --app-path requires --app

- GIVEN an operator on a router
- WHEN they run `mjolnir-meshd publish foo --port 80 --app-path /x` without `--app`
- THEN the command is rejected and no service record changes

#### Scenario: Signed ceremony unchanged

- GIVEN a client that signs `mjolnir-name-claim:v1\n<challenge>\n<name>\n<port>` exactly as before
- WHEN it adds or omits the `app` field
- THEN the same signature verifies in both cases
