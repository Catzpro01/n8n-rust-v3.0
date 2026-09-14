# Ticket 07 Verification Evidence

## Final pre-commit full gate

`make test` passed after the final quarantine-barrier remediation.

- Editor install/typecheck/build: passed
- Node Contract Rust tests: 4/4
- `workflowd` Rust tests: 27/27
- Rust doc tests: passed
- Workspace debug build: passed
- Browser suites: 4/4
- Python public-seam acceptance: 13/13

Final debug-profile Eco output from the full gate:

```text
generate-suspension=passed durable_state=suspended revalidated_state=succeeded
generate-eco=passed items=49998 logical_bytes=2027698 backpressure_events=781 rss_baseline=29855744 rss_peak=33128448 rss_delta=3272704 queue_items=256 queue_bytes=4194304
generate-restart=passed durable_cursor_before=1664 terminal_count=49998 activations=2 unique_checkpoints=61
```

Browser evidence:

```text
two-tab-browser=passed encrypted-recovery=passed undo-reload-redo=passed
publication-browser=passed axe-serious=0 visual-reload-diff=0 mobile-overflow=0
run-browser=passed sse-gap-resync=passed axe-serious=0 reload-markup-diff=0 mobile-overflow=0
generate-ui=passed lazy-preview-bytes=4 axe-serious=0 desktop-visual=passed mobile-visual=passed mobile-overflow=0
```

## Focused safety gates

Passed regressions include:

- cancellation settlement after Artifact preparation failure;
- complete progressive checkpoint delta from durable cursor;
- exact resume suffix and digest preservation;
- bounded preview/range plaintext retention;
- bounded full-content HTTP streaming after a complete integrity pass;
- ciphertext damage quarantine before plaintext release;
- read/quarantine/cleanup shared mutation barrier;
- expired staging lease quarantine and safety age;
- live referenced object protection;
- interrupted streaming upload quarantine;
- hard budgets and signed overflow as permanent failure;
- secret-like field rejection from retained traces.

Strict Clippy passed:

```text
cargo clippy --workspace --all-targets --locked -- -D warnings
```

RustSec audit passed:

- Tool: `cargo-audit 0.22.2`
- Audit-only toolchain: Rust 1.88.0
- Advisory database: 1,243 advisories at verification time
- Locked dependencies scanned: 153
- Denied findings: 0

BLAKE3 evidence:

- Exact lock: `blake3 1.8.7`
- Direct consumer: `workflowd`
- Package license declaration: `CC0-1.0 OR Apache-2.0`
- Debug measurement: 33,554,432 combined keyed/unkeyed bytes in 36,662 microseconds, 872.83 MiB/s

## Clean release gate

A clean release directory was removed and rebuilt from commit `e3580231cbf6246a1aefaf802c8898da5ca82005`.

`make release-test` passed:

- editor typecheck/build and npm high-severity audit;
- frozen release build;
- release bundle inspection;
- release binary Manual Trigger public seam.

Release binary:

```text
binary_bytes=8126848
```

Release-profile Generate/Artifact acceptance passed all 3 tests:

```text
generate-suspension=passed durable_state=suspended revalidated_state=succeeded
generate-eco=passed items=49998 logical_bytes=2027698 backpressure_events=569 rss_baseline=19968000 rss_peak=23023616 rss_delta=3055616 queue_items=256 queue_bytes=4194304
generate-restart=passed durable_cursor_before=1024 terminal_count=49998 activations=2 unique_checkpoints=50
```

Systemd smoke passed:

```text
systemd-smoke=passed state-preserved=/var/lib/workflow-rust/workflow.sqlite3 rss_bytes=31072256 idle_cpu_cores=0.0000 hardening_exposure=1.6
```

Final clean install verification:

```text
deployment=passed
local=e3580231cbf6246a1aefaf802c8898da5ca82005
origin=e3580231cbf6246a1aefaf802c8898da5ca82005
manifest=e3580231cbf6246a1aefaf802c8898da5ca82005
systemd=e3580231cbf6246a1aefaf802c8898da5ca82005
service_state=active
```

## Review conclusion

Final `/code-review` outcome: no Critical, High, Medium, or blocking Low finding remained. The source was committed only after the final full gate passed. Release and systemd checks were then run from the clean commit before deployment verification.
