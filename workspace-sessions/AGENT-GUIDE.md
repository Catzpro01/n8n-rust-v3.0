# Agent guide: keep session work durable

Every agent working in this repository must keep the session record current.
Use simple, factual notes so the next agent can resume without guessing.

## Before work

1. Read this file and `workspace-sessions/current/SESSION.md`.
2. Read `AGENTS.md`, the relevant Arena/project instructions, `CONTEXT.md`,
   `docs/agents/PROJECT-STATUS.md`, the active `.scratch/` map/ticket, and
   relevant ADRs.
3. Confirm the current branch and working tree with Git.
4. Do not treat `cache.kv` as a source of truth; refresh it only with the
   dependency-free indexer when source/docs change.

## During work

After every meaningful progress point, append one entry to
`workspace-sessions/current/PROGRESS.md` containing:

- date;
- action and files/issue touched;
- decision or result;
- verification command/result;
- blocker or next action.

Update `workspace-sessions/current/SESSION.md` whenever the active ticket,
owner decision, blocker, branch head, or next action changes. Keep it short and
point to detailed source files instead of copying them.

For a new durable decision, update the relevant local ticket and ADR/CONTEXT
according to the project workflow. Do not silently override an accepted ADR.
For an implementation change, keep the ticket acceptance evidence and
`docs/agents/PROJECT-STATUS.md` synchronized. Run the repository indexer after
adding or changing source/documentation metadata, then inspect the diff.

## Before ending a session

1. Append the final result, tests, CI run IDs, commit, and remaining blockers.
2. Make sure `git diff --check` passes and record the working-tree state.
3. If the work is complete, push only the fixed Arena branch and record the
   remote/CI evidence.
4. Do not call unverified historical checks fresh.

## Starting a later session

1. Preserve the old active record by copying its safe summary into
   `workspace-sessions/previous/SESSION.md` (replace the previous snapshot only
   at this boundary).
2. Reset `workspace-sessions/current/SESSION.md` and `PROGRESS.md` for the new
   session, retaining links to the previous record and active map/ticket.
3. Never delete the source-of-truth ADRs, tickets, or Git history while doing
   this rotation.

## Safety

Never write secrets or recovery material into session records: no passwords,
private keys, access tokens, cookies, CSRF values, master keys, recovery
phrases, wrapped keys, nonces, credential values, or sensitive server details.
Use a redacted description such as “credential boundary verified” and link to
an approved security document when needed.
