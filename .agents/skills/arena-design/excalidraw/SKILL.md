---
name: excalidraw
description: Create editable system diagrams, architecture flows, wireframes, and matching SVG previews.
mode: arena-workspace
auto_activate: true
triggers:
  - "diagram"
  - "architecture"
  - "arsitektur"
  - "flowchart"
  - "sequence"
  - "wireframe"
  - "sketch"
source: https://github.com/excalidraw/excalidraw
source_license: MIT
audited_at: 2026-09-11
---

# excalidraw

## Use when
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
Do not install or self-host the Excalidraw application unless the user separately requests it. Do not put secrets in diagrams. Do not pretend an SVG-only result is an editable Excalidraw file.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
