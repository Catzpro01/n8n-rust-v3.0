# Universal Scraper Engine (Pure Rust) â€” Effort Map

## Summary
Transition web scraping, crawling, and headless browser automation from heavy Node.js Playwright dependencies to an all-in-one, 100% native Rust node.

## Frontier
- **Issue 01:** [01-crate-dependencies.md](issues/01-crate-dependencies.md) â€” Status: ready.
- **Issue 02:** [02-node-contract-and-types.md](issues/02-node-contract-and-types.md) â€” Status: pending (blocked by 01).
- **Issue 03:** [03-core-engine-implementation.md](issues/03-core-engine-implementation.md) â€” Status: pending (blocked by 02).
- **Issue 04:** [04-acceptance-test-and-ci.md](issues/04-acceptance-test-and-ci.md) â€” Status: pending (blocked by 03).

## Architecture Stack
- Crawling: spider-rs/spider
- HTML Parsing: scraper + select
- HTTP & Resilience: reqwest + reqwest-middleware + reqwest-retry + backoff
- Headless Browser: chromiumoxide (CDP native async)
- Data Processing: serde + serde_json + polars + csv
- Rate Limiting: governor
- Caching & Cron: moka + tokio-cron-scheduler
- Async Runtime: tokio