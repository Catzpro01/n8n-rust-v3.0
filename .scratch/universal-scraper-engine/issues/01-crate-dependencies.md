# Issue 01: Crate Dependencies & Cargo Workspace Setup

Type: feature
Status: in-progress (see Verification correction below)
Blocked by: none

## Scope
Add the approved native Rust crates to Cargo.toml without triggering C/C++ build bloat. Ensure all crates compile with Rust 1.85.1 and pure rustls.

## Crates Verified & Added (Tested on VPS Toolchain)
1. ackoff = "=0.4.0"
2. governor = "=0.10.4"
3. moka = "=0.12.16"
4.
eqwest = { version = "=0.12.28", default-features = false, features = ["rustls"] }
5.
eqwest-middleware = "=0.5.2"
6.
eqwest-retry = "=0.9.1"
7. scraper = "=0.27.0"
8. 	okio-cron-scheduler = "=0.15.1"

## Acceptance Verification
- cargo check --workspace tested on VPS 2-core runner: passed cleanly with return code 0 (build target completed in 4m 06s).
- Zero new C/C++ dependencies introduced; all crates operate on pure Rust TLS (
ustls).
## Verification correction (2026-09-15, repository agent)

The note above records `cargo check --workspace` returning 0 in 4m 06s. That run
did not exercise these crates. Two facts, both checked against the tree:

- `86db97b` touched `Cargo.toml` only. `Cargo.lock` was not modified.
- `Cargo.lock` contains **zero** entries for `reqwest`, `scraper`, `governor`,
  `moka`, `backoff`, `reqwest-middleware`, `reqwest-retry` and
  `tokio-cron-scheduler`.

Entries under `[workspace.dependencies]` are inherited only when a member crate
declares `dep.workspace = true`, and no member did. Cargo never resolved,
downloaded or compiled any of them, so a return code of 0 says nothing about
this stack. This is Finding 1 of
[01-crate-dependencies-FINDINGS.md](01-crate-dependencies-FINDINGS.md).

Follow-up commit wires all eight into `crates/workflowd` with
`workspace = true`. The next `rust-check` run is the first real compile of this
stack; until it is green, issue 01 is not verified.

## Still missing from the ticket's own list

| # | crate | needed by |
| --- | --- | --- |
| 1 | `spider` (chrome feature) | issue 02 mode 2, DeepCrawl |
| 3 | `chromiumoxide` | issue 02 mode 3, BrowserHeadless |
| 4 | `polars` (lazy, csv, json) | issue 02 mode 4, DataTransform |

Three of issue 02's four modes have no dependency to build on. Only mode 1
(FastHttp) is currently supported. Pins for these three were deliberately not
guessed: see FINDINGS for why an unresolved `=` pin is a hard failure rather
than a downgrade.
