---
name: onlook
description: Inspect and improve React, Next.js, or compatible component UIs through source edits and real browser feedback.
mode: arena-workspace
auto_activate: true
triggers:
  - "frontend"
  - "react"
  - "next.js"
  - "preact"
  - "css"
  - "web page"
  - "halaman"
  - "dashboard"
  - "editor UI"
source: https://github.com/onlook-dev/onlook
source_license: Apache-2.0
audited_at: 2026-09-11
---

# onlook

## Use when
Use when a web UI exists and the user wants visual changes written back to source code.

## Workflow
1. Read the project structure, framework, styling approach, commands, and local instructions.
2. Start the existing development server rather than replacing its stack.
3. Capture representative states and viewports with the project's browser tooling; inspect console and network failures.
4. Trace each visual issue to the owning component/token/style, make the smallest coherent source change, and preserve behavior.
5. Re-run typecheck, build, functional tests, accessibility checks, and visual comparison.

## Deliverables
- Source diff, validation results, and before/after screenshots when browser tooling is available.

## Guardrails
Do not force React, Next.js, Tailwind, or Onlook into a project that uses another viable stack. Do not edit generated build output. The upstream Onlook application is not installed by this wrapper.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
