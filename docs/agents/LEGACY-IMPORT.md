# Legacy workspace recovery

## Source

On 2026-09-13 the four branch files
`workspace-split.zip.001.pdf` through `.004.pdf` were concatenated in numeric
order. The resulting outer ZIP and its inner workspace ZIP both passed
`unzip -t`.

The inner workspace contained a prior `workflow-rust` project. Its source and
safe project records were merged into this repository at commit history after
`74f8709`, without replacing the current Arena branch or creating a new branch.

## Imported

- Rust workspace, lockfile, toolchain pin, contracts, crates, and editor;
- `.scratch` maps and dependency-ordered tickets;
- 56 ADRs and legal, operations, security, and specification documents;
- acceptance tests, release scripts, systemd packaging, and SDK contract files;
- safe historical handoff/evidence documents under
  `docs/legacy/session-archive/`;
- current `AGENTS.md`, `CONTEXT.md`, GitHub tracker rules, and project status
  were merged rather than overwritten.

## Deliberately excluded

The recovery archive also contained historical SSH private-key material and
other session-only artifacts. No private key, SSH wrapper, credential, token,
`.ssh` directory, VPS bundle, or recovery tarball was imported into the source
repository. The uploaded split archive parts themselves remain in the branch as
user-provided provenance files, but the reconstructed ZIP is ignored and must
not be committed.

If any historical credential was ever exposed outside its intended owner
scope, rotate or revoke it before using the recovered deployment path.

## Verification boundary

The archive reconstruction was verified. The historical session's Ticket 07
build, browser, release, systemd, and VPS results are preserved as historical
evidence only. This checkout has not re-run those gates because the local
sandbox does not currently provide `cargo` or the pinned browser/toolchain
stack.
