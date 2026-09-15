## ADDED Requirements

### Requirement: Apps shelf on hello.mesh

hello.mesh SHALL present mini-apps in an Apps shelf on the front desk,
above the Services list. The shelf SHALL read `GET /api/apps` and the
directory services already polled for the page. A service SHALL appear as
an app tile only when `isMiniApp` is true. Tile name, description, icon
and embed mode SHALL come from the `/api/apps` record when present, else
from the service name and `embed: link`. Manifest strings SHALL render as
text (or `<img>` for an inlined icon), never as markup.

A card-mode tile that `canEmbed` SHALL create no iframe until the visitor
opens it. After open, insertion SHALL follow Sandboxed insertion. Every
tile SHALL keep an open-in-new-tab control. A missing, stale or invalid
manifest SHALL still show a link-out tile. An empty directory SHALL show
an empty Apps state, not Services' empty copy. Apps SHALL NOT contact app
hosts from the browser except the entry URL of an opened card.

#### Scenario: Card tile opens on tap

- GIVEN `/api/apps` has a valid card-mode mini-app whose entry origin is not key-bearing
- WHEN the visitor opens its tile
- THEN a sandboxed iframe loads the entry URL and an open-in-new-tab control is visible

#### Scenario: Failed manifest is still a tile

- GIVEN a mini-app whose `/api/apps` record has no manifest
- WHEN Apps renders
- THEN a link-out tile using the service name is shown and no iframe is created

#### Scenario: Page load does not fetch app hosts

- GIVEN three mini-apps in the directory
- WHEN a visitor loads hello.mesh
- THEN the page's network requests go only to a hello.mesh origin until a card is opened

#### Scenario: Unmarked services stay in Services

- GIVEN a web service with no `app=v1`
- WHEN hello.mesh renders
- THEN it appears in Services and not in Apps
