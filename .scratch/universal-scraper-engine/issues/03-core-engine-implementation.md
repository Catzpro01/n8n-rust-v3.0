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