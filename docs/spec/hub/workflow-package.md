# Workflow Package

A Workflow Package is a declarative, content-addressed distribution separate from a Skill Package. It may carry Workflow Revisions, display metadata, screenshots, fixture data, contract tests, compatibility requirements, migrations, locked Node Definitions, immutable Skill Package and MCP requirements, permission and cost manifests, license records, checksums, SBOMs, and provenance attestations.

It may not contain credential values or require an installer to execute merely to inspect the package. An unverified package may be inspected, imported into a Mutable Draft, and exercised in a no-credential/no-production-side-effect sandbox in that order. Executable artifacts remain quarantined until their source, digest, attestation, requested capabilities, and sandbox evidence satisfy local policy. Installation never changes a Published Revision automatically.
