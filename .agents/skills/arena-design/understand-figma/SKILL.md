---
name: understand-figma
description: Turn user-authorized Figma exports into a traceable knowledge graph of pages, layers, tokens, components, and assets.
mode: arena-workspace
auto_activate: true
triggers:
  - "figma"
  - "figma export"
  - "figma plugin"
  - "design file"
  - "file desain"
source: https://github.com/figma/plugin-samples
source_license: MIT
audited_at: 2026-09-11
---

# understand-figma

## Use when
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
Figma plugin samples are examples, not a ready-made parser product. Never request or store an API token in committed files. Never claim to understand an inaccessible live Figma file. Preserve provenance and do not copy third-party brand assets without permission.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
