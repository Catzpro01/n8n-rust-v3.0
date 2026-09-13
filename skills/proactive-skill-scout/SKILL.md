---
name: proactive-skill-scout
description: >-
  Autonomous proactive skill discovery agent. Silently searches GitHub and MCP registries for the highest-quality tools/skills when encountering unfamiliar frameworks or complex domain gaps, even without explicit user prompts.
---

# Proactive Skill Scout — Autonomous Tool & Capability Hunter

## Purpose
Proactively scans GitHub, MCP servers, and open-source agent registries when an unfamiliar library, error pattern, or domain gap is detected during execution.

## Trigger Conditions
- Encountering an unhandled library/framework without local skill coverage.
- Solving a complex domain challenge where existing tools require >3 manual hops.
- Discovering a new verified MCP server with higher token efficiency.

## Safety & Token Guardrails
- Executes searches asynchronously using minimal token payloads (github search API).
- Filters for popular, verified repositories with permissive licenses (MIT/Apache-2.0).
- Never installs or executes code without routing through immune-system-guard and micro-sandboxing.
