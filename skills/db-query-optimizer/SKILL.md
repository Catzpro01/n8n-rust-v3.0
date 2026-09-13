---
name: db-query-optimizer
description: >-
  Autonomous SQL query tuner and database performance optimizer. Analyzes EXPLAIN plans, detects missing indexes, N+1 patterns, and generates optimal migrations.
---

# DB Query Optimizer — Autonomous Database Performance Tuner

## Purpose
Diagnoses slow database queries and ORM inefficiencies, automatically generating optimal indexing strategies, query rewrites, and database migration scripts.

## Diagnostic Checklist
- **Execution Plan Analysis:** Inspect EXPLAIN (ANALYZE, BUFFERS) for sequential scans on large tables.
- **ORM N+1 Detection:** Identify looped queries in application logic and rewrite with eager loading/joins.
- **Index Recommendations:** Generate composite, partial, or B-tree/GIN index creation DDL statements.
- **Locking & Deadlock Prevention:** Verify transaction isolation levels and lock order consistency.
