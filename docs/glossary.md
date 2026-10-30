# Guild glossary

**Status:** words as Lightning Admin and `lightning-guild` use them | **Date:** 2026-10-30

The product word is **guild**. The signed record calls the same thing a **keyspace**. One log file holds every guild. Replaying it builds each guild's current state: who is on the roster, which routers are assigned, and which child guilds exist.

Names such as Admin and Strawberry Creek live in `guilds.json` on this computer. The signed lines have no name field. On this machine the log is `~/.config/lightning-admin/guild-ops.log`. The routers do not hold a copy. Wraps are empty, so no guild secret exists yet.

Radio, overlay addresses, and `.mesh` names are a different vocabulary. See [How the house mesh works](../docs-web/content/join/house/03-how-it-works.md).

## Entries

### Absent

Your public key is not on that guild's roster. The role chip says absent.

### Access guild

The child guild created for one router. Assigning ap3000-outdoor-b to Strawberry Creek creates one. Its profile in the signed record is `access-node`. Its id is the hash of the nest that created it. The screen lists it only from that node's **Open access guild** button.

A house roster is the people who run the group. The access guild is where a holder would later wrap a key for someone admitted at that router. hello.mesh looks for an access-node keyspace id and, when one is present, can ask the holder to wrap a login key. Joining the client Wi-Fi does not consult this guild.

### Actor

The public key of whoever signed. A person, or this computer. Roster entries and log lines name actors. The private half stays in `actor-seed` and is never written into an op.

### Admin

Your key is on that guild's roster and the entry says `admin: true`. The role chip says admin. An admin is the holder who would later wrap the guild secret for someone else.

### Assign node

Record that a router belongs to a guild. The signed kind is `keyspace.assign-node`. The line names the guild and the node id. It does not create a guild. In the ceremony it is followed by a nest, which creates that router's access guild.

### Ceremony

The fixed signing order in `admin/scripts/guild/ceremony.rhai`. Genesis writes four lines. A later assign writes two: assign-node, then nest.

### Change

Replace a guild's roster, profile, or epoch. The signed kind is `keyspace.change`. The current screen has no control for it.

### Client SSID

The Wi-Fi name phones join (`Lightning Mesh`, or a house rename). Signing a guild does not write it.

### Display name

The nickname in `guilds.json`: Admin, Strawberry Creek, ap3000-outdoor-b. The access guild has no nickname, so the screen says access-node.

### Epoch

The generation number of the guild secret. These guilds are still at 0. A login wrap asks the holder to merge a recipient at the current epoch. It does not rotate the secret.

### Genesis

The first ceremony, and only while the log is empty. It creates the Admin house, creates the workplace house, assigns the first router, and nests that router's access guild.

### Guild

A membership group: who is in it, who is an admin, and which routers belong to it. The screen's guild list shows houses. Child access guilds open from a node.

### Guild id

The short id on the Id drill-in. It is the guild's 32-byte content hash, written in base58. For a house created by join, and for an access guild created by nest, that hash is the hash of the creating op.

### Hex

Those same 32 bytes, written as 64 hex characters. The log and `guilds.json` store this form as the keyspace id. The screen keeps it behind the Id drill-in.

### Holder

The admin who can wrap the guild secret for someone else. With empty wraps, that secret has not been minted, so there is no holder duty to perform yet.

### House

A top-level guild. Profile in the signed record is `house`. Admin is a house. Strawberry Creek is a house. Access guilds are not houses.

### Join

Create a house. The signed kind is `keyspace.join`. A join with no keyspace id starts a new guild, and the new id is the hash of that op. The roster is the signer, marked admin.

### Keyspace

The protocol name for a guild. Wire kinds are `keyspace.join`, `keyspace.change`, `keyspace.assign-node`, and `keyspace.nest`.

### Log

The append-only file of signed ops, one hex line each. `guild-ops.log`. Every guild on this computer shares that one file. A guild's history is the lines that name it, in order.

### Member

Your key is on that guild's roster and the entry is not admin. The role chip says member.

### Nest

Create a child guild under a parent. The signed kind is `keyspace.nest`. The body names the parent as `outer`, sets the profile to `access-node`, and starts a roster. The child's id is the hash of this op. The same line is a line in the parent's history and the child's birth record.

### No seed

This computer has no `actor-seed`, so it has no signing key. The role chip says no seed. The public id on the identity card reads "no seed".

### Node

A router. In the guild screen, a row under Nodes.

### Node id

That router's own mesh identity, 64 hex characters. Read it from the node (`service mjolnir-meshd diag` or `GET /api/node`). Assign node stores this id. It is not a guild id.

### Op

One signed line in the log. The Ops drill-in lists the lines that belong to the guild on screen: a house shows its join, its assign-node lines, and the nests under it. An access guild shows the nest that created it, until a later line names that child.

### Open access guild

Show the access guild for that router. It changes which guild is on screen. It does not sign a line and it does not talk to the router. **In {parent}** selects the house again. The house row stays highlighted.

### Outer

The parent guild a nest was created under. Strawberry Creek is the outer of outdoor-b's access guild.

### Rename

Change a display name in `guilds.json` and reload. The log does not grow. There is no rename kind in the signed record.

### Roster

The member list. Each entry is an actor's public key, marked admin or not. The Roster drill-in shows a role chip and that public key.

### Workplace

A house marked `workplace: true` in `guilds.json`. That mark is local. The signed record has no workplace field. Strawberry Creek is the workplace on this computer. The screen prefers it when it picks a guild to show.

### Wrap

An encrypted copy of the guild secret, addressed to one member. The joins and the nest on this computer carry an empty wrap list. No pairwise transient key is minted. A wrap is the step that would later admit someone at a router. A spool file from hello.mesh asking for a wrap is a request to the holder. It is not membership and it does not contain the secret.

### XID

An identikey document id: the hash of an inception signing key. A guild id is a keyspace content hash. The two ids name different objects.

### You

This computer's actor. The identity card shows its public id, in base58. The role chip on a guild says what that key is on that roster.
