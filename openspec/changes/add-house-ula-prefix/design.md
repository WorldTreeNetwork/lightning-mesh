# add-house-ula-prefix

## Inputs

The house guild id is the raw 32-byte keyspace content hash. The file `/etc/mjolnir/house-guild-id` stores it as 64 hex characters. Base58 is a display form and is not hashed, so a display change cannot renumber the house.

The domain string is `mjolnir/house/ula/v1`. It is not `ULA_ID_INPUT` (`mjolnir/mesh/ula/v0`) and it is not the CRDT gossip topic. Bumping gossip must not change these addresses.

## Derivation

Hash inputs are raw bytes. The domain is the 20 ASCII bytes `mjolnir/house/ula/v1` with no trailing NUL. The guild id is the 32 raw bytes. The hash input is those 52 bytes concatenated, with no separator.

`/48`: `fd` plus the first 40 bits of that digest (the first five digest bytes), placed in network byte order immediately after `fd`.

Subnet id is a 16-bit integer in network byte order in the next two bytes. Router subnet id is 0. App subnet id is 1.

Router interface id: `blake3` of the node id's UTF-8 bytes, the same string `secret.public().to_string()` already feeds `ula_addr`. Not the raw key bytes. First 8 digest bytes, network byte order. The value 0 clamps to 1.

App interface id: `blake3` of the raw 32-byte Ed25519 public key. Not hex, not base58. First 8 digest bytes, network byte order. The value 0 clamps to 1.

A 40-bit prefix and a 64-bit interface id separate typical inputs and can still collide. This change does not detect or reallocate collisions.

The legacy address stays `ula_addr(node_id)` at prefix length `/64`, byte-identical to today.

Fixture, guild `11` repeated to 32 bytes, node id UTF-8 `node-example`, app key `ab` repeated to 32 bytes:

| Address | Value |
|---|---|
| House A `/48` | `fd81:2b22:8aca::/48` |
| Router A | `fd81:2b22:8aca::ddcd:86a6:64e9:f2df` |
| App A | `fd81:2b22:8aca:1:bd5e:fa7a:5460:697` |
| House B `/48` (guild `22` × 32) | `fd8f:78a9:2caa::/48` |
| Router B, same node id | `fd8f:78a9:2caa::ddcd:86a6:64e9:f2df` |
| Legacy for `node-example` | `fd53:6213:4797::ddcd:86a6:64e9:f2df` |

## Assignment

The guild file is read when the daemon starts and again on the same reconcile pass that restores the legacy ULA after an interface reload. There is no file watcher.

Parsing: trim ASCII whitespace, accept upper or lower hex, require exactly 64 hex digits. Anything else is malformed, including a display-form base58 guild id.

Ownership that survives restart is one line in `/var/lib/mjolnir/house-ula`: the IPv6 text of the last house `/128` this daemon observed as usable. The file is the only address the daemon may delete. It never deletes an address that file does not name, so a missing state file means "delete nothing." The legacy `/64`, a stock `ula_prefix`, and any other IPv6 address stay.

Reconcile order:

1. Read the state file. A missing or unparseable file means there is no owned address.
2. Compute the desired `/128` from a valid guild file, or none.
3. If an owned address exists and it is not the desired address, delete that address from `br-mesh` and no other.
4. If a desired address exists and is not usable, try to add it at prefix length 128.
5. Observe `br-mesh` again.

Usable means the desired address is present at prefix length 128 and is neither tentative nor DAD-failed. An add that returns an error while that usable address is already present still counts as assigned. An add that returns an error and leaves no usable address does not. Tentative and DAD-failed entries are not published and are not written to the state file.

After that observation, if the desired address is usable, write it to the state file. If there is no desired address, remove the state file. If the add failed, leave the state file unchanged so the next pass can retry, and do not record the failed address.

`house_ula` is the usable address on `br-mesh`, or omitted. It is not the desired address and it is not the state file.

Interface recreation: the same reconcile restores the desired `/128` while the guild file is valid. A restart with a changed or removed guild file uses the state file to find the old `/128` and removes only that.

Neither address goes on `mjolnir0` or `br-lan`, and neither enters the Iroh bind set. A house `/128` is not a reachability claim. Cross-site forwarding is a later change.

## Cutover

Both addresses exist until `add-ula-node-reference` is what operators use. Dropping the compiled prefix is not this change.

## Unverified hypotheses

None. The derivation is a pure function. Cross-site reachability of the `/128` is spiked on `add-ula-host-forwarding` and is not a premise of this change.
