# Issue 01: Crate Dependencies & Cargo Workspace Setup

Type: feature
Status: ready
Blocked by: none

## Scope
Add the approved native Rust crates to Cargo.toml without triggering C/C++ build bloat. Ensure all crates compile with Rust 1.85.1 and pure rustls.

## Crates to add
1. spider (crawling framework with chrome feature)
2. scraper (CSS selectors)
3. chromiumoxide (Tokio CDP client)
4. polars (data manipulation, features: lazy, csv, json)
5. eqwest-middleware, eqwest-retry, ackoff
6. governor (rate limiting)
7. moka (caching)
8. 	okio-cron-scheduler (cron triggers)

## Acceptance Criteria
- cargo check --workspace passes cleanly on Linux VPS (2-core).
- All dependencies use pure Rust TLS (ustls).