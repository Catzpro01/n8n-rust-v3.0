---
name: repo-map
description: >-
  Tree-sitter PageRank repository map generator (Aider architecture). Maps codebase structure and symbols using 1-2% of original tokens.
---

# Repo Map — Codebase Awareness Engine (Aider Architecture)

## Purpose
Generates a dense, token-efficient repository map using Tree-Sitter parsing and PageRank centrality ranking. Gives the agent 360-degree codebase awareness without reading raw files.

## Guidelines
- Parse function signatures, class declarations, and import graphs using AST Tree-Sitter.
- Rank symbols by PageRank centrality to surface the most relevant definitions.
- Keep total codebase representation strictly within 1-2K tokens.
