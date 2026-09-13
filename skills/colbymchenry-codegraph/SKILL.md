---
name: colbymchenry-codegraph
description: >-
  Local code knowledge graph using tree-sitter and SQLite FTS5. Enables
  fast semantic symbol navigation without brute-force grep scanning.
---

# CodeGraph — Semantic Code Navigation

## What It Does
Builds a local knowledge graph of your codebase:
- All function/class/variable definitions indexed
- Full-text search via SQLite FTS5
- Symbol relationships (caller/callee, imports)
- Incremental updates on file change

## Usage

### Index Codebase
```bash
codegraph index --path ./src
codegraph index --path . --lang python,typescript,rust
```

### Query
```bash
# Find all functions named "authenticate"
codegraph find --symbol authenticate --type function

# Find all files importing "jwt"  
codegraph find --import jwt

# Find callers of a function
codegraph callers --symbol authenticate_user --file src/auth.py
```

### In Agent Workflow
Instead of:
```
Read file src/auth.py
Read file src/middleware.py  
Read file src/routes.py
...searching for authentication logic...
```

Use:
```
codegraph find --symbol authenticate
→ Found: src/auth.py:45 (authenticate_user), src/middleware.py:12 (auth_middleware)
→ Read only those specific files at those line numbers
```

## Performance
- Initial index: ~30s for 100K LOC
- Incremental update: <1s per changed file
- Query speed: <10ms for symbol lookup

## Security Note
⚠️ C-bindings (tree-sitter parsers) may have memory safety issues
✅ Only parses — never executes — source code
✅ Graph stored locally in SQLite, no network access
