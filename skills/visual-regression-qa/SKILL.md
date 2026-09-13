---
name: visual-regression-qa
description: >-
  Automated pixel-diff and visual regression testing engine. Uses headless browser snapshots to verify visual layout integrity across viewport sizes.
---

# Visual Regression QA — Pixel-Diff & Layout Verifier

## Purpose
Prevents unintended CSS layout regressions by taking pre/post modification screenshots and computing pixel difference matrices across desktop and mobile viewports.

## Protocols
1. **Baseline Capture:** Take viewport screenshot of target component/page before applying UI edits.
2. **Post-Edit Snapshot:** Take matching screenshot post-compilation.
3. **Pixel Diff Analysis:** Calculate visual delta percentage; flag unintended padding shifts, overflow clipping, or font reflows.
4. **Auto-Correction:** Fix broken CSS declarations before presenting output to user.
