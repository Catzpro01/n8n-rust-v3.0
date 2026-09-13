# Workspace session records

This directory keeps small, safe handoff records for agents working in this repository.

- `current/` — the active session's durable state and append-only progress log.
- `previous/` — the sanitized handoff from the session before the current one.
- `AGENT-GUIDE.md` — the required update procedure.

These files are navigation and handoff records, not a replacement for source,
ADRs, tickets, tests, or Git history. The repository's code, `CONTEXT.md`,
`docs/adr/`, `.scratch/` maps/tickets, and verified CI are authoritative.

Do not copy credentials, private keys, tokens, cookies, recovery material,
server secrets, Artifact paths, or source archives into this directory. Record
only safe relative paths, decisions, commit IDs, issue URLs, test commands,
results, blockers, and next actions.
