# 02: Decide the Workflow and Skill Package lifecycle

Type: wayfinder-decision
Status: blocked
Blocked by: 01 — Decide the Node Form and Execution Lane extension contract
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/10

## Question

What is the complete safe lifecycle for Workflow Packages and Skill Packages across Hub Source indexing, search, trust evidence, verification, sandboxing, Draft import, credential mapping, publication, update, rollback, uninstall, and local/remote execution?

Settle package identities and locks, Skill Set scope/shadowing, trust labels, capability/permission manifests, version and migration handling, failure/quarantine behavior, and the rule that installed packages never silently mutate a Published Revision.

## Resolution

Pending the GitHub decision ticket. The result is an accepted package/lifecycle contract with API, UI, security, and fixture boundaries for a vertical implementation ticket.
