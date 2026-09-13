---
name: entroly
description: >-
  Budgeted evidence selection and recoverable context compression for local codebases (juyterman1000/entroly).
---

# Entroly — Budgeted Context Compression

## Purpose
Compresses dense codebases and documents within strict token budgets while guaranteeing that key functional evidence, signatures, and call-graphs remain recoverable.

## Methodology
- Parse AST symbols and index variable definitions.
- Allocate token budgets per module based on centrality scores.
- Compress implementation bodies while strictly retaining public interfaces.
