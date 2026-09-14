---
status: accepted
owner_decision: 2026-09-14
---
# Use a private-first dual-lane expansion gate

## Context

The expansion map now has accepted contracts for the extension lane, Hub and
Skill Package lifecycle, AI Agent turns, external adapters, and release/recovery
state. A phase must be proven as one product rather than a collection of
placeholder documents or an unbounded compatibility promise. The gate must
also distinguish deterministic platform guarantees from inherently variable
provider and external-agent evidence.

The Owner selected these #13 policies through HITL on 2026-09-14:

- a private-first vertical acceptance journey;
- dual-lane deterministic and nondeterministic evidence;
- the Rust default profile with optional remote-first/local-opt-in workers;
- production completion only with code, versioned contracts/UI seams, tests,
  documentation, and retained release evidence.

## Decision

### 1. Smallest integrated acceptance journey

The canonical fixture is a private control-plane journey that:

1. boots the signed Rust daemon/editor in the default resource and security
   profile;
2. creates a Mutable Draft and imports/inspects the required Workflow and
   Skill Package locks without granting unverified authority;
3. publishes a locked Workflow Revision and Agent Blueprint with effective
   scope, Model Route, Memory/Skill/MCP selections, Policy, Output Contract,
   Capability Grants, and no ambient credentials;
4. runs a bounded deterministic Agent Turn through a fixture engine, including
   a Memory reference, safe tool request, a classified pre-side-effect
   fallback, output validation/repair, Artifact production, and downstream
   delivery only after validation;
5. injects suspension/resume, cancellation, duplicate delivery, timeout,
   unknown external outcome, invalid output, budget exhaustion, and daemon
   restart cases and verifies typed outcomes, idempotency, reconciliation, and
   redacted Causal Trace;
6. creates and verifies a complete Recovery Set, performs an isolated restore
   drill with ingress/credentials/side effects disabled, then exercises a
   signed Staging → Current/Previous upgrade before new traffic;
7. verifies the browser/control surfaces for lock visibility, scope, trust,
   outcome, recovery, and operator evidence, including keyboard and accessible
   status behavior.

The fixture is private by default. External provider calls, public webhooks,
production credentials, and irreversible writes are never prerequisites for
the deterministic release gate.

### 2. Two evidence lanes

**Deterministic contract lane** is the release authority for platform
semantics. It uses fake/fixture engines and tools, bounded schemas, fixed
inputs, resource profiles, failure injection, replay, migration fixtures, and
cryptographic/content-addressed expectations. It proves plan/Blueprint locks,
capability enforcement, budgets, ordering, idempotency, Output Contracts,
recovery, and editor/control behavior.

**External conformance lane** records real provider, MCP, A2A, package, and
agent-adapter evidence separately. It pins source/release/license and notice
evidence, capabilities, network/auth mode, cost, latency, resource usage,
stream/event normalization, cancellation, error/uncertain behavior, and
conformance fixture results. Its variability is visible and quarantined; it
cannot weaken a deterministic contract assertion or authorize a worker.

### 3. Resource and trust envelope

Every phase must pass in the default private Rust profile and preserve the
existing Eco memory/CPU/disk/security budgets. Heavy engines, GPU, browser,
Node/Python/other language runtimes, hosted providers, and external agent
harnesses are optional remote-first/local-opt-in workers. They require pinned
identity, capability manifest, lane, license/notices, provenance, resource and
cost budget, compatibility evidence, and explicit grants. The default bundle
contains none of those runtimes or plaintext credentials.

### 4. Production-complete phase gate

A phase is complete only when all applicable evidence is retained:

- production behavior exists; no placeholder or demo-only path is accepted;
- a versioned domain/API/adapter contract exists, plus a UI/control seam when
  the behavior is user-visible;
- unit, contract, integration, security, resource, failure-injection,
  migration/recovery, and deterministic replay tests pass as applicable;
- user/operator documentation, threat/capability notes, compatibility and
  migration notes, and known limitations are published;
- a signed or otherwise verifiable release artifact contains checksums,
  provenance, SBOM/notices, test/conformance results, resource profile, and
  rollback/recovery evidence;
- the canonical journey passes on the supported deployment presentation, and
  failures are diagnosable through retained redacted traces.

A phase may be marked partial only as an explicitly labeled development
milestone; it cannot be called production-complete or advance the release
slot.

## Consequences

- Small deterministic fixtures provide a stable release authority while real
  adapters can mature without hiding variability or requiring vendor accounts.
- The canonical journey exercises Hub, Agent, recovery, and editor seams in
  one private-first path, while focused suites still isolate failures.
- Optional ecosystems remain useful without turning the Rust installation
  into a large or untrusted bundle.
- Release work is evidence-heavy by design; a visually convincing UI or a
  passing happy-path demo cannot bypass contracts, security, recovery, or
  resource proof.

## Not permitted

This gate does not require arbitrary full n8n parity, copy of protected n8n
source/assets/trade dress, live production credentials, mutable third-party
branches, or resident model/agent/browser runtimes. It does not treat a
provider's uptime, popularity, or source license as proof of capability,
security, billing authorization, or compatibility.
