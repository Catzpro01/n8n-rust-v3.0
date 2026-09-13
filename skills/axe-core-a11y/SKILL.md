---
name: axe-core-a11y
description: >-
  Automated accessibility auditor based on Deque axe-core. Validates WCAG 2.2 AA compliance, color contrast, keyboard navigation, and ARIA attributes.
---

# Axe-Core A11y — Automated Accessibility Engine (Deque Labs)

## Purpose
Audits AI-generated HTML/React code against WCAG 2.2 AA standards, ensuring complete focus management, keyboard accessibility, and contrast compliance.

## Audit Checklist
1. **Contrast Ratio:** Minimum 4.5:1 for standard text, 3:1 for large text/icons.
2. **Focus Visibility:** Clear, visible :focus-visible ring on all interactive elements.
3. **ARIA Labeling:** Interactive buttons/icons must have descriptive ria-label or text children.
4. **Form Association:** Labels must explicitly connect to inputs via htmlFor / id.
