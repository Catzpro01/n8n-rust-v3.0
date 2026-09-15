# Universal Scraper Engine (Pure Rust) — Effort Map

## Summary
Transition web scraping, crawling, and headless browser automation from heavy Node.js Playwright dependencies to an all-in-one, 100% native Rust node.

## Frontier
- **Issue 01:** [01-crate-dependencies.md](issues/01-crate-dependencies.md) — Status: completed.
- **Issue 02:** [02-node-contract-and-types.md](issues/02-node-contract-and-types.md) — Status: contract landed and CI-green; modes 3 and 4 gated.
- **Issue 03:** [03-core-engine-implementation.md](issues/03-core-engine-implementation.md) — Status: implemented in `crates/workflowd/src/universal_scraper.rs`; awaiting the pinned `rust-check` run.
- **Issue 04:** [04-acceptance-test-and-ci.md](issues/04-acceptance-test-and-ci.md) — Status: ready (unblocked by 03).

## Architecture Stack (Verified MSRV 1.85.1)
- HTTP & Resilience: `reqwest` (=0.12.12, pure `rustls-tls`) + `reqwest-middleware` (=0.4.1) + `reqwest-retry` (=0.7.0) + `backoff` (=0.4.0)
- HTML Parsing: `scraper` (=0.23.1)
- Rate Limiting: `governor` (=0.10.4)
- In-Memory Caching & Scheduling: `moka` (=0.12.16, `future`) + `tokio-cron-scheduler` (=0.15.1)
- Headless Browser (Mode 3): `chromiumoxide` (=0.9.1, CDP pure rustls)
- Data Processing (Mode 4): `csv` (=1.3.1) + `serde_json`
