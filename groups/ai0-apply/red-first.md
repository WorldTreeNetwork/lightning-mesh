# mjolnir-apply red-first record

Scope: laptop-only transaction engine. These cases were named before the implementation and initially had no `mjolnir-apply` crate or test target, so the focused Cargo command failed. They are now executable Rust tests in `crates/mjolnir-apply/src/tests.rs`.

| Red case | Executable proof |
|---|---|
| stale revision | `stale_revision_refuses_before_mutation` |
| concurrent apply | `concurrent_apply_is_refused_by_process_lock` |
| duplicate ID, same-plan replay, different-plan conflict | `duplicate_id_is_idempotent_but_conflicting_plan_is_refused` |
| crash at every journal transition | `crash_after_every_nonterminal_transition_recovers_snapshot`, `crash_after_committed_transition_preserves_new_configuration`, `crashes_during_restoring_and_restored_are_restart_safe`, `recovery_required_transition_remains_blocked_after_a_crash` |
| health passes, crash before commit | `crash_after_health_pass_but_before_commit_restores` |
| partial snapshot | `partial_snapshot_and_unavailable_storage_never_mutate` |
| unavailable/unwritable transaction storage | `partial_snapshot_and_unavailable_storage_never_mutate` |
| corrupt journal | `corrupt_journal_blocks_recovery_and_new_admission` |
| reboot (`boot_id` changes) | `boot_id_change_uses_reboot_recovery_not_wall_clock_expiry` |
| missing backup | `missing_backup_and_failed_verification_require_recovery` |
| restoration verification failure | `missing_backup_and_failed_verification_require_recovery` |
| local-only false-positive internet evidence | `local_only_probe_cannot_be_recorded_as_client_internet` |
| exact UCI allowlist and missing-file marker | `only_allowlisted_files_are_snapshotted_and_missing_files_restore_as_absent` |

The fixtures inject `TxnPaths`, monotonic time, `boot_id`, and one-shot crash points. They do not invoke an OpenWrt adapter, init system, launcher, radio, or fleet node. Passing them is software fault-injection evidence only, not power-loss or hardware qualification.
