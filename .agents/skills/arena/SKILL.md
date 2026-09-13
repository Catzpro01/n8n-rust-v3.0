---
name: arena
description: >-
  Always-on repository workflow for this project. Combine the codebase-memory-mcp
  discovery skill with the smallest relevant Matt Pocock engineering skill,
  using the local cache.kv index first to keep context small and progress fast.
---

# Arena repository workflow

Use this skill at the start of every task in this repository. It is a thin
router, not a replacement for the two installed skill families.

## Required two-layer workflow

### 1. Codebase discovery: `codebase-memory-mcp`

Use the configured Codebase Memory tools when they are available:

1. Call `list_projects` first and select only the project whose canonical
   `root_path` is the live checkout.
2. Check `index_status` before conclusions that depend on branch state or
   freshness.
3. Use `get_architecture` once only when the repository or area is unfamiliar.
4. Use bounded `search_graph` and `trace_path` for symbols, relationships,
   callers, callees, and impact. Use exact local search for literals/config.
5. Verify graph results against local source before editing or making strong
   claims. Keep result limits small and finish pagination only for exhaustive
   claims.

If Codebase Memory is unavailable, use the local incremental index instead:

```bash
python3 tools/codebase_index.py index
python3 tools/codebase_index.py search "<task terms>" --limit 6 --no-index
```

The local index is a fast lexical/symbol fallback, not proof of graph
completeness. It intentionally skips `.agents`, dependencies, and build output.

### 2. Engineering discipline: the smallest relevant Matt skill

Do not load all Matt Pocock skill files on every turn. Select the smallest
skill that matches the task; use `ROUTING.md` when the choice is unclear.

- behavior change or bug fix: `tdd` (red → green → refactor)
- module/interface/design decision: `codebase-design`
- ambiguous plan or requirements: `grill-me` or `grilling`
- any bug, test failure, or unexpected behavior: `systematic-debugging`
- difficult bug/performance issue after initial evidence: `diagnosing-bugs`
- final diff review: `code-review`
- before any completion/PR claim: `verification-before-completion`
- repository skill configuration: `setup-matt-pocock-skills`
- documentation for agents: `writing-for-agents`

For code changes, consult `codebase-design` before changing a seam, apply
`tdd` when behavior changes, and run `code-review` before handoff when the diff
is non-trivial. For a known one-file edit, keep discovery narrow rather than
running a repository-wide scan.

The two lightweight Superpowers additions are guardrails, not a second full
workflow: use `systematic-debugging` only when there is a failure or surprise,
and use `verification-before-completion` once with fresh command output before
claiming completion, committing, or opening a PR.

## Token and latency rules

- Start with `arena_context.py` or the index search; do not dump the repository.
- Ask for the smallest useful result set: usually 5–6 files and short snippets.
- Read source only after search identifies a candidate path.
- Do not load companion references unless the selected Matt skill points to one
  needed for the current decision.
- Reuse `cache.kv`; refresh incrementally. Use `--force` only after a metadata
  or indexing-version problem.
- Keep graph discovery and source verification separate so one tool response
  does not flood the context with duplicate code.
- Never treat the cache or graph as the source of truth; checked-out source is
  authoritative.

## Multi-agent and cross-session continuity

The repository is intended to be worked on by multiple agents. Before a large
or resumed task, read `docs/agents/PROJECT-STATUS.md` and `CONTEXT.md` after
this skill. Use `docs/agents/COLLABORATION.md` for the writer/reviewer protocol
and `docs/agents/HANDOFF-TEMPLATE.md` at a context boundary.

For a project larger than one session, `/ask-matt` should route to:

1. `/setup-matt-pocock-skills` once, so the issue tracker and domain-doc layout
   are explicit;
2. `/wayfinder` to map unresolved architecture/product decisions;
3. `/to-spec`, then `/to-tickets`, when the way is clear;
4. one fresh `/implement` session per vertical ticket;
5. `/code-review` and `verification-before-completion` before integration.

Do not carry the entire conversation into the next session. Carry only the
status, ticket, decisions, evidence, blockers, and next action as a handoff.

## Completion checklist

1. Confirm the scope and current Git state.
2. Read the durable status/context and run the discovery layer.
3. Select the smallest Matt skill and any applicable guardrail.
4. Make the smallest coherent change.
5. Run focused tests/checks and `git diff --check`.
6. Refresh `cache.kv` if source paths or symbols changed.
7. Update durable status or write a handoff before changing sessions.
8. Report what was indexed, what was verified locally, and any unavailable MCP
   capability instead of inventing results.
