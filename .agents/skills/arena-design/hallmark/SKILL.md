---
name: hallmark
description: Perform a professional post-build UI audit and apply evidence-backed structural and visual repairs.
mode: arena-workspace
auto_activate: true
triggers:
  - "design audit"
  - "audit desain"
  - "professional"
  - "profesional"
  - "release-ready"
  - "final polish"
source: https://github.com/tailwindlabs/tailwindcss
source_license: MIT
audited_at: 2026-09-11
---

# hallmark

## Use when
Use near completion to turn a merely functional interface into a coherent, release-ready one.

## Workflow
1. Capture key screens and states at phone, tablet, and desktop widths.
2. Score hierarchy, alignment, spacing, type, color, components, content, responsiveness, interaction, accessibility, and performance.
3. Separate defects from preferences; tie each defect to user impact and source ownership.
4. Fix shared tokens/primitives before one-off CSS, then verify all affected surfaces.
5. Re-run browser, accessibility, and visual-regression checks and report remaining risks.

## Deliverables
- Audit scorecard, prioritized remediation diff, comparison evidence, and residual-risk list.

## Guardrails
Tailwind CSS is a reference, not a required dependency and not an automatic UI auditor. Do not rewrite stable UI wholesale when a token or component-level fix is sufficient.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
