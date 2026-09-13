# Arena skill routing

Use this table after the mandatory Codebase Memory discovery pass. Only read
the selected Matt skill, not the whole collection.

| Request shape | Matt skill | Minimum verification |
| --- | --- | --- |
| Add or change behavior | `tdd` | failing test first, focused test suite |
| Fix a hard or slow bug | `diagnosing-bugs` | reproduction, instrumentation, regression test |
| Choose a module/interface/seam | `codebase-design` | interface and adapter impact, tests at the seam |
| Plan is vague or has competing options | `grill-me` | resolve open decisions before implementation |
| Explore a large/unfamiliar area | `improve-codebase-architecture` | bounded scope and source evidence |
| Review an existing diff | `code-review` | standards and spec review against a fixed point |
| Create or update agent docs | `writing-for-agents` | concise pointer, no duplicated instructions |
| Configure issue/domain docs | `setup-matt-pocock-skills` | user confirmation before writing repo config |

## Default code-change sequence

1. `codebase-memory-mcp` or `tools/codebase_index.py` for discovery.
2. `codebase-design` for a seam/interface decision, if one exists.
3. `tdd` for behavior changes.
4. `code-review` for a meaningful final diff.

Skip a step only when it does not apply, and state that choice in the final
summary. For tiny literal/config edits, exact local search is enough; do not
pay for broad architecture discovery.
