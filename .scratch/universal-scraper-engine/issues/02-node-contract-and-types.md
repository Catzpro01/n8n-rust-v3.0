# Issue 02: Node Contract & Schema Definition

Type: specification
Status: pending
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