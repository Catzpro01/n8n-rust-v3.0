# Arena skill routing

Use this table after the mandatory Codebase Memory discovery pass. Only read
the selected Matt skill, not the whole collection.

| Request shape | Matt skill | Minimum verification |
| --- | --- | --- |
| Add or change behavior | `tdd` | failing test first, focused test suite |
| Any bug, test failure, or surprise | `systematic-debugging` | reproduce and establish root cause before changing code |
| Fix a hard or slow bug | `diagnosing-bugs` | reproduction, instrumentation, regression test |
| Choose a module/interface/seam | `codebase-design` | interface and adapter impact, tests at the seam |
| Plan is vague or has competing options | `grill-me` | resolve open decisions before implementation |
| Project larger than one session | `wayfinder` | decision map, blockers, and explicit destination |
| Explore a large/unfamiliar area | `improve-codebase-architecture` | bounded scope and source evidence |
| Review an existing diff | `code-review` | standards and spec review against a fixed point |
| Continue in a fresh session/agent | `handoff` | status, evidence, blockers, one next action |
| Before completion, commit, or PR | `verification-before-completion` | fresh command output proving the claim |
| Create or update agent docs | `writing-for-agents` | concise pointer, no duplicated instructions |
| Configure issue/domain docs | `setup-matt-pocock-skills` | user confirmation before writing repo config |

## Default code-change sequence

1. `codebase-memory-mcp` or `tools/codebase_index.py` for discovery.
2. `codebase-design` for a seam/interface decision, if one exists.
3. `tdd` for behavior changes.
4. `systematic-debugging` when a failure or surprise occurs.
5. `code-review` for a meaningful final diff.
6. `verification-before-completion` before a completion claim, commit, or PR.

Skip a step only when it does not apply, and state that choice in the final
summary. For tiny literal/config edits, exact local search is enough; do not
pay for broad architecture discovery.
