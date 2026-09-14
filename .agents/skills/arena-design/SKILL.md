---
name: arena-design-suite
description: Route design requests through a curated set of diagram, design-system, implementation, accessibility, and visual-QA workflows.
mode: arena-workspace
installed_at: 2026-09-11
---

# Arena Design Suite

This is the entry point for the independently authored design skill pack.

## Automatic activation

Automatic activation is mandatory for requests involving visual design, UI/UX, frontend presentation, components, diagrams, Figma/Penpot, design systems/tokens, accessibility, screenshots/visual regression, generative interfaces, animation, or programmatic video.

For every new user request:

1. Run `python3 /home/user/arena-design-skills/route.py --json "<request>"` or apply the same deterministic policy directly.
2. If `design_intent` is true, read every selected skill's `SKILL.md` before planning or editing.
3. Tell the user briefly, in simple Indonesian, which design skills activated and why.
4. Apply the smallest selected pipeline; do not activate all 13 merely because one design word appears.
5. Respect explicit opt-out phrases such as “jangan gunakan skill desain”.

Backend, database, infrastructure, or prose work without a visual/design signal must not activate the suite.

## Routing

Choose the smallest useful set rather than invoking every skill:

1. **Understand inputs**
   - `understand-figma` for user-authorized Figma exports.
   - `excalidraw` for architecture, flows, and wireframes.
2. **Define the system**
   - `design-md-tokens` for the plain-text visual contract and tokens.
   - `ui-ux-pro-max` for broad UX and design-system decisions.
3. **Build or edit**
   - `shadcn-ui-engine` for accessible components in the host stack.
   - `onlook` for browser-guided source editing.
   - `assistant-generative-ui` for streaming and agent interfaces.
   - `remotion-best-practices` for programmatic video work.
4. **Polish and verify**
   - `taste-skill` for aesthetic curation.
   - `hallmark` for release-ready remediation.
   - `axe-core-a11y` for automated plus manual accessibility QA.
   - `visual-regression-qa` for deterministic screenshot comparison.
   - `penpot` for open design handoff artifacts.

## Invocation

The user may invoke a skill by name, for example:

- `gunakan taste-skill untuk halaman ini`
- `/skill axe-core-a11y`
- `jalankan onlook lalu visual-regression-qa`

Read this router and the selected skill's `SKILL.md`, explain the intended work in very simple Indonesian, then perform the workflow using available Arena tools.

## Global rules

- Inspect the existing project before choosing a framework or dependency.
- Prefer existing tools and lightweight, development-only test dependencies.
- Never install a large application, browser, framework, MCP, or service without explicit approval.
- Never copy distinctive trade dress, protected assets, proprietary content, or credentials.
- Keep generated previews self-contained because Arena file previews have no network access.
- Treat automated accessibility and visual checks as evidence, not proof of correctness.
- Preserve source provenance and state uncertainty honestly.
- For code changes: test, review the diff, and report bundle/runtime impact.
