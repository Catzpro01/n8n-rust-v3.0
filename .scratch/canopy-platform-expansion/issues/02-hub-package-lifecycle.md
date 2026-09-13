# 02: Decide the Workflow and Skill Package lifecycle

Type: wayfinder-decision
Status: resolved
Blocked by: 01 — Decide the Node Form and Execution Lane extension contract
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/10
Resolved: 2026-09-13
Decision record: `docs/adr/0060-keep-hub-package-installation-reviewable-and-scoped.md`

## Question

What is the complete safe lifecycle for Workflow Packages and Skill Packages across Hub Source indexing, search, trust evidence, verification, sandboxing, Draft import, credential mapping, publication, update, rollback, uninstall, and local/remote execution?

## Answer

The Owner approved the following lifecycle:

- Workflow Packages and Skill Packages are separate content-addressed distributions connected by explicit immutable dependency locks. A Workflow Package may reference pinned Node, Skill, and MCP requirements but does not silently embed mutable skill branches.
- An unverified package may be inspected, imported into a Mutable Draft, and exercised in a no-credential/no-production-side-effect sandbox in that order. Signature or popularity alone never grants execution authority.
- Skill installation uses explicit Global, Project, or Workflow scope. A Skill Package required by a Workflow Package defaults to Workflow-local. Effective shadowing is visible and ordered Local, then Project, then Global; changes require a lock change.
- Credential mapping, Capability Grants, and production use require separate policy checks and explicit publication.
- Updates are side-by-side candidates showing graph, configuration, capability, dependency, compatibility, migration, and cost differences. They become effective only through a new reviewed Published Revision; existing Published Revisions, Runs, and Recovery Sets remain usable.
- Uninstall retires a package while any Draft, Published Revision, Run, Evidence Pin, or Recovery Set references it. Required evidence and Artifacts are not deleted.
- Local, Curated, Verified, and Unverified trust labels summarize evidence; they never replace capability policy or Owner approval.

## Consequences

Hub implementation needs separate Workflow/Skill manifests, immutable dependency locks, Draft import and sandbox states, explicit scope/override records, candidate diffing, retirement/reference tracking, and non-destructive update/recovery behavior. The existing accepted Hub/Skill ADRs remain valid and this decision supplies their integrated lifecycle boundary.
