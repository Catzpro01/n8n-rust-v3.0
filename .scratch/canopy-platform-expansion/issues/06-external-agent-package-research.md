# 06: Research external agent and package adapter boundaries

Type: wayfinder-research
Status: resolved
Blocked by: None
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/14
Resolved: 2026-09-14
Research note: `docs/research/external-agent-package-adapters-2026-09.md`

## Question

Collect primary-source facts needed before locking the external adapter boundary: MCP protocol/version and authorization behavior, A2A interoperability surface, OpenAI-compatible tool/stream semantics, and operational constraints/licensing of the named optional agent adapters and package sources.

## Answer

The official research note records the facts and citations. Key consequences are:

- MCP authorization is transport-specific; HTTP uses scoped OAuth-style discovery, while STDIO needs an isolated local credential mechanism and explicit Secret Lease.
- A2A 1.0 provides Agent Cards, opaque remote agents, tasks, streaming, cancellation, subscription, artifacts, and push notifications; it maps to the remote Agent Engine boundary without sharing internal memory/tools.
- OpenAI-style tool calling is an application-controlled loop with call IDs and typed streaming events; provider-specific events must be normalized behind Agent Engine.
- Hermes, OpenCode, OpenClaw, Claude Code, and Antigravity remain optional isolated/remote-first adapters with pinned source/release, capability, license, cost, and conformance evidence. MiroFish's intended primary identity remains unresolved.

No production code or third-party source was imported.
