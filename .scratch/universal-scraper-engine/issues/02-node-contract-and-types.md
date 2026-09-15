# Issue 02: Node Contract & Schema Definition

Type: specification
Status: partially complete - contract landed and CI-green; modes 3 and 4 gated
Blocked by: 01

## Scope
Define normative contract and configuration lock in crates/node-contract for
8n-nodes-base.universalScraper.

## Modes
1. FastHttp: Direct async GET/POST with selector extraction (ms latency).
2. DeepCrawl: Domain spider with depth limit, concurrency limiter, and link harvester.
3. BrowserHeadless: CDP-driven page evaluation, form submit, screenshot, and JS execution.
4. DataTransform: Polars DataFrame filtering, column mapping, and CSV/JSON output.

## Acceptance Criteria
- Contract lock adheres to NORMATIVE_FIELDS in crates/node-contract.
- Input and output ports strictly validated.
## Implementation note (2026-09-15)

`crates/node-contract/src/universal_scraper.rs` implements this contract and is
green in CI: `rust-check` passed on run 34917759414 with 11 tests in
`canopy-node-contract`.

The mode lives in `configuration.defaults.mode`, not `configuration.mode`,
because the crate-root validator restricts `configuration` to `defaults`,
`editor_hints` and `schema`.

Two of the four modes are refused by the gate with `UnlockedStack`:
`BrowserHeadless` has no CDP client in the workspace and `DataTransform` has no
`csv` in `[workspace.dependencies]`. `spider`, `chromiumoxide` and `polars`
were dropped for MSRV 1.85.1. Modes 1 and 2 pass on the locked
reqwest/scraper/governor/backoff set.

Closing this ticket needs a decision on modes 3 and 4: add a CDP client and
`csv`, or revise this ticket to two modes.
