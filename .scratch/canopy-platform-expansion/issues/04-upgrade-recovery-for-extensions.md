# 04: Decide upgrade and recovery for Hub and Agent state

Type: wayfinder-decision
Status: resolved
Blocked by: None
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/12
Resolved: 2026-09-14
ADR: `docs/adr/0062-preserve-extension-state-across-verified-release-slots.md`
Spec: `docs/spec/recovery/release-and-extension-state.md`

## Question

How do signed upgrades, migrations, Recovery Sets, Current/Previous Release Slots, and Quarantine Mode preserve Hub packages, Skill locks, Agent Blueprints, Model Routes, active/suspended Execution Plans, Artifacts, vault metadata, and optional worker identities?

Cover compatibility checks, admission/side-effect closure, pre-upgrade snapshots, candidate readiness, rollback before and after new traffic, lock/schema migration, stale package/engine handling, local-only versus off-site recovery, and evidence required to declare a release safe.

## Resolution

The Owner selected the following #12 baseline:

- Use a verified Staging → Current/Previous slot transaction. Verify trust,
  compatibility, reserve, and a pre-upgrade Recovery Set; stop new admission
  and side-effect ingress; checkpoint or suspend work; migrate; boot privately;
  run readiness/no-side-effect checks; then switch slots.
- Allow automatic verified Previous-slot rollback only before new candidate
  traffic. After traffic, preserve newer state, close admission, enter
  Quarantine/Recovery, and require explicit reconciliation rather than
  destructively restoring old state.
- Recovery Sets are complete and incremental by digest: SQLite/migration
  ledger, referenced Artifacts, encrypted vault metadata, configuration and
  identity digests, exact package/skill/plan/Blueprint/route/worker locks, and
  integrity/restore metadata.
- Disaster-Recovery Ready requires an owner-held encrypted Recovery Kit, recent
  encrypted off-site set, verified manifest, and an isolated restore drill with
  triggers, credentials, ingress, and side effects disabled. Local-only
  operation is reported honestly as not host-loss ready.
- Stale or incompatible packages, skills, engines, routes, and workers are
  preserved as evidence and cannot silently receive authority or replace the
  identities of active Runs.

ADR 0062 and the recovery spec record the accepted contract. No upgrade or
recovery implementation was started in this decision step.
