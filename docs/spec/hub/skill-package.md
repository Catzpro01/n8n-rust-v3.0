# Skill Package

A Skill Package is a separately locked, content-addressed distribution of a
Skill Set. Its manifest records the source commit and subpath, scope, required
capabilities, tests, license, provenance, dependency locks, and trust evidence;
it contains no credential values and never follows a mutable branch implicitly.

## Scope

An Owner chooses Global, Project, or Workflow scope during installation. A
Workflow Package dependency defaults to Workflow-local scope. Effective
shadowing is explicit and ordered Local, then Project, then Global; the chosen
lock and any override are visible in the Agent Blueprint/Workflow Revision.

## Safe lifecycle

An unverified Skill Package may be inspected, imported into a Draft, and run in
a sandbox without credentials or production side effects. Credential mapping,
capability grants, and production use require separate policy checks and
explicit publication. Trust labels summarize evidence and never replace those
checks.

Updates are side-by-side candidates. They show source, capability, dependency,
license, test, and behavior differences and become effective only through a
new reviewed lock and Published Revision. A package is retired rather than
deleted while any Draft, Published Revision, Run, Evidence Pin, or Recovery Set
still references it.
