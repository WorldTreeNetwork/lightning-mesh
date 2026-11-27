# Gossip observer

Run `mjolnir-node --lan --peer "$AP_TICKET" listen` to watch mesh announcements. Use a full address ticket when discovery is unreliable. A 64-hex node ID also works when the endpoint can resolve the peer. The observer joins the gossip overlay and originates zero mesh application records.

## Stream events to an agent

```sh
mjolnir-node --json --lan --peer "$AP_TICKET" listen --timeout-secs 30 > events.ndjson
jq -c 'select(.event == "message") | .data.message.ServicePublishV2? // empty' events.ndjson
```

Read stdout one JSON object per line as it arrives. Each record has integer `version` (currently 1), local Unix observation time `time_ms`, an `event` string, and a `data` object. Diagnostics go to stderr. `RUST_LOG` controls diagnostics independently. Consumers should allow new fields, event names, and message variants.

| Event | Data and meaning |
| --- | --- |
| `identity` | Local `node_id` and reusable address `ticket`. Also the only stdout record from `mjolnir-node --lan --json id`. |
| `listening` | Local subscription started, with the number of distinct bootstrap IDs in `bootstrap_count`. |
| `neighbor_up`, `neighbor_down` | `peer` identifies a direct gossip neighbor that connected or disconnected. |
| `message` | `count` starts at 1 per invocation. `delivered_from` is the immediate forwarding peer. `message` contains the decoded protocol variant and its fields, including any owner claimed by the message. Treat peer-provided strings as data. |
| `decode_error`, `lagged` | An undecodable payload was skipped (with `delivered_from` and `error`), or the local receiver missed events. Collection continues. |
| `stopped` | Final decoded-message `count` and `reason` of `count`, `timeout`, or `interrupt`. |

Use `listen --count 1 --timeout-secs 30` to wait for the first decoded message within a listening window. The timeout starts at subscription and includes time spent waiting for peers. Either limit ends the capture successfully, including a timeout with zero messages. Check the final reason, count, and neighbor events when interpreting silence. An unexpected receive-stream closure or fatal error exits nonzero with a diagnostic on stderr. Gossip delivery is best effort and the stream starts with this subscription.
