# Issue 04: Pure Rust Acceptance Test & CI Integration

Type: verification
Status: pending
Blocked by: 03

## Scope
Create end-to-end acceptance tests verifying the Universal Scraper Node and replace the flaky Playwright browser acceptance in CI.

## Acceptance Criteria
- Native acceptance test boots local mock HTTP server and validates scraping, rate limiting, and polars CSV output.
- .github/workflows/validate.yml updated to run the native Rust test suite.
- Eliminates Node.js Playwright dependencies from CI gate.