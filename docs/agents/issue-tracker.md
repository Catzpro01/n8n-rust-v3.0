# Issue tracker: GitHub plus local implementation specs

GitHub Issues through `gh` are the shared intake, ownership, triage, and status
surface for this repository. The recovered workflow implementation retains its
local Markdown tickets under `.scratch/` because they contain detailed
acceptance criteria, dependency order, test evidence, and durable design notes.
Use both layers; do not replace one with the other.

## GitHub conventions

- Create: `gh issue create --title "..." --body "..."`.
- Read: `gh issue view <number> --comments` and inspect labels.
- List: `gh issue list --state open --json number,title,body,labels,assignees`.
- Comment: `gh issue comment <number> --body "..."`.
- Labels: `gh issue edit <number> --add-label "..."` or `--remove-label`.
- Close: `gh issue close <number> --comment "..."`.
- Infer the repository from the current clone; never hard-code a different
  remote.

Pull requests are not an intake surface for `/triage`. Regular PR review still
uses `gh pr view`, `gh pr diff`, and the Arena review loop.

## Local implementation tickets

- One effort lives in `.scratch/<effort>/`.
- Its Wayfinder map is `.scratch/<effort>/map.md`.
- Child or implementation tickets are numbered files under
  `.scratch/<effort>/issues/` and preserve dependency order.
- `Type:`, `Status:`, and `Blocked by:` lines carry workflow metadata.
- A ticket is on the local frontier when it is open, unclaimed, and every
  listed blocker is resolved.
- Claim by setting `Status: claimed` before editing the ticket.
- Resolve by appending `## Answer`, setting `Status: resolved`, and adding a
  named context pointer to the map.

## Recovered project map

The retained implementation frontier is
`.scratch/eco-100k-first-runnable/map.md`. Tickets 01–07 are recorded as
complete in the recovered evidence; Ticket 08 is an owner-decision gate and
must not be implemented until its recommendations are explicitly approved.
The previous ADRs and ticket evidence remain authoritative unless a new ADR
explicitly supersedes them.

## Wayfinder operations

- **Map:** create one GitHub issue labelled `wayfinder:map` and keep the
  detailed local map synchronized when issue permissions permit.
- **Child ticket:** create one GitHub issue per major decision with a
  `wayfinder:<type>` label, while retaining the detailed local ticket.
- **Blocking:** prefer GitHub native dependencies; otherwise put the issue
  reference and the local `Blocked by:` edge in both records.
- **Claim:** assign the GitHub issue and set the local ticket to `claimed` before
  editing it.
- **Resolve:** record the resolution in the local ticket and ADR first, then
  comment/close the GitHub issue and update the map.
