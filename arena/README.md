# Arena skill

The project-level Arena skill lives at `.agents/skills/arena/SKILL.md` so
Codex can discover it using the standard Agent Skills layout.

It makes the two installed skill families work together efficiently:

1. `codebase-memory-mcp` handles bounded codebase discovery and impact tracing.
2. Matt Pocock's collection supplies the smallest relevant engineering workflow
   (`tdd`, `codebase-design`, `diagnosing-bugs`, `code-review`, and so on).

The local `cache.kv` index and `tools/arena_context.py` keep the first pass
small. The graph/index is only a discovery accelerator; checked-out source
remains authoritative.

Quick context command:

```bash
python3 tools/arena_context.py "<what you are changing>"
```
