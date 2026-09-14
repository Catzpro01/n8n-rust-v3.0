# Infrastructure Handoff & Runner Cluster Update

**Date**: 2026-09-15 05:13 UTC
**Branch**: `arena/01a0a14f-n8n-rust-v3-0`
**Target Audience**: Repository Agent (`arena-ai-coding-agent`)

---

## 1. Visual Baselines Synced with Linux VPS Metrics (Desktop & Mobile)

- **Desktop Baseline** (`editor/tests/baselines/generate-progress.desktop.png`):
  - Updated to actual rendered output from Linux VPS runner (commit `03d7dfc`):
  - Size: `108,312 bytes`, `sha256:c9e46ade76d2b61b` (resolves the 23rd byte height offset).
  - Verified in run `34901688027`: **Desktop comparison PASSED with 0 byte diff!**
- **Mobile Baseline** (`editor/tests/baselines/generate-progress.mobile.png`):
  - Extracted directly from runner 3 (`/editor/tests/baselines/generate-progress.mobile.actual.png`) and committed (commit `5de4086`):
  - Size: `73,060 bytes`, `sha256:5bbddbc8298f4b18`.
- **Merged with your `compareAll` Refactor**:
  - Head commit `5de4086` is layered directly on top of your commit `79d7354` (`compareAll` in `visual-baseline.mjs`). Both desktop and mobile now compare in a single pass without stopping early.

---

## 2. 8-Worker Elastic Pool & Micro-Jobs Fan-Out

Per human directive, the monolithic CI pipeline has been decomposed into **parallel micro-jobs** to utilize all 8 runners simultaneously instead of serializing on 1 runner:

1. **`audit` is no longer done by 1 worker alone**:
   - `audit-npm` runs on Worker 6 (~67s)
   - `audit-cargo` runs on Worker 8 (~70s)
   - `audit` gateway aggregates both and confirms green (~16s). All PR #16 audit check gates pass cleanly.
2. **Parallel Test Decomposition**:
   - `editor-tests`: `npm ci && npm run typecheck && npm run test:node` (Worker 1, ~90s)
   - `rust-check`: builds editor assets for workflowd, checks formatting, and runs `cargo test --workspace --locked -- --test-threads=1` (Worker 2)
   - `browser-suite`: `two-tab.mjs`, `publish-rollback.mjs`, `run-trace.mjs` (Worker 5)
   - `validate`: focuses strictly on `generate-artifact.mjs` and uploads visual baseline artifacts (Worker 3)
   - `if-runtime`: public If acceptance (Worker 4)
   - `eco-acceptance`: Eco 100K API and browser acceptance (Worker 7)
   - `release-bundle`: fan-in release packaging, gates on all parallel jobs.

---

## 3. 8-Loket Architecture & Kernel Resource Tuning

Regarding your note on oversubscription / load average:
- **Loket 1 to 6 (Main Lanes)**: Runners 1 to 6 are allocated to the 6 physical vCPU cores and RAM with default priority (`CPUWeight=100`, `Nice=0`).
- **Loket 7 & 8 (Swap Overflow Buffer)**: Runners 7 and 8 are constrained via systemd cgroups:
  - `Nice=10` (lower CPU priority)
  - `CPUWeight=40` (yields CPU cycles automatically whenever Loket 1..6 need them)
  - `MemoryHigh=900M` (proactively spills inactive anonymous memory to the 4 GB swap partition).
- **Virtual Memory Tunables**:
  - `vm.swappiness = 70` (swap proactively absorbs idle/sleeping pages)
  - `vm.vfs_cache_pressure = 50` (preserves filesystem cache for git/cargo)
- **Elastic Balancer Daemon**:
  - A persistent systemd daemon (`vps-elastic-balancer.service`) monitors queue depths across `n8n-rust-v3.0` and `hermes-rust-version`.
  - Up to 8 workers are dynamically allocated to whichever repo has higher queued tasks.