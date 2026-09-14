# Session State and Handoff

## Current state

Ticket 07 is complete, committed, pushed, release-tested, and deployed.

- Repository branch: `master`
- Local VPS commit: `e3580231cbf6246a1aefaf802c8898da5ca82005`
- `origin/master`: `e3580231cbf6246a1aefaf802c8898da5ca82005`
- Release manifest commit: `e3580231cbf6246a1aefaf802c8898da5ca82005`
- Installed systemd binary commit: `e3580231cbf6246a1aefaf802c8898da5ca82005`
- Last verified service state: `workflowd=active`
- Installed service is loopback-only at the repository default private-control port and reports `setup-required` after the final clean install.
- Final Ticket 07 commit message: `feat: stream generate items through artifacts`

The VPS Git working tree was clean after deployment. The complete workspace source mirror is `/home/user/workflow-rust`; local compilation is unavailable, so compile and test on the VPS.

## Active next work

Ticket 08 is next: `08-transform-items-with-edit-fields-and-the-safe-expression-vm.md`.

The local Ticket 08 document is in `owner-decision-round-1`. It contains recommended decisions 08-A through 08-D, but the Owner has **not explicitly approved them**. The decision UI was skipped twice. No Ticket 08 production implementation has begun.

Do not start production code until the Owner explicitly confirms shared understanding and authorizes implementation, per the Owner's standing instruction. The shortest valid confirmation is approval of the recommended 08-A through 08-D recorded in the ticket. If the Owner changes them, update the ticket first.

## Ticket 07 delivered behavior

Generate Items now:

- emits exactly 49,998 ordered Eco items from `count=49_998`, `start=0`, `step=1`, `data=null`;
- uses one logical Envelope per item with bounded physical micro-batches;
- limits micro-batches to 64 items / 256 KiB and the queue to 256 items / 4 MiB;
- applies cooperative backpressure rather than treating saturation as failure;
- retains progressive durable checkpoints with contiguous ordinal ranges;
- resumes only from the durable cursor and regenerates at most the speculative suffix;
- lets a durable cancellation win over late Artifact preparation or terminal completion;
- enforces checked arithmetic and hard item/logical/storage budgets;
- spills large or forced values into encrypted Owner-scoped Artifacts;
- preserves logical bytes, ordering, provenance, and digest independently of physical spill;
- supports Owner-authorized upload, metadata, preview, range, and full-content streaming;
- verifies ciphertext, AEAD tags, encrypted metadata, and plaintext digest before release;
- bounds preview/range plaintext and streams full content through a two-chunk mailbox;
- quarantines corruption evidence indefinitely and safe orphans for at least 24 hours;
- protects active leases, young content, and live references during startup/runtime cleanup;
- synchronizes source and destination directories around durable filesystem moves;
- shows generated progress, spill/backpressure evidence, and lazy Artifact detail in the editor.

## Ticket 07 review status

Final review result: approved with no remaining blocker.

The last review found one additional race after the earlier remediation: integrity-triggered quarantine did not use the same mutation barrier as reads/writes/cleanup. It was fixed by placing metadata/range/full verification behind the Artifact mutation barrier and adding `reads_and_integrity_quarantine_share_the_mutation_barrier`.

Earlier review blockers that were fixed:

1. progressive checkpoints now cover the complete delta from the durable cursor;
2. Artifact preparation failure honors concurrent durable cancellation;
3. preview/range retain only requested plaintext;
4. full content is integrity-checked before bounded HTTP streaming;
5. Artifact mutation, read/quarantine, and cleanup use the shared barrier;
6. expired staging, object, and quarantine reconciliation is crash-safe;
7. required directory syncs accompany placements, renames, and removals;
8. young, suspect, and referenced content remains protected.

## SSH state

- Replacement key path: `/home/user/.ssh/workflow_ticket07_ed25519`
- The key is valid and its public key is authorized exactly once as last verified.
- The old key's exact public-key line was not present when safe removal was attempted.
- Login with the old private key was independently verified to be rejected.
- Local old-key files were removed.
- Workspace restore has repeatedly changed the replacement private key mode from `0600` to `0644`; run `chmod 600` immediately before every SSH command.
- Use `bash /home/user/session-archive/vps-workflow-ssh` to apply the permission fix automatically.
- During archive export on 2026-09-13, new SSH connections repeatedly timed out during banner exchange. Last successful verification immediately beforehand showed `ssh=connected` and `workflowd=active`. Treat current remote-export outcome as temporarily unknown, not as a product/service failure.

Do not print, copy into prose, or commit private key material. Do not remove `authorized_keys` lines by position or truncation; any future removal must match the exact public-key algorithm and blob.

## Installed development tools on VPS

- Project Rust toolchain: 1.85.1
- Node: 22.19.0 under the user-local installation used by the Makefile
- Playwright: 1.62.1 with its managed Chromium installed for development-only browser testing
- Audit-only Rust toolchain: 1.88.0
- `cargo-audit`: 0.22.2

Do not reinstall Playwright Chromium or retry `cargo-audit 0.21.2`. The older audit tool cannot parse current CVSS 4.0 advisories.

## Important workspace paths

- `/home/user/AGENTS.md` — communication and automatic design-skill routing.
- `/home/user/arena-design-skills/` — all 13 curated design wrappers and router.
- `/home/user/workflow-rust/` — current source mirror and local Ticket 08 decision draft.
- `/home/user/workflow-rust/AGENTS.md` — repository-specific instructions.
- `/home/user/workflow-rust/CONTEXT.md` — canonical domain vocabulary.
- `/home/user/workflow-rust/docs/legal/clean-room-policy.md` — mandatory clean-room policy.
- `/home/user/workflow-rust/.scratch/eco-100k-first-runnable/issues/07-stream-generate-items-through-bounded-envelopes-and-artifacts.md` — Ticket 07 decisions/evidence.
- `/home/user/workflow-rust/docs/operations/generate-items-artifacts.md` — operator/API/recovery contract.
- `/home/user/workflow-rust/.scratch/eco-100k-first-runnable/issues/08-transform-items-with-edit-fields-and-the-safe-expression-vm.md` — active decision ticket.
- VPS repository: `/home/matt1/projects/workflow-rust`

## Archive status

This file is authoritative for the chat handoff. Exact recovery artifact presence and hashes are listed in `MANIFEST.sha256` after archive creation.
