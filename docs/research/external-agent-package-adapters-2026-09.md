# External agent and package adapter boundary research

**Date:** 2026-09-14
**Ticket:** Wayfinder issue #14
**Status:** research complete; facts only, no production code
**Method:** primary documentation and official repositories where available; no external source code or secrets imported.

## Model Context Protocol (MCP)

The MCP authorization specification says authorization is optional overall. For
HTTP transports, implementations that support authorization should use the
specified OAuth-based flow; STDIO implementations should not use that HTTP
authorization flow and instead obtain credentials through their local mechanism.
Protected HTTP resources use OAuth 2.1-style roles/discovery and Protected
Resource Metadata. MCP task state must be bound to the authorization context
when available, and task operations need access control and rate limiting.

Sources: [1](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization), [3](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks), [5](https://modelcontextprotocol.io/extensions/auth/overview).

**Adapter consequence:** the platform must not pass a general credential or
provider token merely because an MCP server is connected. HTTP MCP requires a
scoped capability/authorization mapping; local STDIO uses an isolated worker
and an explicit Secret Lease. Task IDs and result reads must remain bound to
that authorization context.

## Agent2Agent (A2A)

The official released A2A specification is version 1.0.0. It models opaque
remote agents through an Agent Card and supports synchronous messages,
long-running Tasks, JSON-RPC/gRPC/HTTP bindings, streaming updates, task
subscription, cancellation, and push notifications. The protocol is designed
to exchange capabilities and artifacts without sharing an agent's private
memory, internal plan, or tool implementation.

Source: [3](https://a2a-protocol.org/v1.0.0/specification/).

**Adapter consequence:** A2A fits the Heavy Orchestrator/remote Agent Engine
boundary. Agent Card capabilities must be recorded before choosing streaming
or push; Task identity, cancellation, reconnect, artifact references, auth,
and terminal outcomes must map to Canopy's durable Activation/Run evidence.
A2A does not justify importing a remote agent's internal memory or tools into
the daemon.

## OpenAI-compatible model/tool APIs

The official OpenAI function-calling documentation describes a five-step
application loop: send tools, receive a tool call, execute it in the
application, return tool output tied to the call, and receive the next/final
response. The official streaming documentation uses typed semantic SSE events
for incremental output; tool-call arguments are also streamed as events.

Sources: [1](https://developers.openai.com/api/docs/guides/function-calling), [2](https://developers.openai.com/api/docs/guides/streaming-responses).

**Adapter consequence:** an OpenAI-compatible adapter must normalize provider
responses into a provider-neutral turn/tool event model. `call_id`, tool
arguments, tool output, partial text, refusal/error, usage, cancellation, and
terminal response are evidence fields; provider-specific streaming event names
must not leak into the public Agent Engine contract.

## Optional external agent runtimes

### Hermes Agent

Nous Research's official site describes Hermes as a self-hosted agent with
persistent memory, skills, messaging surfaces, isolated subagents, and sandbox
backends including local, Docker, SSH, Singularity, and Modal. The site states
that Hermes is MIT-licensed and that hosted model/tool services are optional.

Source: [4](https://hermes-agent.nousresearch.com/).

**Boundary:** treat Hermes as an optional isolated/remote Agent Engine. Its
memory, skills, channels, and sandbox are adapter-visible capabilities, not
implicit access to Canopy storage or credentials.

### OpenCode

OpenCode's official CLI documentation exposes explicit agent permissions,
including read/edit/shell/task/web/MCP-related permissions, supports local or
remote server attachment, and provides explicit MCP add/auth/logout commands.
The official repository identifies the project as MIT-licensed, but service or
provider terms are separate from the source license.

Sources: [1](https://opencode.ai/docs/cli/), [1](https://github.com/anomalyco/opencode/blob/dev/LICENSE), [4](https://github.com/anomalyco/opencode/?tab=MIT-1-ov-file).

**Boundary:** run OpenCode as an isolated External Process/Heavy Orchestrator
with an allowlisted permission profile. Its provider credentials remain
outside the workflow payload and arrive only through an explicit Secret Lease.

### OpenClaw

The official OpenClaw repository is public and rapidly changing. Its official
license file states MIT, and the repository records third-party notices for
incorporated/adapted code. The repository's operational surface is large and
must therefore be pinned by release/commit and scanned before use.

Sources: [1](https://github.com/openclaw/openclaw), [2](https://github.com/openclaw/openclaw/blob/main/LICENSE), [1](https://github.com/openclaw/openclaw/blob/main/THIRD_PARTY_NOTICES.md).

**Boundary:** remote-first/local-opt-in only; never install the mutable main
branch as a trusted resident dependency. Lock the exact source/release,
license/notice set, requested capabilities, worker image, and conformance
fixture.

### Claude Code

Anthropic's official documentation describes Claude Code as a CLI/IDE/desktop/
web coding assistant with tool and project access. The documented installation
and provider/subscription requirements are controlled by Anthropic; the runtime
is not treated as an open-source package in this platform's default bundle.

Source: [1](https://code.claude.com/docs/en/overview).

**Boundary:** external proprietary adapter only, with explicit provider terms,
credential scope, network policy, cost budget, and worker isolation. Do not
vendor or reimplement its source.

### Antigravity and MiroFish

An official Google Antigravity CLI repository and official product/docs links
were discoverable, and an official Antigravity SDK announcement states Apache
2.0 for that SDK. The CLI's exact distribution, authentication, release, and
runtime license still need to be pinned separately before certification.

Sources: [2](https://github.com/google-antigravity/antigravity-cli), [4](https://antigravity.google/blog/introducing-google-antigravity-sdk).

No authoritative primary source for the intended MiroFish adapter was
identified in this research pass. MiroFish therefore remains an unresolved
adapter identity, not a certified implementation or license claim.

## Cross-cutting findings

1. External agents are adapters, not resident core dependencies.
2. Every adapter needs an immutable source/release identity, license/notice
   evidence, capability manifest, resource/cost budget, authentication method,
   conformance fixture, and typed outcome mapping.
3. MCP and A2A task/stream identity must be bound to the Canopy Run/Activation
   and authorization context; a remote task ID alone is not sufficient.
4. Provider-specific model/tool streaming is normalized behind Agent Engine;
   partial events remain speculative until durable checkpoint/Output Contract
   validation.
5. No research result authorizes importing third-party source, credentials,
   private memory, or mutable release branches into the default daemon.

## Not decided here

This note does not decide the AI Agent Node turn contract, Model Route policy,
Memory Stack, Skill/MCP scope, Output Contract, or upgrade behavior. Those are
Owner decisions in Wayfinder issue #11 and later #12/#13.
