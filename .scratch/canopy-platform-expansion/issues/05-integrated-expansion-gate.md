# 05: Define the integrated expansion acceptance and release gate

Type: wayfinder-decision
Status: resolved
Blocked by: None
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/13
Resolved: 2026-09-14
ADR: `docs/adr/0063-use-a-private-first-dual-lane-expansion-gate.md`
Spec: `docs/spec/release/integrated-expansion-gate.md`

## Question

What single integrated acceptance journey and release gate proves that the completed core, extension contracts, Workflow/Skill Hub, AI Agent Node, upgrades, recovery, compatibility, and editor surfaces work together inside the Eco resource and security envelope?

Settle the smallest highest stable public seam, fixtures, deterministic versus nondeterministic evidence, resource profiles, browser/accessibility coverage, package/agent trust evidence, failure injection, upgrade/recovery drills, and the definition of production-complete for each phase.

## Resolution

The Owner selected the following #13 baseline:

- Use one private-first vertical journey: boot the signed Rust daemon/editor;
  Draft and publish Workflow/Skill locks plus an Agent Blueprint; run a bounded
  deterministic Agent fixture through memory, safe tool, fallback, validation,
  Artifact, and downstream delivery; inject suspension/cancel/duplicate/timeout/
  uncertain/budget/restart cases; restore; exercise signed pre-traffic
  upgrade; and verify browser/control/accessibility evidence.
- Keep two evidence lanes. Deterministic fixtures, schemas, replay, budgets,
  and failure injection are the release authority. Real provider, MCP, A2A,
  package, and agent results are separately pinned conformance evidence and do
  not weaken deterministic guarantees.
- Preserve the Rust default Eco profile. Heavy runtimes, GPU, providers,
  browsers, and external agents remain remote-first/local-opt-in with pinned
  identity, capabilities, license/provenance, cost/resource budget, and
  explicit grants.
- A phase is production-complete only with production behavior, versioned
  contracts/UI seams where applicable, applicable tests, documentation, and
  verifiable release/recovery evidence. A demo or placeholder is not complete.

ADR 0063 and the integrated gate spec record the accepted release boundary.
The decision map is now complete; implementation follows the approved phase
order, beginning with core completion and preserving the release gate.
