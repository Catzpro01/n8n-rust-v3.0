---
name: composio-orchestrator
description: >-
  Multi-agent orchestration framework by Composio for tool execution, GitHub actions, and CI/CD pipelines.
---

# Composio Agent Orchestrator

## Architecture
- Planner: Breaks complex objectives into dependency graphs
- Executor: Runs tools, APIs, and git commands in isolated environments
- Verifier: Validates task output against acceptance criteria

## Guardrails
- Enforce human review for destructive git pushes or production deployments
