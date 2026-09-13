# Archive Status

## Present

- Complete source tree snapshot: `workflow-rust-session-tree.tar.gz`
- Restorable synthetic Git source snapshot: `workflow-rust-session-snapshot.bundle`
- Complete curated design skills: `arena-design-skills-current.tar.gz`
- Earlier workspace progress and intermediates: `legacy-progress-files.tar.gz`
- Human handoff documents: present
- Active source tree at `/home/user/workflow-rust`: present
- Skill tree at `/home/user/arena-design-skills`: present
- Replacement SSH private-key file: present only at its original private path; deliberately not copied

## Not copied during this archive operation

- Original-history Git bundle `workflow-rust-e358023.bundle`
- Final release bundle `ticket07-release-e358023.tar.gz`

Reason: repeated SSH banner-exchange timeout from the VPS during export. The latest prior direct verification had confirmed SSH connectivity and `workflowd=active`; the timeout began during this archive operation. Do not infer that the service or data failed.

The complete source is nevertheless recoverable from the source tarball or synthetic Git bundle. Exact release identity, binary size, test output, manifest commit, and systemd evidence are preserved in `VERIFICATION.md`.

## Future completion command

When SSH accepts connections again, use the wrapper and export the two optional artifacts. Always repair the private-key mode in the same shell. The source of truth on the VPS is `/home/matt1/projects/workflow-rust` at expected baseline `e3580231cbf6246a1aefaf802c8898da5ca82005`.
