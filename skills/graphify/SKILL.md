---
name: graphify
description: >-
  Converts codebases into AST-based knowledge graphs using tree-sitter.
  Enables agents to reason over code relationships, call graphs, and 
  dependency chains without brute-force file scanning.
---

# Graphify — Codebase Knowledge Graph

## What It Produces
For any codebase, Graphify extracts:
- **Call graph**: which function calls which
- **Import graph**: module dependency tree
- **Type graph**: inheritance and interface relationships
- **Symbol index**: all definitions and their locations

## Usage

### Index a Codebase
```bash
graphify index --path ./src --output ./graph.json
graphify index --path . --languages python,typescript
```

### Query the Graph
```bash
# Find all callers of a function
graphify query "WHO_CALLS:authenticate_user"

# Find all dependencies of a module
graphify query "DEPENDS_ON:src/auth/jwt.py"

# Find circular dependencies
graphify query "CYCLES:"
```

### Agent Integration
The agent can use the graph to:
1. Understand impact before modifying a function
2. Find all usages before renaming a symbol
3. Detect architectural violations (e.g., UI layer importing DB layer)
4. Identify dead code (symbols with no callers)

## Security Notes
⚠️ Knowledge graph may expose sensitive architectural information
⚠️ Do not export graph files to external/cloud services without review
✅ Graph is built locally — no external API calls
✅ Read-only analysis — does not modify source code
