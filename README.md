# n8n-rust-v3.0

This repository includes a small, dependency-free codebase indexer for fast local navigation. It uses an incremental `cache.kv` file in the repository root, similar in spirit to the local index that powers editor search tools such as Cursor.

## Fast codebase indexing

The indexer:

- scans source and documentation files while skipping `.git`, `.agents`, dependencies, and build output;
- reuses unchanged files using size and modification time metadata;
- tokenizes natural-language words and common identifier styles such as `snake_case` and `CamelCase`;
- extracts lightweight symbols (`fn`, `class`, `function`, `struct`, `def`, and similar definitions);
- ranks search results using an inverted index with TF-IDF-style scoring plus path, prefix, and symbol boosts;
- stores file metadata and the inverted index in `cache.kv` without storing source contents, so search does not need to rebuild postings on every invocation.

### Usage

```bash
# Build or incrementally refresh the index
python3 tools/codebase_index.py index

# Search by concept, filename, or symbol prefix
python3 tools/codebase_index.py search "workflow execution"
python3 tools/codebase_index.py search run_workflow --limit 20

# Inspect the cache
python3 tools/codebase_index.py stats

# Force a full rebuild
python3 tools/codebase_index.py index --force
```

The cache is updated atomically, so an interrupted refresh does not leave a partially written index. Run `search --no-index` when a read-only lookup must use exactly the current cache.

### Arena workflow

The project-level skill at `.agents/skills/arena/SKILL.md` combines the two
installed skill families without loading all of their instructions on every
turn:

- `codebase-memory-mcp` for bounded discovery and impact tracing;
- the smallest relevant Matt Pocock skill for design, TDD, diagnosis, or
  review.

For a compact task context packet:

```bash
python3 tools/arena_context.py "<what you are changing>"
```

`AGENTS.md` points agents to this workflow at the start of every task. The
local index is only a discovery accelerator; checked-out source remains the
source of truth.

### Tests

```bash
python3 -m unittest discover -s tests -v
```

The index is intentionally lexical and local: it does not require an embedding model, API key, database, or background service. That keeps first-time setup reliable and makes incremental refreshes fast. A semantic/vector layer can be added later without changing the `cache.kv` file contract.
