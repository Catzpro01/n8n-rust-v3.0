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

GitHub Issues through `gh` CLI are the shared intake and status surface. The
recovered implementation also retains detailed local Markdown specifications
and evidence under `.scratch/`; see `docs/agents/issue-tracker.md`.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and
`wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Read `CONTEXT.md` before naming domain concepts and read relevant ADRs under
`docs/adr/` before changing architecture. The recovered project has a large
canonical glossary and 56 retained ADRs; do not silently replace them with a
new vocabulary. See `docs/agents/domain.md`.

### Clean-room and design constraints

The independent clean-room policy in `docs/legal/clean-room-policy.md` is
mandatory. Do not copy n8n source, Enterprise files, UI assets, icons, product
copy, tests, or distinctive trade dress. Compatibility must be established by
public behavior and independently authored fixtures.

When a request concerns visual design, UI/UX, accessibility, diagrams,
wireframes, design systems, screenshots, visual regression, generative UI,
animation, or video, read `.agents/skills/arena-design/SKILL.md`, route the
request with its `route.py`, and activate only the smallest selected design
skill set. Do not load design tooling for backend, database, infrastructure, or
prose-only work.

## Cross-session and multi-agent continuity

Before a large or resumed task, also read:

- `CONTEXT.md` for the canonical glossary, retained decisions, and current
  unresolved questions;
- `docs/agents/PROJECT-STATUS.md` for the current frontier and last green state;
- `docs/agents/COLLABORATION.md` for writer/reviewer rules;
- `docs/agents/HANDOFF-TEMPLATE.md` before handing work to another agent;
- the relevant `.scratch/<effort>/map.md` and ticket when continuing the
  recovered implementation.

Historical recovery records are preserved under `docs/legacy/session-archive/`.
They are evidence of prior work, not permission to expose secrets or to claim
that a check was rerun in this checkout.

## Verification

Build behavior through externally observable tests at the highest stable
interface. Resource claims require cgroup-enforced benchmarks and recorded
evidence. Never claim completion without running the relevant verification
commands. The recovered project pins Rust 1.85.1 and Node 22.19.0 in its
release path; if those tools are unavailable locally, report that limitation
and use GitHub Actions or an approved Rust-enabled environment rather than
silently weakening the gate.

Keep context efficient: search first, read only candidate files, use focused
limits, and verify important graph/index findings against checked-out source.
Run focused checks and refresh the index after source changes.

Treat GitHub issues/PRs, commits, tests, ADRs, `.scratch` tickets, and these
small artifacts as shared memory. Do not use chat transcripts, uploaded archive
parts, private keys, or `cache.kv` as the source of truth.
