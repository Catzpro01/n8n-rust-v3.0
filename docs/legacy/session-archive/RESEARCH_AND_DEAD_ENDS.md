# Research, Diagnostics, and Approaches Not to Repeat

## External sources used for Ticket 07

- WHATWG Server-Sent Events, Last-Event-ID: https://html.spec.whatwg.org/multipage/server-sent-events.html#the-last-event-id-header — reconnect reports the last event ID; server replay/gap behavior must be explicit.
- MDN EventSource: https://developer.mozilla.org/en-US/docs/Web/API/EventSource — HTTP/1 browser/domain connection limits support explicit global subscriber bounds.
- IETF Idempotency-Key draft 07: https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-idempotency-key-header-07 — unique keys, payload fingerprinting, and reuse conflicts support safe POST retry behavior.
- SQLite WAL: https://www.sqlite.org/wal.html — WAL commits and reader/writer behavior require deliberate checkpoint management.
- SQLite synchronous pragma: https://www.sqlite.org/pragma.html#pragma_synchronous — WAL plus `synchronous=FULL` adds the required commit synchronization for power-loss durability.
- BLAKE3 crate API metadata: https://crates.io/api/v1/crates/blake3 — 1.8.7 was the current stable release checked and exact-pinned.
- cargo-audit: https://crates.io/crates/cargo-audit — 0.22.2 supports the current advisory format but requires Rust 1.88 for the audit tool itself.
- cargo-audit compatibility: https://lib.rs/crates/cargo-audit — 0.21.2 supports the project compiler but cannot parse current CVSS 4.0 advisories.

## Browser-startup diagnosis

The daemon was healthy when started manually. Six-second browser readiness windows failed when SQLite startup competed with VPS disk activity, including another user's `apt/dpkg`. Readiness windows in `two-tab.mjs`, `publish-rollback.mjs`, and `run-trace.mjs` were increased to 20 seconds.

An independent `run-trace.mjs` race then clicked the old Run's visible cancel control before the new Run ID appeared. The test now waits until the Run ID changes before cancelling.

Do not interpret those historical failures as a production deadlock.

## Errors and dead ends

- Running the cancellation test without its full module path selected zero tests. Correct path: `run::tests::artifact_preparation_failure_cannot_strand_a_requested_cancellation`.
- The cancellation fixture needs parent execution-plan and related rows before inserting a Run.
- `CapabilityIdentity.capabilities` is correctly length 24; do not restore the earlier length 20.
- Do not reinstall Playwright Chromium; it is already installed on the VPS.
- Browser CSP testing already uses explicit waiting and Playwright `bypassCSP: true` where required.
- `cargo-audit 0.21.2` is unusable with the current advisory DB. Use 0.22.2 through the isolated Rust 1.88 toolchain.
- `pkill -f` with the daemon's full command matched and killed the invoking SSH shell once.
- `kill -0` as the unprivileged user cannot reliably test a root-owned process because `EPERM` appears as failure; checking `/proc/<pid>` was reliable.
- `tail --pid=... -f /dev/null` returned immediately on this host and did not wait as expected.
- Do not terminate another VPS user's package installation; shared host activity must be allowed to finish.
- The first safe old-key removal attempt found zero exact public-key matches. No unrelated line was removed. A direct old-key login test later proved the old key was rejected.
- One final deployment verification initially queried `.build.commit`; the release manifest field is `.buildCommit`. The corrected verification passed.
- Workspace snapshots repeatedly restored the replacement key to mode `0644`, causing OpenSSH to reject it. Always `chmod 600` in the same shell immediately before SSH.

## Invariants that must not regress

- Never reopen frozen Ticket 07 decisions without Owner request.
- Never expose Artifact filesystem paths, cross-owner addresses/equality, wrapped keys, nonces, or dedup identities.
- Never count deduplicated storage as logical output.
- Never delete young, suspect, active-lease, pinned, or referenced content to relieve disk pressure.
- Queue saturation is backpressure, not failure.
- Hard-invalid configuration/overflow is Permanent Failure, not indefinite suspension.
- Temporary unsafe storage pressure is Durable Suspension, not destructive cleanup.
- Integrity failure quarantines evidence and fails without releasing unverified plaintext.
