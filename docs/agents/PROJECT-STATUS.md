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

1. Run `/setup-matt-pocock-skills` once and configure GitHub Issues plus the
   domain-document layout.
2. Use `/wayfinder` to resolve the open decisions in `CONTEXT.md`.
3. Turn the settled decisions into a spec, then vertical tickets.
4. Scaffold the smallest Dioxus/Leptos or backend/frontend slice selected by
   the decision record.

## Active work

- **Owner:** none
- **Ticket:** none claimed
- **Blockers:** framework and product-boundary decisions are still open

## Last verified

- `python3 -m unittest discover -s tests -v` — 3 tests passed before this
  planning update.
- `python3 tools/codebase_index.py index` — index refresh is available.
- `git diff --check` — clean at the last verification boundary.

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
