# Workflow Hub installation and updates

The local index federates configured Hub Sources. Installation resolves a Workflow Package or Skill Package to immutable source identifiers, verifies evidence and policy, previews all requirements, imports a Mutable Draft or explicit Skill scope, runs fixtures in a sandbox without credentials or production side effects, and waits for capability/credential mapping and explicit publication.

A Skill dependency defaults to Workflow-local scope; Global, Project, and Workflow scope are explicit and visible, with Local-over-Project-over-Global shadowing only through a lock change. Trust labels summarize evidence and never grant authority.

Updates are side-by-side candidates. The editor shows graph, configuration, permission, capability, dependency, compatibility, migration, and expected-cost differences. Existing Published Revisions, Runs, and Recovery Sets remain runnable and rollbackable until the Owner publishes a reviewed successor. Uninstall retires a package while any durable Draft, Revision, Run, Evidence Pin, or Recovery Set refers to it; required evidence is not deleted.
