---
name: swe-overnight-builder
description: Autonomous PRD-to-Production sprint engine for AFK / overnight development. Decomposes massive specifications into dependency-ordered atomic units.
---

# SWE Overnight Builder (AFK Sprint Engine)

## Purpose
Enables fully autonomous overnight engineering runs by turning high-level PRD documents and feature roadmaps into structured, dependency-ordered task graphs.

## Core Capabilities:
1. **PRD & Architecture Decomposition**:
   - Breaks monolithic features into decoupled packages/modules.
2. **Dependency Graph Ordering**:
   - Orders tasks so foundational models/types are built before APIs, and APIs before UIs.
3. **Shadow Staging & Failure Rollback**:
   - Stages risky changes in isolated branches, rolling back cleanly if fatal barriers are met.
