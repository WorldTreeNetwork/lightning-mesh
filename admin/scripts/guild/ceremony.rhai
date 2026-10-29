// Order for one workplace guild. The host signs; this file only sequences.
// Scope: mode, workplace_name, workplace_id, node_name, node_id.
// Host: log_lines, sign_op(verb, guild_id, node_id, counter),
//       save_genesis(...), save_assign(...).

let base = log_lines();

if mode == "genesis" {
    if base > 0 {
        throw "guild log already has ops; assign with the workplace keyspace id instead of a new genesis";
    }
    let admin = sign_op("join", "", "", 0);
    let work = sign_op("join", "", "", 1);
    sign_op("assign-node", work.keyspace_id, node_id, 2);
    let access = sign_op("nest", work.keyspace_id, "", 3);
    save_genesis(admin.keyspace_id, work.keyspace_id, access.keyspace_id, admin.actor, workplace_name, node_name, node_id);
    #{
        actor: admin.actor,
        admin_guild: admin.keyspace_id,
        workplace: workplace_name,
        workplace_guild: work.keyspace_id,
        node: node_name,
        node_id: node_id,
        access_node_guild: access.keyspace_id
    }
} else if mode == "assign" {
    if workplace_id == "" {
        throw "assign needs a workplace keyspace id";
    }
    if base == 0 {
        throw "assign needs an existing guild log; run genesis first";
    }
    sign_op("assign-node", workplace_id, node_id, base);
    let access = sign_op("nest", workplace_id, "", base + 1);
    save_assign(workplace_id, access.keyspace_id, access.actor, node_name, node_id);
    #{
        actor: access.actor,
        workplace_guild: workplace_id,
        node: node_name,
        node_id: node_id,
        access_node_guild: access.keyspace_id
    }
} else {
    throw "unknown ceremony mode";
}
