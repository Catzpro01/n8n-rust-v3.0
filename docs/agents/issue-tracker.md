# Issue tracker: GitHub

Issues and specs for this repository live as GitHub Issues. Use `gh` for all
issue and PR operations.

## Conventions

- Create: `gh issue create --title "..." --body "..."`.
- Read: `gh issue view <number> --comments` and inspect labels.
- List: `gh issue list --state open --json number,title,body,labels,assignees`.
- Comment: `gh issue comment <number> --body "..."`.
- Labels: `gh issue edit <number> --add-label "..."` or `--remove-label`.
- Close: `gh issue close <number> --comment "..."`.
- Infer the repository from the current clone; never hard-code a different
  remote.

## Pull requests as a triage surface

**No.** Pull requests are not treated as an intake surface for `/triage`.
Regular PR review still uses `gh pr view`, `gh pr diff`, and the Arena review
loop.

## Wayfinder operations

- **Map:** create one issue labelled `wayfinder:map` containing the destination,
  notes, decisions-so-far, and fog of war.
- **Child ticket:** create one issue per decision with a `wayfinder:<type>` label
  (`research`, `prototype`, `grilling`, or `task`), and link it to the map as a
  sub-issue where GitHub supports it.
- **Blocking:** prefer GitHub's native issue dependencies. If unavailable, put
  `Blocked by: #<number>` at the top of the issue body.
- **Claim:** assign the ticket to the current agent before editing it.
- **Resolve:** add the resolution as a comment, close the issue, then append a
  short linked decision pointer to the map.

## When a skill says "publish to the issue tracker"

Create or update a GitHub Issue. Keep acceptance criteria and blocking edges in
the issue; keep implementation detail in the code and tests.
