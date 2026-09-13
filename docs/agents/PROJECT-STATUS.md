# Project status

**Last updated:** 2026-09-13
**Stage:** recovered implementation baseline; Ticket 08 implementation
**Current branch:** `arena/01a09a2a-n8n-rust-v3-0`
**Current PR:** [#1](https://github.com/Catzpro01/n8n-rust-v3.0/pull/1)
**Recovered source baseline:** Canopy Workbench / `workflow-rust`

## Destination

Continue the original Rust workflow-automation product rather than restarting
from the earlier empty scaffold. The target is an independently authored,
self-hosted workflow platform with a browser editor, durable revisions,
reliable execution, public compatibility seams, and a small-resource default
profile.

## Current truth

- The four uploaded split parts are present in the branch as
  `workspace-split.zip.001.pdf` through `.004.pdf`.
- Concatenating the parts produced a valid outer ZIP; its inner workspace ZIP
  also passed `unzip -t`.
- The recovered `workflow-rust` implementation has been merged into the
  repository root, including Rust crates, Preact editor, contracts, ADRs,
  operations/security/spec docs, `.scratch` tickets, acceptance tests, and
  release packaging.
- Historical session records are preserved under
  `docs/legacy/session-archive/`. They are evidence, not freshly rerun checks.
- Historical `.ssh` material and other recovery-only secrets were intentionally
  not imported.
- The local sandbox has Node/npm/Python but no `cargo`; Rust and browser gates
  have not been rerun in this checkout.
- The current context preserves both the earlier Arena decisions and the
  recovered project's canonical glossary and ADR decisions.

## Retained implementation frontier

Tickets 01–07 in `.scratch/eco-100k-first-runnable/` are recorded as complete
by the recovered session evidence. Ticket 08 is now authorized and its scope
is frozen by `docs/adr/0057-approve-edit-fields-and-safe-expression-vm.md`.

- **Ticket 08:** `08-transform-items-with-edit-fields-and-the-safe-expression-vm.md`
- **Status:** `approved-for-implementation`
- **Next action:** resolve the frozen Eco label's numeric-index/string-
  concatenation inconsistency, then wire transformed Envelopes into the
  durable Run scheduler.

The earlier Wayfinder selection of “architecture spike first” now means a
reversible audit/reconciliation of the recovered Rust + connected Preact
baseline. It does not authorize throwing away the existing implementation or
starting a new frontend framework from scratch.

## Active work

- **Owner:** none
- **Ticket:** Ticket 08 is the current frontier and is authorized for
  implementation within ADR 0057.
- **Blockers:** the pinned CI environment compiles the pure seam, but exposes
  a frozen 08-A/08-C type conflict for the Eco label; durable Run integration
  and public-seam tests remain after that decision.

## Last verified in this checkout

- Combined four-part archive: `unzip -t` passed for the outer and inner ZIPs.
- `python3 tools/codebase_index.py index` — 218 documents indexed.
- `python3 -m unittest discover -s tests -v` — 3 indexer tests passed after
  the recovery and Ticket 08 foundation changes.
- `node --check` passed for the changed editor build script and browser test.
- JSON/Python metadata checks passed for the changed contract/release paths.
- Pinned GitHub workflow run `34757520909` passed editor build and Rust
  formatting/compilation, then failed two focused Eco label tests because of
  the recorded mixed string/number operation.
- `git diff --check` — clean before the current Ticket 08 progress update.
- The historical Ticket 07 full gate is recorded in
  `docs/legacy/session-archive/VERIFICATION.md`; it has not been re-claimed as
  a fresh local result.

## Resume protocol

A new agent should read, in order:

1. `AGENTS.md`
2. `.agents/skills/arena/SKILL.md`
3. this file
4. `CONTEXT.md`
5. `docs/legal/clean-room-policy.md`
6. `.scratch/eco-100k-first-runnable/map.md`
7. the active ticket and relevant ADRs
8. only then the relevant source paths from `cache.kv` or Codebase Memory

Do not paste recovery secrets or the entire archive into a handoff. Update this
file with durable state, decisions, ownership, blockers, and fresh evidence.
