---
status: accepted
owner_decision: 2026-09-14
---
# Preserve extension state across verified release slots

## Context

ADR 0055 already defines the Rust-first signed Production Bundle, immutable
Current/Previous release slots, protected Recovery Reserve, Quarantine Mode,
and complete Recovery Sets. Hub Packages, Skill locks, Agent Blueprints,
Model Routes, active/suspended Runs, Artifacts, vault metadata, and optional
worker identities now need an explicit extension-state upgrade boundary.

The Owner selected these #12 policies through HITL on 2026-09-14:

- verified Current/Previous slot transaction with admission and side-effect
  closure before migration;
- no destructive automatic rollback after a candidate has admitted new traffic;
- complete incremental Recovery Sets rather than database-only or runtime-image
  snapshots;
- Recovery Kit, encrypted off-site copy, and isolated restore drills as the
  threshold for Disaster-Recovery Ready.

## Decision

### 1. Candidate admission is a verified slot transaction

A release candidate is first placed in an immutable Staging slot and verified
against the Release Trust Root. Admission checks include bundle completeness,
signature/anti-rollback policy, SBOM/notices/provenance, schema and migration
compatibility, Recovery Reserve, and the ability to interpret every active or
suspended Execution Plan, Workflow/Skill Package lock, Agent Blueprint, Model
Route, Output Contract, Artifact reference, and optional-worker identity.

Before migration or activation, the daemon:

1. stops new Run Admission and closes public side-effect ingress;
2. allows safe work to finish or creates durable checkpoints/suspensions;
3. creates and verifies a pre-upgrade Recovery Set;
4. runs checksummed migration preflight and controlled migration;
5. boots Staging privately with no production traffic;
6. runs database, Artifact, vault-metadata, plan/Blueprint-load, API/editor,
   and synthetic no-side-effect readiness checks;
7. switches Staging to Current only after all checks pass, retaining the old
   Current as Previous.

An update cannot proceed when a required backup, compatibility check, reserve,
readiness test, or Recovery Kit validation fails. Active Runs retain the exact
plan/Blueprint/lock digests they started with; the candidate cannot silently
rewrite their semantics.

### 2. Rollback has a traffic boundary

Before the candidate admits new production traffic, a startup, migration, or
readiness failure may automatically stop the candidate, restore the verified
pre-upgrade Recovery Set when needed, and reactivate Previous.

After new traffic has been admitted, automatic destructive rollback is
forbidden. The system closes admission and side effects, preserves the newer
state and trace as evidence, enters Quarantine or an explicit Recovery flow,
and presents compatibility, data-loss, and reconciliation choices for owner
approval. A Previous slot may be started read-only or in a controlled recovery
mode, but new state is never silently discarded to make the health check green.

### 3. Recovery Set contents and identity

A complete Recovery Set contains a consistent SQLite snapshot and migration
ledger; every referenced immutable Artifact root; encrypted vault metadata but
never plaintext credentials; redacted configuration and instance identity
digests; and the exact release, compiler/Execution Plan, Node Contract,
Compatibility Profile, Workflow/Skill locks, Agent Blueprint/Model Route locks,
and optional-worker identities needed to interpret the state. Its signed or
hashed manifest records retention, integrity, restore-tool, schema, and
migration metadata.

Immutable objects are copied incrementally by digest and pinned during
collection. A set is complete only after every referenced object verifies. The
set records optional worker manifests and trusted source/release identities,
not unreviewed binaries or provider secrets. Restored workers must pass the
same admission/conformance policy before they can receive a Capability Grant.

Stale or incompatible packages, skills, engines, routes, or workers are
preserved as references and evidence but are not silently substituted. A Run
may continue with its locked compatible implementation, suspend awaiting an
approved migration, or enter a typed failure/uncertain state according to its
contract.

### 4. Recovery readiness and drills

Local verified Recovery Sets support crash and local-corruption recovery. The
installation is **Disaster-Recovery Ready** only when all of the following are
true:

- a recent encrypted off-site Recovery Set has a verified manifest and all
  referenced roots;
- the owner holds a separately encrypted Recovery Kit for release trust,
  instance identity, and restored vault metadata;
- an isolated Restore Drill has opened and checked the SQLite/migration
  history, Artifacts, vault metadata, release/plan/contract/Blueprint locks,
  and output digests;
- the drill ran with triggers, schedules, public ingress, credentials, and
  external side effects disabled, and produced retained evidence;
- the declared local and host-loss recovery objectives are still within age.

A local-only installation remains usable but reports that it is not
Disaster-Recovery Ready. Recovery Kit material, credentials, and tokens are
never committed to Git or stored only beside the state they protect.

### 5. Quarantine and side-effect safety

Suspected database, Artifact, vault, migration, lock, or package-integrity
corruption stops writes, Run Admission, triggers, schedules, and side effects.
The system preserves suspect files as evidence, runs non-destructive
assessment, lists verified Recovery Sets and estimated loss, and requires
owner approval before replacement.

Restore drills and candidate readiness are read-only/no-side-effect exercises.
External Agent, MCP, A2A, model, browser, and package workers remain optional,
remote-first/local-opt-in components. Release state stores their approved
manifests and lock identities, not resident runtimes or plaintext credentials.

## Consequences

- Upgrade safety is measurable before and after the traffic boundary instead
  of relying on a database copy or process health alone.
- Newer post-upgrade data is preserved for reconciliation, at the cost of a
  deliberate manual recovery path after traffic has opened.
- Hub and Agent state can be interpreted after restore because package/skill/
  Blueprint/route/worker identities are part of the manifest.
- The default Rust daemon remains light; off-site transfer, Recovery Kit
  custody, and Restore Drills are explicit operational capabilities.
- Upgrade implementation must include migration, slot, reserve, quarantine,
  artifact-integrity, lock-compatibility, and drill evidence before release.
