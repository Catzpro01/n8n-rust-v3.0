# Issue 03: Core Engine Implementation in workflowd

Type: feature
Status: pending
Blocked by: 02

## Scope
Implement crates/workflowd/src/universal_scraper.rs providing unified single-pass execution.

## Key Properties
- Zero intermediate disk serialization: raw HTML is parsed in RAM and discarded before passing tabular output.
- Concurrency bounded to system cgroups (safe on 2 vCPU VPS).
- Resilient retry with exponential backoff on 429 / 5xx HTTP responses.

## Acceptance Criteria
- Module integrated into crates/workflowd/src/main.rs.
- cargo test -p workflowd --lib universal_scraper passes.
## Implementation (2026-09-15)

`crates/workflowd/src/universal_scraper.rs` implements the engine and is
declared in `crates/workflowd/src/main.rs`. `docs/operations/universal-scraper.md`
describes the modes, budgets, retry policy and evidence shape.

Key properties as built:

- **Zero intermediate disk serialization.** A body is decoded into a `String`,
  parsed by `scraper` inside `parse_document`, and dropped there; only rows and
  harvested links leave the function. The module opens no file handles, and
  `ScraperSummary::disk_spills` is a recorded `0` rather than an assumption.
- **Concurrency bounded to the cgroup.** `bounded_concurrency` caps in-flight
  requests by the cgroup v2 `quota_cores` already discovered by
  `crate::cgroup`, then by `MAX_CONCURRENCY` (8). A fractional quota still gets
  one request in flight. Each wave is also capped by the remaining page budget,
  and every request passes a `governor` token bucket.
- **Resilient retry.** `fetch_with_retry` retries 429/5xx and transport faults on
  the exponential schedule from the pinned `backoff` crate, honours
  `Retry-After`, and treats every other 4xx or an exhausted budget as a
  permanent typed failure. Attempts are counted in the run evidence.

Modes 3 and 4 are refused by the engine as well as by the contract gate, so an
unvalidated contract still cannot reach a stack that does not exist.

## Acceptance evidence

Verified by `rust-check` run `34945219044` on commit `5f28b59` (PR #17), whose
steps "Check Rust formatting" and "Run Rust tests" both report `success`:

- `cargo fmt --all -- --check`: no diff. Also reproduced locally with
  rustfmt 1.8.0-stable (4d91de4e48), the formatter shipped with the pinned
  1.85.1 toolchain.
- `cargo test --workspace --locked -- --test-threads=1`, which includes
  `cargo test -p workflowd --lib universal_scraper`: green. The sandbox has no
  Cargo toolchain and crates.io is unreachable from it, so this criterion can
  only be verified on the pinned runner.

Three defects had to be fixed on the way, each one found by that job:

| run | defect | fix |
| --- | --- | --- |
| `34931924759` | `E0599`: `next_backoff` not found, because `backoff` does not re-export its trait at the crate root | `7839795` imports `backoff::backoff::Backoff` |
| `34934598306` | `E0277`: `UniversalScraper` is not `Debug`, which `Result::unwrap_err` requires of the `Ok` type | `3eb7a93` matches on the constructor result |
| `34935542196` | `row_and_page_budgets_are_permanent_failures` saw `page_rejected` instead of `page_bytes_exceeded`: the first scenario consumed the only scripted 200 | `5f28b59` re-scripts the page |

Two jobs stay red on this branch for reasons the evidence puts outside it.

`browser-suite` fails differently each time and passed once with this code:
"expected Draft Version 5; got 4" on `vps-fern-worker-2` in run `34945219044`,
then "daemon did not start (still running when the 20s health budget expired)" on
`vps-fern-worker-5` in run `34947325495`, after `success` in run `34935542196`.

`validate` fails identically every time: the geometry gate reports
`generate-progress.mobile.png` actual 340x546 73391 bytes sha256:3d380bf896d42cb7
against baseline 340x545 73060 bytes sha256:5bbddbc8298f4b18 (the committed
baseline is indeed 340x545 / 73060 bytes). All four runs put that job on
`vps-fern-worker-3`, which reported `success` for `main` at 04:13 and `failure`
for this branch at 06:23, 08:25 and 08:31 - so it correlates with time, not with
the branch. Four checks support that: the branch diff has no file under `editor/`
or `contracts/`; the engine module is referenced exactly once, by
`mod universal_scraper;` in `main.rs`, and is dead code, so no HTTP response can
change; `build.rs` embeds only `editor/dist`, so the bundle inputs are identical;
and the identity card that renders `build_commit` is gated on `{!workflowId && ...}`
in `editor/src/main.tsx:459`, so not even the build SHA reaches this screenshot.

What could not be checked from the sandbox: the rendered PNG itself (log and
artifact storage is unreachable), and what changed on `vps-fern-worker-3` between
04:13 and 06:23. `workflow_dispatch` returns 403 for this token and
`gh run rerun` refuses both older runs ("cannot be rerun"), so the clean
main-versus-branch experiment has to be run by the owner.

### Addendum: the owner's dispatch experiment on `main` (run `34951125279`)

`workflow_dispatch` on `main` at `934c475` puts the same code on the same runner
and answers the branch question directly. It ran at 09:11 on `vps-fern-worker-3`
and `validate` failed - but earlier than the geometry gate:

```
editor/tests/generate-artifact.mjs:229  Error: daemon did not start
```

`ready()` polls `/health/live` 400 times at 50 ms, so the budget is 20 s. The
visual comparison was never reached, so the one-pixel question is still open.
`browser-suite` was green in that run on `vps-fern-worker-8`, and so were
`rust-check`, `editor-tests`, `if-runtime`, `eco-acceptance` and the three
audits.

The failure mode on `vps-fern-worker-3` changed during the morning, which the
ordered `validate` history shows:

| started | run | code | failure |
| --- | --- | --- | --- |
| 04:13 | `34926933956` | `main` | success |
| 06:23 | `34935542196` | `3eb7a93` | geometry 340x546 vs 340x545 |
| 08:25 | `34945219044` | `5f28b59` | geometry 340x546 vs 340x545 |
| 08:31 | `34947325495` | `fab5f4b` | geometry 340x546 vs 340x545 |
| 08:48 | `34948966146` | `c8d44ae` | daemon did not start |
| 09:11 | `34951125279` | `main` | daemon did not start |

`main` now fails `validate` the same way this branch does, which settles the
startup failures as host CPU starvation rather than a branch defect. It does not
settle the geometry failures: nothing has reached that gate on `main` since
04:13.

`if-runtime` failed in run `34948966146` and published no annotations, so its
reason cannot be read from here. The code path is
`tests/acceptance/test_if_runtime.py` importing `Daemon` from
`tests/acceptance/test_run_manual_trigger.py`, whose health budget is
`range(1200)` at `time.sleep(0.05)` - 60 s, not 8 - and whose failure path then
calls `self.process.wait(8)`; the `8` is the post-`terminate()` wait, so a daemon
that ignores SIGTERM raises `subprocess.TimeoutExpired` from that call and buries
the "daemon did not start (still running when the 60s health budget expired)"
assertion underneath it.
