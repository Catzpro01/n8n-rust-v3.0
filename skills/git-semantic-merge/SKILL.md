---
name: git-semantic-merge
description: >-
  AST-aware Git merge conflict resolver (Tree-Sitter semantic merge). Resolves complex code branch conflicts by understanding AST structures instead of line-by-line diffs.
---

# Git Semantic Merge — AST-Aware Conflict Resolver

## Purpose
Resolves git merge conflicts deterministically using Tree-Sitter AST symbol parsing, preserving concurrent modifications to imports, interfaces, and function bodies without syntax breakage.

## Resolution Protocol
1. **Symbolic Partitioning:** Parse conflicting hunks into distinct AST declarations.
2. **Independent Merge:** Combine non-colliding symbol additions (e.g. independent import statements or new methods).
3. **Semantic Synthesis:** Reconstruct unified function signatures when signatures are modified concurrently.
4. **Validation Check:** Run compiler dry-run to ensure 0 syntax or type errors after merge.
