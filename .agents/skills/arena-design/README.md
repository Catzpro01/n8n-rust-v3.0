# Arena Design Skills

A curated, independently authored instruction pack for Arena agent sessions.

## Installed skills

1. `excalidraw`
2. `penpot`
3. `onlook`
4. `shadcn-ui-engine`
5. `assistant-generative-ui`
6. `design-md-tokens`
7. `ui-ux-pro-max`
8. `taste-skill`
9. `hallmark`
10. `axe-core-a11y`
11. `visual-regression-qa`
12. `remotion-best-practices`
13. `understand-figma`

Start with [`SKILL.md`](SKILL.md) for routing. Each folder contains the actual workflow instructions. [`registry.json`](registry.json) provides a machine-readable index, and [`SOURCES.md`](SOURCES.md) records source curation and licensing caveats.

## Automatic activation

Design intent is automatically routed by [`route.py`](route.py). Workspace-level activation is recorded in [`/home/user/AGENTS.md`](../AGENTS.md). It selects only relevant skills for UI/UX, frontend presentation, components, diagrams, Figma/Penpot, tokens, accessibility, visual regression, assistant UI, animation, or video. Pure backend/database work stays untouched. An explicit “jangan gunakan skill desain” disables activation for that request.

## What “installed” means

The pack is stored in the persistent Arena workspace and can be invoked by name in this conversation. It does **not** modify Arena's hidden platform registry, and it does not pretend that unrelated GitHub applications are native Arena plugins.

No upstream application or dependency was cloned, installed, or added to a production bundle. If a task later needs an actual package or service, the agent must ask first, pin an exact version, review its license/security posture, and keep heavy tooling development-only or remote where possible.

## Validate

```bash
python3 validate.py
python3 test_route.py
python3 route.py --json "redesign tampilan dashboard React"
```
