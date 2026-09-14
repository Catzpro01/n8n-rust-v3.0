#!/usr/bin/env python3
"""Build the independently-authored Arena design skill pack."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
AUDITED_AT = "2026-09-11"

TRIGGERS = {
    "excalidraw": ["diagram", "architecture", "arsitektur", "flowchart", "sequence", "wireframe", "sketch"],
    "penpot": ["penpot", "design handoff", "handoff desain", "vector handoff"],
    "onlook": ["frontend", "react", "next.js", "preact", "css", "web page", "halaman", "dashboard", "editor UI"],
    "shadcn-ui-engine": ["component", "komponen", "form", "dialog", "menu", "table", "tabel", "button"],
    "assistant-generative-ui": ["assistant UI", "chat UI", "generative UI", "agent canvas", "streaming card", "tool card"],
    "design-md-tokens": ["design system", "sistem desain", "design token", "token desain", "theme", "tema", "palette", "palet", "typography", "tipografi"],
    "ui-ux-pro-max": ["design", "desain", "UI", "UX", "user interface", "tampilan", "layout", "responsive", "mobile"],
    "taste-skill": ["visual", "aesthetic", "estetika", "polish", "redesign", "make it beautiful", "perindah"],
    "hallmark": ["design audit", "audit desain", "professional", "profesional", "release-ready", "final polish"],
    "axe-core-a11y": ["accessibility", "aksesibilitas", "a11y", "WCAG", "ARIA", "keyboard", "contrast", "kontras"],
    "visual-regression-qa": ["visual regression", "screenshot", "pixel diff", "viewport", "resolution", "resolusi"],
    "remotion-best-practices": ["remotion", "programmatic video", "video", "animation", "animasi", "composition"],
    "understand-figma": ["figma", "figma export", "figma plugin", "design file", "file desain"],
}

skills = [
    {
        "name": "excalidraw",
        "description": "Create editable system diagrams, architecture flows, wireframes, and matching SVG previews.",
        "source": "https://github.com/excalidraw/excalidraw",
        "source_license": "MIT",
        "kind": "artifact workflow",
        "body": """## Use when
Use for architecture maps, sequence flows, state machines, data paths, and low-fidelity wireframes.

## Workflow
1. Restate the diagram's audience, boundary, and one central message.
2. Inventory nodes, actors, trust boundaries, data stores, and edge labels before drawing.
3. Prefer one readable left-to-right or top-to-bottom flow; split genuinely different views instead of creating a giant poster.
4. Generate an editable `.excalidraw` JSON artifact and a self-contained `.svg` preview when practical.
5. Validate JSON structure, labels, arrow endpoints, contrast, and legibility at normal zoom.

## Deliverables
- Editable Excalidraw-compatible JSON.
- Embedded, network-free SVG preview.
- Short legend and assumptions when symbols are not self-evident.

## Guardrails
Do not install or self-host the Excalidraw application unless the user separately requests it. Do not put secrets in diagrams. Do not pretend an SVG-only result is an editable Excalidraw file.""",
    },
    {
        "name": "penpot",
        "description": "Prepare open design-system handoff artifacts, vector assets, tokens, components, and CSS-ready specifications.",
        "source": "https://github.com/penpot/penpot",
        "source_license": "MPL-2.0",
        "kind": "handoff workflow",
        "body": """## Use when
Use for UI/UX system planning, component libraries, reusable vector assets, design-token handoff, or preparation for Penpot import.

## Workflow
1. Inspect the product's existing visual language and framework before proposing a new one.
2. Define primitive, semantic, and component tokens with light/dark and interaction states.
3. Specify component anatomy, variants, constraints, keyboard behavior, responsive rules, and content limits.
4. Produce original SVG assets and CSS/JSON token exports that can be imported or recreated in Penpot.
5. Supply a Penpot handoff checklist: pages, boards, components, variants, tokens, exports, and naming.

## Deliverables
- `DESIGN.md`, token files, component/state matrix, and SVG assets as needed.
- Penpot import/recreation notes; native Penpot files only when a supported export path is actually available.

## Guardrails
Penpot is a large application, not embedded by this skill. Never claim to have edited a remote Penpot workspace without credentials and a successful API/plugin operation. Keep third-party assets and licenses traceable.""",
    },
    {
        "name": "onlook",
        "description": "Inspect and improve React, Next.js, or compatible component UIs through source edits and real browser feedback.",
        "source": "https://github.com/onlook-dev/onlook",
        "source_license": "Apache-2.0",
        "kind": "visual source-edit workflow",
        "body": """## Use when
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
Do not force React, Next.js, Tailwind, or Onlook into a project that uses another viable stack. Do not edit generated build output. The upstream Onlook application is not installed by this wrapper.""",
    },
    {
        "name": "shadcn-ui-engine",
        "description": "Design and implement accessible, composable UI components using the host project's own stack.",
        "source": "https://github.com/shadcn-ui/ui",
        "source_license": "MIT",
        "kind": "component workflow",
        "body": """## Use when
Use for forms, dialogs, menus, tables, cards, command surfaces, and reusable application components.

## Workflow
1. Define semantics, states, events, focus order, keyboard behavior, loading/error/empty states, and content constraints.
2. Reuse the host project's primitives and tokens before introducing dependencies.
3. Keep public APIs small, typed, composable, and controlled where state ownership matters.
4. Implement semantic HTML and accessible names first; add visual treatment after behavior works.
5. Test keyboard navigation, reduced motion, high zoom, narrow screens, and automated accessibility.

## Deliverables
- Original component source, examples, and focused tests.

## Guardrails
The name describes a workflow, not an installed shadcn CLI. Add shadcn/Radix/Tailwind packages only with explicit user approval, exact version pinning, license review, and bundle-impact measurement. Do not paste a component merely because it looks familiar.""",
    },
    {
        "name": "assistant-generative-ui",
        "description": "Design truthful streaming assistant interfaces, tool cards, approval flows, and bounded multi-agent canvases.",
        "source": "https://github.com/assistant-ui/assistant-ui",
        "source_license": "MIT",
        "kind": "interaction architecture workflow",
        "body": """## Use when
Use for AI chat, streaming output, tool execution, approvals, multi-agent activity, and generative cards.

## Workflow
1. Model explicit states: queued, streaming, awaiting approval, completed, failed, cancelled, stale, and reconnecting.
2. Separate conversational text from durable artifacts and tool evidence.
3. Show provenance, authority, timestamps, retry safety, cancellation, and partial-result status truthfully.
4. Keep lists and canvases virtualized/bounded; disconnected clients must not block agents.
5. Make live regions, focus movement, keyboard actions, and reduced motion deliberate.
6. Use AG-UI or an upstream package only when the product contract actually needs it.

## Deliverables
- State/event contract, component map, error/recovery behavior, and implementation/tests when requested.

## Guardrails
Never fabricate a completed tool action. Never hide uncertainty behind a perpetual spinner. The assistant-ui package is not installed automatically.""",
    },
    {
        "name": "design-md-tokens",
        "description": "Create a plain-text DESIGN.md and synchronized CSS/JSON design tokens for agent and human use.",
        "source": "https://github.com/VoltAgent/awesome-design-md",
        "source_license": "MIT",
        "kind": "design-system documentation workflow",
        "body": """## Use when
Use to establish or normalize a product's visual language before broad UI work.

## Workflow
1. Audit existing interfaces and identify reusable intent instead of copying a famous brand.
2. Define primitive tokens, then semantic roles, then component aliases; avoid components reading raw palette values.
3. Cover color, typography, spacing, radius, border, elevation, motion, breakpoints, density, focus, disabled, success, warning, and danger.
4. Generate synchronized `DESIGN.md`, `tokens.css`, and `tokens.json` where useful.
5. Include do/don't rules, responsive behavior, contrast targets, and a migration map from existing values.

## Deliverables
- Plain-text design contract plus machine-readable token exports and validation notes.

## Guardrails
The user-supplied `kaushikgopal/awesome-design-md` URL returned 404 during installation; this wrapper uses the active VoltAgent collection only as a reference. Never clone another product's distinctive trade dress or claim extracted public CSS values are an official design system.""",
    },
    {
        "name": "ui-ux-pro-max",
        "description": "Build or audit a complete UI/UX system across hierarchy, color, type, spacing, motion, responsiveness, and accessibility.",
        "source": "https://github.com/heroui-inc/heroui",
        "source_license": "Apache-2.0",
        "kind": "system design workflow",
        "body": """## Use when
Use for a broad product redesign, a new design language, or a cross-screen consistency audit.

## Workflow
1. Identify users, critical journeys, information hierarchy, density, and platform constraints.
2. Establish a small visual thesis and derive tokens, layout rules, component families, and content voice from it.
3. Specify responsive behavior and all interaction states before polishing isolated screenshots.
4. Evaluate learnability, efficiency, error prevention, recovery, accessibility, perceived performance, and touch ergonomics.
5. Produce prioritized findings and implement the highest-impact coherent slice first.

## Deliverables
- Design-system proposal or audit report, prioritized fixes, and measurable acceptance criteria.

## Guardrails
NextUI now resolves to the HeroUI project; this skill is independently authored and does not represent an official `ui-ux-pro-max` package. Do not redesign merely to chase a trend, and do not erase established product identity without approval.""",
    },
    {
        "name": "taste-skill",
        "description": "Apply an aesthetic quality gate that removes generic AI-design habits while preserving product identity and usability.",
        "source": "https://github.com/shadcn-ui/ui",
        "source_license": "MIT",
        "kind": "aesthetic critique workflow",
        "body": """## Use when
Use after a first visual draft or when a UI feels generic, noisy, artificial, or template-driven.

## Workflow
1. State the product's intended mood in one sentence and test every visual choice against it.
2. Remove unearned gradients, glass effects, glow, excessive pills, nested cards, decorative charts, emoji-as-icons, and filler copy.
3. Repair hierarchy with type, alignment, rhythm, whitespace, and one restrained accent before adding decoration.
4. Check that important actions are visually dominant and destructive/secondary actions are honest.
5. Preserve intentional quirks that support identity; this is curation, not forced minimalism.

## Deliverables
- Ranked taste findings with concrete keep/remove/change instructions and an updated artifact when requested.

## Guardrails
This is an independent review rubric inspired by high-quality open component work, not an upstream shadcn skill. Aesthetic preference never overrides accessibility, truthful state, or task success.""",
    },
    {
        "name": "hallmark",
        "description": "Perform a professional post-build UI audit and apply evidence-backed structural and visual repairs.",
        "source": "https://github.com/tailwindlabs/tailwindcss",
        "source_license": "MIT",
        "kind": "quality remediation workflow",
        "body": """## Use when
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
Tailwind CSS is a reference, not a required dependency and not an automatic UI auditor. Do not rewrite stable UI wholesale when a token or component-level fix is sufficient.""",
    },
    {
        "name": "axe-core-a11y",
        "description": "Run automated and manual accessibility checks targeting WCAG 2.2 AA behavior.",
        "source": "https://github.com/dequelabs/axe-core",
        "source_license": "MPL-2.0",
        "kind": "accessibility QA workflow",
        "body": """## Use when
Use for every meaningful web UI change and before release.

## Workflow
1. Exercise real states and routes in a real browser, not only static HTML.
2. Use an already-installed accessibility runner when available; otherwise request approval before adding an exact-pinned development dependency.
3. Run axe rules at relevant viewports and store machine-readable violations.
4. Manually test keyboard-only use, focus visibility/order, dialogs, skip paths, live updates, zoom/reflow, reduced motion, labels, errors, and contrast.
5. Fix root semantics before adding ARIA and add regression tests for verified failures.

## Deliverables
- Automated results, manual checklist, source fixes, and known limitations.

## Guardrails
A zero-violation axe report is not proof of WCAG conformance. Never suppress a rule without a documented, reviewed reason. Axe remains development/test tooling unless explicitly needed at runtime.""",
    },
    {
        "name": "visual-regression-qa",
        "description": "Create deterministic multi-viewport screenshot baselines and review meaningful visual diffs.",
        "source": "https://github.com/puppeteer/puppeteer",
        "source_license": "Apache-2.0",
        "kind": "browser QA workflow",
        "body": """## Use when
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
Never auto-approve changed baselines. Browser binaries and image-diff tooling are development-only and must not enter a lightweight production bundle.""",
    },
    {
        "name": "remotion-best-practices",
        "description": "Plan deterministic React-based video compositions, timelines, assets, captions, audio, and render validation.",
        "source": "https://github.com/remotion-dev/remotion",
        "source_license": "Custom Remotion License",
        "kind": "video production workflow",
        "body": """## Use when
Use when the user requests a programmatic video, animation composition, or Remotion code review.

## Workflow
1. Define output size, frame rate, duration, codec, safe areas, narrative beats, and asset provenance.
2. Make every frame a deterministic function of frame number and props; avoid wall clock, network fetches during render, and CSS transitions with hidden timing.
3. Preload assets, bound memory, split long work into compositions/sequences, and design captions/audio for accessibility.
4. Render representative frames and a short proof segment before the full output.
5. Validate duration, dropped frames, audio levels, text clipping, licenses, and final metadata.

## Deliverables
- Composition plan/code, asset manifest, preview frames, and render report when the runtime is approved.

## Guardrails
No Remotion package or code is installed by this skill. Its repository uses a custom license: individuals, non-profits, and qualifying small companies may be eligible for free use, while other for-profit organizations need a company license. Confirm eligibility and exact terms before dependency installation or rendering. Arena speech tools cannot sing and are not a substitute for music licensing.""",
    },
    {
        "name": "understand-figma",
        "description": "Turn user-authorized Figma exports into a traceable knowledge graph of pages, layers, tokens, components, and assets.",
        "source": "https://github.com/figma/plugin-samples",
        "source_license": "MIT",
        "kind": "design ingestion workflow",
        "body": """## Use when
Use when the user supplies a Figma export/plugin payload or authorizes API access and wants design structure understood or mapped to code.

## Workflow
1. Confirm scope, file ownership/authorization, export format, and whether secrets or private comments must be excluded.
2. Parse pages, frames, groups, components, instances, variants, variables/styles, constraints, interactions, text, and asset references.
3. Normalize stable IDs and relationships into a provenance-preserving knowledge graph.
4. Map variables/styles to semantic design tokens and components to implementation candidates; flag detached instances and one-off overrides.
5. Produce summaries at page, component, and token levels plus an unresolved-ambiguity list.

## Deliverables
- `figma-graph.json`, design inventory, token mapping, asset manifest, and implementation notes when source data is provided.

## Guardrails
Figma plugin samples are examples, not a ready-made parser product. Never request or store an API token in committed files. Never claim to understand an inaccessible live Figma file. Preserve provenance and do not copy third-party brand assets without permission.""",
    },
]

for skill in skills:
    folder = ROOT / skill["name"]
    folder.mkdir(parents=True, exist_ok=True)
    trigger_lines = "\n".join(f"  - {json.dumps(value)}" for value in TRIGGERS[skill["name"]])
    content = f"""---
name: {skill['name']}
description: {skill['description']}
mode: arena-workspace
auto_activate: true
triggers:
{trigger_lines}
source: {skill['source']}
source_license: {skill['source_license']}
audited_at: {AUDITED_AT}
---

# {skill['name']}

{skill['body']}

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
"""
    (folder / "SKILL.md").write_text(content)

registry = {
    "schema": 1,
    "pack": "arena-design-skills",
    "installed_at": AUDITED_AT,
    "mode": "workspace-instruction-pack",
    "upstream_code_installed": False,
    "auto_activation": {
        "enabled": True,
        "router": "route.py",
        "policy": "select-minimal-matching-design-skills",
        "explicit_opt_out_supported": True,
    },
    "skills": [
        {
            "name": item["name"],
            "description": item["description"],
            "kind": item["kind"],
            "path": f"{item['name']}/SKILL.md",
            "source": item["source"],
            "source_license": item["source_license"],
            "audited_at": AUDITED_AT,
            "auto_activate": True,
            "triggers": TRIGGERS[item["name"]],
        }
        for item in skills
    ],
}
(ROOT / "registry.json").write_text(json.dumps(registry, indent=2, ensure_ascii=False) + "\n")
