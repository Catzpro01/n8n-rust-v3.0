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
  `rust-check` job rather than locally. That job is green for this branch: run
  `34945219044` on commit `5f28b59` reports `success` for both "Check Rust
  formatting" and "Run Rust tests". Formatting was also checked locally with
  rustfmt 1.8.0-stable, the formatter shipped with the pinned toolchain.
- `if-runtime`, `eco-acceptance`, `editor-tests`, `audit`, `audit-npm` and
  `audit-cargo` are green in the same run. Two jobs are red for reasons outside
  this branch: `browser-suite` on the two-tab draft-version race ("expected
  Draft Version 5; got 4", green one run earlier) and `validate` on the
  `generate-progress.mobile.png` geometry gate (actual 340x546
  sha256:3d380bf896d42cb7 against baseline 340x545 sha256:5bbddbc8298f4b18,
  byte-identical across two runs and both times on `actions-runner-3`). This
  branch changes no file under `editor/` or `contracts/`.

## Next action

- Issue 03 is closed out: `rust-check` run `34945219044` is recorded in the
  ticket, the map and `PROGRESS.md`. PR #17 carries the branch.
- The owner decides what to do about the `validate` visual baseline: re-run it,
  fix the rendering environment on `actions-runner-3`, or approve the one-pixel
  change deliberately with `UPDATE_VISUAL_BASELINE=1`. Do not regenerate a
  baseline from inside a scraper change.
- Then start Issue 04: native acceptance test plus removing the Playwright
  browser gate from CI.
- The mode-4 unlock still needs a cargo-enabled host to regenerate
  `Cargo.lock`; it stays gated until then.
