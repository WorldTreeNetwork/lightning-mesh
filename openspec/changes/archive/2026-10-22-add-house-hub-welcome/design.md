# House hub welcome

## Pins already made

- One initial owner delegates later. This change does not create that
  owner and does not treat a visitor identity as admin.
- Identity is optional for internet. The living captive-portal spec
  already says a dismissal does not change routing. This change must
  not add a pass button that pretends otherwise.
- hello.mesh Services is the surface. Mini-apps stay on their own change.
- Browser keys on plain HTTP are extractable and soft custody
  (`hello-mesh-web` identity notes). Do not store an owner seed in that
  browser, and do not move a private key through a link.

## Return URL

The 2026-10-21 review accepted this and answered the open question:
advertise both `http://hello.mesh/` and the node LAN gateway
(`10.42.<x>.1`). hello binds the LAN address. The QR encodes that
stable local URL and carries no per-visitor token. Existing access
logs stay out of this change. "Anonymous" here means the welcome
flow creates no identity, membership, or capability. It does not
add a new logging policy.
