---
name: llm-council
description: >-
  Multi-LLM consensus deliberation. Routes ambiguous or high-stakes decisions 
  to multiple models for debate and cross-validation. Reduces single-model bias
  and hallucination risk.
---

# LLM Council — Multi-Model Consensus

## When to Invoke Council
Invoke LLM Council for decisions that are:
- **Architectural** — choosing between design patterns or system designs
- **Security-sensitive** — evaluating cryptographic approaches or access control
- **Ambiguous** — requirements that could be interpreted multiple ways
- **Irreversible** — database migrations, API breaking changes, deletions

## Council Process

### Round 1: Independent Analysis
Each council member produces an independent response without seeing others' answers.

### Round 2: Cross-Review
Each member reviews the others' responses and identifies:
- Points of agreement (likely correct)
- Points of disagreement (needs deeper analysis)
- Risks or blind spots in each approach

### Round 3: Synthesis
Produce a final synthesis that:
- Incorporates the strongest arguments from each model
- Explicitly notes where models disagreed and why
- Assigns confidence levels to each recommendation

## Output Format
```
## Council Decision: [TOPIC]
**Consensus Level**: HIGH / MEDIUM / LOW / SPLIT

### Points of Agreement
- [unanimous finding 1]

### Points of Disagreement  
- Model A says X; Model B says Y — Analysis: ...

### Final Recommendation
[synthesized recommendation with confidence level]

### Minority Opinion (if any)
[dissenting view worth preserving]
```

## Cost Note
⚠️ Council mode multiplies token cost by the number of models used. Reserve for truly high-stakes decisions.
