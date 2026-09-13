# Agent guidance

For every task in this repository, start with the project Arena skill:
`.agents/skills/arena/SKILL.md`.

The Arena workflow requires two layers:

1. use `codebase-memory-mcp` for bounded codebase discovery, falling back to
   `python3 tools/codebase_index.py` and `cache.kv` when the MCP tools are not
   available;
2. use the smallest relevant skill from the Matt Pocock collection, following
   `.agents/skills/arena/ROUTING.md`.

Keep context efficient: search first, read only candidate files, use focused
limits, and verify important graph/index findings against checked-out source.
Run focused checks and refresh the index after source changes.
