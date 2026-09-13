# Project status

**Last updated:** 2026-09-13
**Stage:** foundation and planning; Rust application scaffolding has not started
**Current branch:** `arena/01a09a2a-n8n-rust-v3-0`
**Current PR:** [#1](https://github.com/Catzpro01/n8n-rust-v3.0/pull/1)

## Destination

A testable Rust workflow-automation product with a browser node editor and a
small end-to-end execution path. The first milestone is not feature parity
with n8n; it is one demonstrable vertical slice.

## Current truth

- Project-scoped skills are installed under `.agents/skills/`.
- Arena routing is defined in `.agents/skills/arena/` and required by
  `AGENTS.md`.
- `cache.kv` contains the local incremental discovery index. It is not project
  memory and must not be treated as a source of truth.
- The current repository contains tooling and planning artifacts, not the Rust
  workflow engine yet.
- The current sandbox does not provide `cargo`; GitHub Actions or a Rust-enabled
  development environment will be the first Rust build verifier.

## Frontier

1. Resolve the product boundary and clean-room compatibility target in
   [#3](https://github.com/Catzpro01/n8n-rust-v3.0/issues/3), the first
   decision frontier in the [Wayfinder map #2](https://github.com/Catzpro01/n8n-rust-v3.0/issues/2).
2. Use the resolved boundary to evaluate the browser/UI architecture (#4) and
   workflow domain/execution semantics (#5).
3. Resolve persistence/security (#6) and GitHub validation/delivery (#7) only
   after their stated blockers are settled.
4. Turn the settled decisions into an ADR-backed spec and then vertical
   implementation tickets.
5. Scaffold the smallest selected Rust slice only after the relevant decision
   records are green.

## Active work

- **Owner:** none
- **Ticket:** [#3](https://github.com/Catzpro01/n8n-rust-v3.0/issues/3) is ready for
  agent work but is not claimed.
- **Blockers:** product boundary and clean-room acceptance criteria; framework,
  domain, persistence, and delivery decisions remain downstream.

## Last verified

- `python3 -m unittest discover -s tests -v` — 3 tests passed.
- `python3 tools/codebase_index.py index` — 15 files indexed.
- `git diff --check` — clean before the Wayfinder issue update.
- Latest pushed commit: `283360b` (`docs: configure GitHub issue workflow`).

## Resume protocol

A new agent should read, in order:

1. `AGENTS.md`
2. `.agents/skills/arena/SKILL.md`
3. this file
4. `CONTEXT.md`
5. the active ticket or PR
6. only then the relevant source paths from `cache.kv` or Codebase Memory

Update this file only with durable state: decisions, ownership, blockers,
last-green commit, and the next verifiable step. Do not paste conversation
transcripts here.
