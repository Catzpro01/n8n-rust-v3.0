# Multi-agent collaboration protocol

This repository is designed for several AI agents to work on one product
without sharing hidden conversation state. Git, tickets, and small durable
Markdown artifacts are the coordination layer.

## Source-of-truth order

When two agents disagree, use this order:

1. checked-out source and passing tests;
2. the current ticket/spec acceptance criteria;
3. ADRs and `CONTEXT.md`;
4. `PROJECT-STATUS.md` and a handoff document;
5. Codebase Memory or `cache.kv` results, which are discovery accelerators only;
6. chat messages and assumptions last.

## Roles

- **Planner:** uses `/ask-matt`, `/wayfinder`, `/grill-with-docs`, and
  `/to-spec`; records decisions and blockers.
- **Builder:** claims exactly one ticket, uses `/implement` and `/tdd`, and
  owns the code changes for that ticket.
- **Reviewer:** reads the ticket and diff, uses `/code-review` plus Codebase
  Memory/source verification, and reports findings without silently rewriting
  the builder's work.
- **Verifier:** runs the narrow test suite, lint/build checks, and
  `verification-before-completion`; records command evidence.

One agent can hold more than one role sequentially, but the role boundary must
be explicit in the handoff.

## Same-repository rules

- Prefer one branch/worktree per ticket or agent. Share the remote repository,
  not one mutable working directory.
- Never let two agents edit the same files concurrently. A reviewer is
  read-only until the builder accepts findings.
- Claim a ticket before editing it. Use states: `ready`, `claimed`,
  `in-progress`, `review`, `blocked`, `done`.
- Keep commits small and ticket-shaped. Include the ticket name in the commit
  message when possible.
- Before starting, check `git status`, branch, and the latest remote state. Do
  not overwrite uncommitted work from another agent.
- A reviewer reports `file:line`, evidence, severity, and a suggested fix. The
  builder decides whether to apply it; unverified opinions are not changes.

## Session handoff

At a context boundary or before stopping, update `PROJECT-STATUS.md` and create
one handoff from `HANDOFF-TEMPLATE.md` when another agent must continue. Keep a
handoff short and link to the ticket, PR, commit, ADR, test output, and changed
files instead of copying them.

A handoff must state:

- exact current status and last green commit;
- what was verified and the commands used;
- decisions made and where they are recorded;
- files changed or intentionally untouched;
- blockers, risks, and open questions;
- one concrete next action;
- skills the next agent should use.

Never put credentials, tokens, private payloads, or full chat transcripts in a
handoff.

## Critique loop

1. Builder writes a failing test or acceptance check before implementation
   when behavior is changing.
2. Builder implements one vertical slice.
3. Verifier runs focused checks and records fresh output.
4. Reviewer inspects the diff against the ticket and source evidence.
5. Builder addresses accepted findings and reruns checks.
6. Only after fresh verification does the owner commit, open/update a PR, or
   call the slice complete.

The goal is independent criticism, not multiple agents making overlapping edits.
