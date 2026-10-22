# Design: app marker on every publish path

## Why unsigned, for now

`NameClaimRequest` already carries two self-reported, unsigned fields:
- `ip`, which is node-vouched
- `scheme`, which projects to a clickable protocol

Both stay outside `mjolnir-name-claim:v1\n<challenge>\n<name>\n<port>`
(`crates/mjolnir-hello/src/routes.rs:144–203`), so the shipped signing ceremony
and every existing client keep working. The app marker follows the same
pattern.

**[AUTO] decision:** an unsigned `app` field in v1 is acceptable because the
marker only changes *presentation*:
- The browser contract derives the entry origin from the record's
  name/protocol/port, never from the marker.
- Cards embed only on non-reserved `.mesh` name hosts.
- `path` is validated at publish time and again by every consumer.

An ingesting node that rewrites the marker can at most hide an app or change its
entry path within the same origin. That's the same trust `scheme` has today.
When owner-signed name records land (`ai0.9` follow-on, needed by
`add-https-aliases`), `app` moves inside the signed record. This change must not
add a second, incompatible signing scheme.

## Shape

- **Request:**
  ```json
  {"pubkey":"…","sig":"…","challenge":"…","name":"walkie-talkie","port":443,"scheme":"https","app":{"v":1,"path":"/"}}
  ```
- **Spool record and leased-name CRDT record:** optional
  `app: Option<AppMarker { v: u8, path: String }>`, with `#[serde(default)]`
  so older records parse. Wire `{"app":{"v":1}}` (omitted path) stores
  `path: "/"`. Directory omits `txt.path` when the stored path is `/`.
- **Directory projection:** `app` becomes `txt.app = "v1"`, plus `txt.path`
  when the path isn't `/`.
- **Control API:** the marker is present iff TXT `app` is present. Then
  `app` must be `v1` byte-exact and `path`, if present, is validated by
  the shared function. Without `app`, `path` and all other TXT keys are
  opaque (plain `publish --txt path=print` still works). The CLI
  `--app` and `--app-path` flags set the marker; `--app-path` alone is
  invalid.

## Validation (one function, shared vectors)

It's the same rules as `contract.ts` `appPath`/`isMiniApp` (no extra
publish-time length ceiling — living consumer has none), and they
apply **only when the marker is present** (`app` object on a name
claim, or TXT `app` on the control API):
- `v == 1` / TXT `app=v1` byte-exact
- omitted `path` canonicalizes to `/`
- `path` starts with `/`
- `path` contains no `//`, `\`, scheme, or control characters
- CLI `--app-path` is invalid without `--app`
- without `app`, do not run path validation

It's implemented once in `mjolnir-mesh` as a library function, used by
mjolnir-hello and meshd, and tested against JSON fixtures shared with
`hello-mesh-web/src/lib/miniapp/fixtures/`. A new `app-marker/` fixture set sits
beside `manifests/`.

## Rejection and leases

A claim with an invalid marker is rejected **before** spooling, so an invalid
update can't overwrite a valid prior lease. A valid claim without `app` clears
the marker on renewal: absence means "not an app". Publishers keep the marker by
sending it on every renewal.
