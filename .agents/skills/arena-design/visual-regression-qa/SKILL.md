---
name: visual-regression-qa
description: Create deterministic multi-viewport screenshot baselines and review meaningful visual diffs.
mode: arena-workspace
auto_activate: true
triggers:
  - "visual regression"
  - "screenshot"
  - "pixel diff"
  - "viewport"
  - "resolution"
  - "resolusi"
source: https://github.com/puppeteer/puppeteer
source_license: Apache-2.0
audited_at: 2026-09-11
---

# visual-regression-qa

## Use when
Use to prevent unintended layout, typography, state, or responsive regressions.

## Workflow
1. Prefer the browser framework already in the project; do not add Puppeteer when Playwright is present and sufficient.
2. Freeze viewport, device scale, locale, timezone, color scheme, reduced motion, data, animation, caret, and font readiness.
3. Capture named baselines for important states, including loading, empty, error, overflow, and modal states.
4. Compare with a declared pixel policy; mask only truly nondeterministic regions and review every baseline update.
5. Pair screenshot checks with semantic/functional assertions so a pretty but broken page cannot pass.

## Deliverables
- Versioned test, baseline images, diff images, thresholds, and review instructions.

## Guardrails
Never auto-approve changed baselines. Browser binaries and image-diff tooling are development-only and must not enter a lightweight production bundle.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
