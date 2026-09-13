# Agent guidance

For every task in this repository, start with the project Arena skill:
`.agents/skills/arena/SKILL.md`.

The Arena workflow requires two core layers:

1. use `codebase-memory-mcp` for bounded codebase discovery, falling back to
   `python3 tools/codebase_index.py` and `cache.kv` when the MCP tools are not
   available;
2. use the smallest relevant skill from the Matt Pocock collection, following
   `.agents/skills/arena/ROUTING.md`.

For the coding loop, use the two lean guardrails from Superpowers when they
apply: `systematic-debugging` for failures/surprises and
`verification-before-completion` before completion, commits, or PRs.

## Agent skills

### Issue tracker

GitHub Issues through `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and
`wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: `CONTEXT.md` plus `docs/adr/`. See
`docs/agents/domain.md`.

Keep context efficient: search first, read only candidate files, use focused
limits, and verify important graph/index findings against checked-out source.
Run focused checks and refresh the index after source changes.

For cross-session or multi-agent work, also read:

- `CONTEXT.md` for domain terms and unresolved decisions;
- `docs/agents/PROJECT-STATUS.md` for the current frontier and last green state;
- `docs/agents/COLLABORATION.md` for writer/reviewer rules;
- `docs/agents/HANDOFF-TEMPLATE.md` before handing work to another agent.

Treat GitHub issues/PRs, commits, tests, ADRs, and these small artifacts as the
shared memory. Do not use chat transcripts or `cache.kv` as the source of truth.
