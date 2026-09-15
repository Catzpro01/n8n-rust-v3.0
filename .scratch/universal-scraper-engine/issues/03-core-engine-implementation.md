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

Unrelated failures in the same runs, for the record: `browser-suite` failed on
`two-tab editing` with "expected Draft Version 5; got 4" in run `34945219044`
after passing in `34935542196`, and `validate` failed on the
`generate-progress.mobile.png` geometry gate (actual 340x546 against baseline
340x545). This branch changes no file under `editor/` or `contracts/`.
