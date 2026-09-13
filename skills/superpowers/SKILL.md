---
name: superpowers
description: >-
  Structured coding methodology with mandatory brainstorm → spec → TDD stages
  before any implementation. Reduces bugs, technical debt, and agent hallucinations.
---

# Superpowers Engineering Methodology

## Core Philosophy
Great software is built through disciplined thinking, not fast typing. This skill enforces a rigorous process before any line of code is written.

## Mandatory Pre-Coding Stages

### 1. Brainstorm (5 min)
Think out loud before starting:
- What is the simplest correct solution?
- What are the top 3 ways this could fail?
- What would I regret in 6 months?

### 2. Specification
Write a clear, testable specification:
- Input/Output contract
- Invariants and preconditions
- Error cases and their handling strategy

### 3. Test-First (TDD)
Write the test before the implementation:
- Start with the simplest failing test
- Make it pass with minimal code
- Refactor while keeping tests green

### 4. Implementation
Code with the spec in hand:
- Each function does ONE thing
- Each module has ONE responsibility
- No premature optimization
- No magic numbers — name everything

### 5. Review Checklist
Before calling DONE:
- [ ] All tests pass
- [ ] No linter warnings
- [ ] Error cases handled explicitly
- [ ] Documentation updated
- [ ] No TODO left unaddressed

## Key Principles
- Clarity > Cleverness
- Explicit > Implicit  
- Simple > Complex (until proven otherwise)
- Tests are first-class code
