# Persistent Arena Session Archive

Created for the Owner on 2026-09-13 (user-local date) so the project can be resumed without relying on chat context alone.

## Start here

1. Read `SESSION_STATE.md` for the authoritative handoff.
2. Read `USER_INSTRUCTIONS.md` before planning or coding.
3. Read `DECISIONS.md` for frozen product/architecture choices.
4. Read `VERIFICATION.md` for exact Ticket 07 evidence.
5. Read `CONTINUATION.md` before beginning Ticket 08.
6. Read `SKILLS.md` for the installed design-skill router.
7. The active source tree is `/home/user/workflow-rust`.

## Recovery artifacts

- `workflow-rust-session-tree.tar.gz` — workspace source snapshot, including the uncommitted local Ticket 08 decision draft.
- `arena-design-skills-current.tar.gz` — complete curated design-skill pack.
- `workflow-rust-session-snapshot.bundle` — restorable synthetic Git snapshot of the complete workspace source, labeled with deployed baseline `e358023…` and including the local Ticket 08 decision draft.
- `workflow-rust-e358023.bundle` — original Git history through Ticket 07, if a later VPS export succeeds.
- `ticket07-release-e358023.tar.gz` — final clean Ticket 07 Production Bundle, if a later VPS export succeeds.
- `legacy-progress-files.tar.gz` — earlier ticket bundles, patches, prototypes, screenshots, and research files that were already in the workspace.
- `CONTENT_HASHES.sha256` — per-file hashes for active source and skill files.
- `MANIFEST.sha256` — checksums for archive files.

The original Git-history and final release-tar exports were unavailable during archive creation because the VPS repeatedly timed out during SSH banner exchange. This does not affect the source snapshot or recorded test/release evidence. `ARCHIVE_STATUS.md` records exact presence.

## Portable copy

`/home/user/arena-session-progress-2026-09-13.tar.gz` is a single portable archive of this entire handoff directory and nested source/skill/progress snapshots. Its adjacent `.sha256` file verifies it. SSH private keys are deliberately excluded.

## Security

No private key, password, recovery phrase, session cookie, CSRF token, wrapped key, nonce, or server-side master key is copied into these documents or archives. The existing SSH private key remains only at `/home/user/.ssh/workflow_ticket07_ed25519`. Always set its mode to `0600` immediately before SSH because workspace restoration has been observed to reset it to `0644`.
