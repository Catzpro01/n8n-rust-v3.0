# Universal Scraper engine

**Status:** core engine implemented in `crates/workflowd/src/universal_scraper.rs`
for the two contract modes that have a locked dependency stack; the run dispatch
and the native acceptance suite are Issue 04.

`8n-nodes-base.universalScraper` executes in a single pass: a response body is
decoded in RAM, parsed by `scraper`, and dropped before the extracted rows leave
the parse function. The module opens no file handles and owns no temporary
paths, so `ScraperSummary::disk_spills` is always zero and no page body is ever
serialized to disk.

## Modes

| Mode | Engine behaviour |
| --- | --- |
| `FastHttp` | Fetch each configured seed once, extract rows from each page. |
| `DeepCrawl` | Breadth-first walk of the seed host, bounded by depth, page budget and link harvest. |
| `BrowserHeadless` | Refused. The workspace has no CDP client. |
| `DataTransform` | Refused. No `csv` consumer is locked in `Cargo.lock`. |

Modes 3 and 4 are refused in two places on purpose: `canopy-node-contract`
rejects them at contract validation, and the engine rejects them again before
any request is issued, so a contract that bypasses the validator still cannot
reach a missing stack.

## Settings

Settings are read from `configuration.defaults`, after the contract gate has run.
Every bound is explicit and every value must sit inside a hard ceiling; an
out-of-range value is a configuration failure rather than a silent clamp.

| Key | Default | Ceiling |
| --- | --- | --- |
| `row_selector` | `tr` | valid CSS selector |
| `fields` | one `value` column | 32 rules |
| `requests_per_second` | 8 | 1000 |
| `burst` | 4 | 64, and never above `requests_per_second` |
| `timeout_seconds` | 20 | 120 |
| `max_retries` | 3 | 10 |
| `initial_retry_millis` | 250 | 60000 |
| `max_retry_millis` | 8000 | 600000, never below `initial_retry_millis` |
| `max_depth` | 2 | 4 |
| `max_pages` | 64 | 256 |
| `max_rows` | 10000 | 50000 |
| `crawl_same_host_only` | `true` | boolean |

Row, row-byte, page-byte and output-byte budgets default to the hard ceilings
(`MAX_ROW_BYTES`, `MAX_PAGE_BYTES`, `MAX_LOGICAL_OUTPUT_BYTES`).

## Bounded concurrency

`bounded_concurrency` caps in-flight requests by the cgroup v2 CPU quota that
`crate::cgroup` already discovers at startup, then by `MAX_CONCURRENCY` (8). A
fractional quota still gets one request in flight, so a throttled container
degrades instead of stalling. Each wave is additionally capped by the remaining
page budget, so `max_pages` is a real bound rather than an approximate one.

Every request also passes a `governor` token bucket sized by
`requests_per_second` and `burst` before it is sent.

## Retry policy

`fetch_with_retry` classifies each answer:

- `2xx`/`3xx`: accepted;
- `429` and `5xx`: retried on the exponential schedule from the pinned `backoff`
  crate, honouring `Retry-After` when the origin asks for longer (capped at
  `MAX_RETRY_AFTER_SECONDS`);
- any other `4xx`: permanent `canopy.universal-scraper.page_rejected`;
- transport faults (connect, timeout, reset): retried on the same schedule;
- exhausted budget: permanent
  `canopy.universal-scraper.retry_budget_exhausted`.

Every attempt is counted in `ScraperSummary::retry_attempts`, so a run report
shows how much resilience a scrape actually needed.

## Determinism and evidence

Pages are parsed in request order regardless of the order in which tasks
complete, rows are numbered in that order, and each row extends the
`canopy.scraped-row-chain/v1alpha1` digest chain. Replaying the same pages
therefore reproduces `stream_digest` and `correctness_digest` exactly. The
correctness digest follows the existing
`canopy.correctness-digest/v1alpha1` shape and records mode, pages fetched,
retry attempts, row count, logical bytes and the stream digest.

## Failure codes

All codes live in the contract's error namespace:
`canopy.universal-scraper.invalid_configuration`, `.unsupported_mode`,
`.invalid_selector`, `.invalid_seed`, `.transport_failed`, `.page_rejected`,
`.retry_budget_exhausted`, `.page_bytes_exceeded`, `.row_bytes_exceeded`,
`.output_rows_exceeded`, `.output_bytes_exceeded`, `.runtime_failed` and
`.digest_failed`.

## Verification

`cargo test -p workflowd --lib universal_scraper` covers the engine against a
scripted transport, so retry, crawl, budget, rate-limit and replay behaviour are
observed without a network:

- tabular rows, ordinals and a zero disk-spill record for `FastHttp`;
- 429, 5xx and connection-reset retries on the backoff schedule;
- permanent 4xx and permanent exhausted-budget failures;
- depth, host and page-budget crawl boundaries;
- in-flight requests never exceeding the cgroup ceiling;
- token-bucket spacing of requests;
- refusal of unlocked modes, unknown modes, missing `rows` port, invalid
  selectors and non-http seeds;
- row and page byte budgets as permanent failures;
- identical digests when the same pages are replayed;
- settings parsed from `configuration.defaults`, with out-of-range values
  refused.
