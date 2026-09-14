---
name: assistant-generative-ui
description: Design truthful streaming assistant interfaces, tool cards, approval flows, and bounded multi-agent canvases.
mode: arena-workspace
auto_activate: true
triggers:
  - "assistant UI"
  - "chat UI"
  - "generative UI"
  - "agent canvas"
  - "streaming card"
  - "tool card"
source: https://github.com/assistant-ui/assistant-ui
source_license: MIT
audited_at: 2026-09-11
---

# assistant-generative-ui

## Use when
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
Never fabricate a completed tool action. Never hide uncertainty behind a perpetual spinner. The assistant-ui package is not installed automatically.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
