---
status: accepted
---
# Keep Hub package installation reviewable and scoped

Workflow Packages and Skill Packages are separate content-addressed distributions connected only by explicit immutable dependency locks. An unverified package may be inspected, imported into a Mutable Draft, and exercised in a no-credential/no-production-side-effect sandbox in that order; signature or popularity alone never grants execution authority. A Skill Package installed as a dependency defaults to Workflow-local scope, while the Owner may explicitly choose Global, Project, or Workflow scope and Local overrides Project overrides Global only through a visible lock change. Updates are side-by-side candidates that show graph, configuration, capability, dependency, compatibility, migration, and cost differences before a new reviewed Published Revision; existing Published Revisions, Runs, and Recovery Sets remain usable. Uninstall retires a package while any durable reference remains and deletes nothing still required by a revision, run, pin, or recovery set. Trust labels are evidence summaries, not permission grants.
