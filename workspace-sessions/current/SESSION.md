# Current session handoff

**Started:** 2026-09-15
**Branch:** `arena/01a0a352-n8n-rust-v3-0`
**Previous handoff:** `workspace-sessions/previous/SESSION.md`
**Active map:** `.scratch/universal-scraper-engine/map.md`
**Active ticket:** Issue 03, core engine implementation in `workflowd`

## Owner direction

- Implement the Universal Scraper effort in the recorded order: Issue 01
  dependencies, Issue 02 contract, Issue 03 engine, Issue 04 acceptance/CI.
- Keep the pinned MSRV 1.85.1 stack; do not re-add crates that were dropped for
  MSRV, and do not weaken the `--locked` gate.
- Unlock mode 4 (`DataTransform`) only through the recorded sequence: `csv` in
  `crates/workflowd/Cargo.toml`, `cargo check --workspace`, commit the
  regenerated `Cargo.lock`, then flip the contract gate.

## Current state

- Issue 01 is complete and Issue 02's contract landed CI-green through PR #16.
- Issue 03 is implemented as `crates/workflowd/src/universal_scraper.rs` and
  declared in `crates/workflowd/src/main.rs`. Modes 1 (`FastHttp`) and 2
  (`DeepCrawl`) execute; modes 3 and 4 are refused by the contract gate and
  again by the engine.
- Operational description: `docs/operations/universal-scraper.md`.
- The sandbox has no Cargo toolchain and cannot reach crates.io, so
  `cargo test -p workflowd --lib universal_scraper` runs in the pinned
  `rust-check` job rather than locally. Formatting was checked locally with
  rustfmt 1.8.0-stable, the formatter shipped with the pinned toolchain.

## Next action

- Confirm the pinned `rust-check` run is green for this branch and record the
  run id in the Issue 03 ticket.
- Then start Issue 04: native acceptance test plus removing the Playwright
  browser gate from CI.
- The mode-4 unlock still needs a cargo-enabled host to regenerate
  `Cargo.lock`; it stays gated until then.
