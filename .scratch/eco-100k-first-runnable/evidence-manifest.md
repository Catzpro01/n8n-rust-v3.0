# Eco 100K Release Candidate 1 Evidence Manifest

**Certification Date:** 2026-09-16
**Commit:** 7965300
**Profile:** Eco 100K First Runnable (Milestone 1)
**Release Version:** v0.1.0-rc1

## 1. System & Resource Envelope Proof
- **CPU Quota:** 50% (0.50 core cgroup-v2 enforced)
- **Memory Max:** 500 MiB (No swap, cgroup-v2 enforced)
- **Tasks Max:** 64
- **Managed Disk Reserve:** 512 MiB protected Recovery Reserve

## 2. Milestone 1 Ticket Completion Ledger
- [x] **Ticket 01–12**: Core runtime, SQLite WAL/FULL durability, durable draft arbitration, streaming generator, safe expression VM, deterministic If routing, closed-branch Merge, 100k Summarize reducer, two-SIGKILL crash-recovery proof.
- [x] **Ticket 13**: Resource Governor token bucket, CPU throttling backpressure, and cgroup-v2 metrics.
- [x] **Ticket 14**: Large Editor virtualized canvas seam verified with 100,000-Node-Instance topology (eco-100k-editor-fixture.cwbt).
- [x] **Ticket 15**: Clean-room n8n 2.39.0 workflow importer with fail-closed credential/secret scanner.
- [x] **Ticket 16**: WCAG 2.2 AA visible focus indicators, keyboard navigation, and ARIA tree/listbox semantics across all critical journeys.
- [x] **Ticket 17**: Tiered retention profiles, pre-run storage estimates, evidence pinning, and safe terminal run compaction.
- [x] **Ticket 18**: Verified Recovery Sets, non-destructive Restore Drill, and fail-closed Quarantine Mode.
- [x] **Ticket 19**: Immutable Current/Previous release slots, preflight verification, and automatic pre-traffic rollback.
- [x] **Ticket 20**: Hardened multi-arch container packaging: Dockerfile (non-root 10001, read-only root), docker-compose, and Podman Quadlet unit.
- [x] **Ticket 21**: Comprehensive certification and reproducible evidence manifest.

## 3. Test Suites Verification Summary
- **Editor & Accessibility Test Suite (
pm run test:node):** 26/26 PASS (100%)
- **Editor TypeScript Compilation & Build:** 0 errors (clean)
- **Python Indexer & Cache Suite:** 3/3 PASS

## 4. Artifact & Binary Digests
- **Topology Digest (100k nodes):** sha256:0fefa26b18bb57e1b52562ddca272e96f70dee8029451034a6719f5fa685c4a3
