# Infrastructure Handoff: VPS 2-Core Elastic Balancer & Universal Rust Scraper

## 1. System Environment
- Host: ps-fern (203.145.35.218)
- Hardware: **2 vCPU Cores**, 3.8 GB RAM, 4.0 GB Swap file.
- Cgroup Priority: Loket 1â€“6 (Core RAM, Weight=100), Loket 7â€“8 (Swap buffer, Nice=10, CPUWeight=40, MemoryHigh=900M).
- Balancer: ps-elastic-balancer.service automatically balances up to 8 active runners across n8n and hermes.

## 2. Active Development Directive: Universal Scraper Node
- Heavy Node.js Playwright/Chromium dependencies are deprecated in favor of a single unified pure Rust scraping node.
- All AI workers should claim issues under .scratch/universal-scraper-engine/issues/.
- Maintain strict TDD: write unit tests against local mock servers before implementing node logic.
- Verify with cargo test -p workflowd before committing.