# Tasks

- [ ] Add explicit publisher and key-init CLI paths, validate the single-service config, and retain observer CLI compatibility. Test missing, corrupt, concurrent-init and restart cases.
- [ ] Implement the local TCP check and bounded challenge/claim loop. Test the canonical signing bytes against the router verifier, successful renewal, omitted/retained app markers, timeout after submission, backoff and shutdown.
- [ ] Exercise the real HTTP/spool/ingest contract in integration tests. Include a lost name conflict and a 200 response with no subsequent matching publication. Check human and machine-readable outcomes without exposing the signing secret.
- [ ] Provide Raspberry Pi Linux install and systemd instructions with a persistent state directory, unprivileged service account, explicit key initialization, and a stable address. Document start, stop, logs and independent observer checks.
- [ ] Correct the publishing guidance about omitted IP against the actual router behavior. Verify the documented config and commands on a clean installation.
- [ ] Record field evidence from AP3000, m3000 and the SSD-backed EdgeBox: observed owner, directory and DNS target, client response, stale-name fade and recovery, and repeat after Pi reboot. Mark hardware acceptance outstanding until actually exercised.
