# 01: Decide the Node Form and Execution Lane extension contract

Type: wayfinder-decision
Status: resolved
Blocked by: None
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/9
Resolved: 2026-09-13
Decision record: `docs/adr/0059-language-bindings-behind-contract-and-lane-boundary.md`

## Question

What is the exact production contract connecting the current Rust Native Node/Execution Plan model to all recorded Node Forms, Execution Lanes, Agent Blueprints, and optional local/remote workers without weakening deterministic Runs, capability boundaries, resource budgets, or clean-room compatibility?

## Answer

The Owner approved the following contract:

- Node Contract, Node Implementation, Node Form, and Execution Lane stay separate. One observable Node Contract may have multiple executable bindings.
- The compiler chooses the cheapest valid Execution Lane. An Owner may restrict a node only toward a safer/more isolated lane; policy and capability boundaries cannot be bypassed.
- Rust Native is the default in-process Promotion target, but only after reproducible build, signature, dependency, resource, conformance, and soak evidence. Custom Rust first runs isolated.
- C++ and other practical languages are supported through the common versioned framed External Process protocol. Initial official SDKs target Rust, C++, Python, and JavaScript; other languages may use the protocol or WASM boundary. No separate Node Form is invented for every language.
- External process messages carry bounded logical activation data, Artifact references for large values, Capability Grants/Secret Lease handles, cancellation/deadline, output ports, trace facts, and typed outcomes. The public boundary is not a Rust ABI.
- Execution Plans pin the exact contract, implementation, selected lane, worker/runtime identity, capability policy, and resource budget. Implementation/worker changes require a new reviewed revision/plan; active Runs are never silently changed.
- Builds happen outside the production daemon and packages carry source/build/dependency/provenance evidence. Unverified implementations remain isolated and do not gain authority from popularity alone.

This preserves the accepted deterministic Rust core and existing four-lane model while opening the platform to C++, Rust, Python, JavaScript, WASM, and other languages without making every runtime resident or trusted.

## Consequences

The core engine needs a versioned language-neutral adapter protocol and plan lock extensions. Rust has the fastest trusted path; C++ and other languages retain full node capability through isolation. Promotion is deliberately narrower than installation, and active Runs remain reproducible across worker updates.
