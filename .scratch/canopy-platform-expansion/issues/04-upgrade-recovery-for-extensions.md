# 04: Decide upgrade and recovery for Hub and Agent state

Type: wayfinder-decision
Status: blocked
Blocked by: 02 — Decide the Workflow and Skill Package lifecycle; 03 — Decide the AI Agent Node and Agent Engine contract
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/12

## Question

How do signed upgrades, migrations, Recovery Sets, Current/Previous Release Slots, and Quarantine Mode preserve Hub packages, Skill locks, Agent Blueprints, Model Routes, active/suspended Execution Plans, Artifacts, vault metadata, and optional worker identities?

Cover compatibility checks, admission/side-effect closure, pre-upgrade snapshots, candidate readiness, rollback before and after new traffic, lock/schema migration, stale package/engine handling, local-only versus off-site recovery, and evidence required to declare a release safe.

## Resolution

Pending the GitHub decision ticket. The result is an accepted upgrade/recovery contract and release-gate boundary that does not silently change active Run semantics.
