# Universal Scraper Engine (Pure Rust) â€” Effort Map

## Summary
Transition web scraping, crawling, and headless browser automation from heavy Node.js Playwright dependencies to an all-in-one, 100% native Rust node.

## Frontier
- **Issue 01:** [01-crate-dependencies.md](issues/01-crate-dependencies.md) â€” Status: in-progress (deps inert until a member crate uses them; 3 crates missing).
- **Issue 02:** [02-node-contract-and-types.md](issues/02-node-contract-and-types.md) â€” Status: partially unblocked - mode 1 only; modes 2/3/4 need spider, chromiumoxide, polars.
- **Issue 03:** [03-core-engine-implementation.md](issues/03-core-engine-implementation.md) â€” Status: pending (blocked by 02).
- **Issue 04:** [04-acceptance-test-and-ci.md](issues/04-acceptance-test-and-ci.md) â€” Status: pending (blocked by 03).

## Architecture Stack
- Crawling: spider-rs/spider (pure rustls mode)
- HTML Parsing: scraper (CSS selectors)
- HTTP & Resilience: reqwest + reqwest-middleware + reqwest-retry + backoff (pure rustls)
- Headless Browser: chromiumoxide (CDP native async)
- Data Processing: serde + serde_json + polars + csv
- Rate Limiting: governor
- Caching & Cron: moka + tokio-cron-scheduler
- Async Runtime: tokio