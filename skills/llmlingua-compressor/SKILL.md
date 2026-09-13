---
name: llmlingua-compressor
description: >-
  Perplexity-guided prompt and token pruning compressor (Microsoft Research LLMLingua-2). Compresses input text 5x-20x without semantic loss.
---

# LLMLingua Compressor — Perplexity Token Pruner (Microsoft Research)

## Purpose
Prunes non-essential words, linguistic fluff, and repetitive token sequences using perplexity scoring, achieving up to 95% token savings on long prompts.

## Guidelines
- Remove redundant adjectives, filler transitions, and repetitive syntax.
- Retain all code logic, AST signatures, identifiers, and constraint definitions verbatim.
- Enforce ultra-compact prompting across multi-agent workflows.
