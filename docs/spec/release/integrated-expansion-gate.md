# Integrated expansion acceptance and release gate

**Contract:** `canopy.release.expansion-gate/v1alpha1`
**Decision:** ADR 0063

## Canonical private-first journey

```text
boot signed Rust daemon/editor in default profile
  -> Draft with Workflow/Skill Package locks
  -> publish immutable Revision + Agent Blueprint
  -> resolve scopes, grants, leases, route, policy, and output contract
  -> deterministic Agent Turn fixture
       memory reference -> safe tool -> pre-side-effect fallback
       -> bounded repair -> validated Output -> Artifact -> downstream node
  -> inject suspension/cancel/duplicate/timeout/uncertain/budget/restart cases
  -> complete Recovery Set + isolated no-side-effect Restore Drill
  -> signed Staging upgrade before traffic, verify Current/Previous behavior
  -> inspect browser/control/accessibility and retained evidence
```

The journey never requires public ingress, production credentials, irreversible
writes, hosted providers, or resident heavy runtimes.

## Evidence lanes

### Deterministic contract lane

This lane is the release authority. It uses fixed fixtures/fake engines,
versioned schemas, bounded inputs, content-addressed expected outputs,
resource profiles, failure injection, replay, migration fixtures, and
cryptographic lock checks. It proves ordering, capability enforcement, hard
budgets, typed outcomes, Output Contracts, idempotency, recovery, and UI/control
visibility.

### External conformance lane

This lane records real provider, MCP, A2A, package, and agent-adapter results
without replacing deterministic proof. Each result pins source/release,
license/notices, capability and network/auth mode, cost/latency/resources,
stream normalization, cancellation, errors/uncertainty, and fixture version.
Variability and quarantine state remain visible.

## Resource and trust checks

The default Rust profile must stay within the existing Eco resource and
security envelope. Optional GPU, browser, hosted model, MCP/A2A, CLI, Node,
Python, and other heavy workers are remote-first/local-opt-in. A worker needs
an immutable identity, capability manifest, lane, provenance, license/notice
evidence, budget, compatibility result, and explicit grant before use.

## Phase completion

A production-complete phase has:

- production implementation, not a placeholder or demo path;
- versioned contract and user-visible UI/control seam where applicable;
- applicable unit, contract, integration, security, resource,
  failure-injection, migration/recovery, and deterministic replay tests;
- user/operator, threat/capability, compatibility/migration documentation;
- verifiable release artifact with checksums, provenance, SBOM/notices,
  conformance results, resource profile, and rollback/recovery evidence;
- passing canonical journey and diagnosable redacted traces.

A development milestone may be labeled partial, but it cannot be declared
production-complete or advance the signed release slot.
