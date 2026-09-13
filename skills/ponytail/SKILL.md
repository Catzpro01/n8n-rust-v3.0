---
name: ponytail
description: >-
  YAGNI-extreme engineering philosophy. Produces minimal, direct code solving
  exactly the stated problem. No speculative abstractions, no premature patterns.
---

# Ponytail — Minimal Engineering Philosophy

## Core Principle: YAGNI Extreme
"You Aren't Gonna Need It" — taken seriously.

## What Ponytail Prevents
- Abstract base classes for single implementations
- Factory patterns for objects created in one place
- Config systems for values that never change
- Plugin architectures for code no one else will extend
- Generic types for non-generic cases
- Dependency injection for trivial constructors

## The Test: "Will This Abstraction Pay Off?"
Before adding any abstraction, answer:
1. **When** will the second use case arrive? (Not if — when, with specifics)
2. **How many** implementations are currently needed? (If one, skip)
3. **How much harder** is the direct version to change later? (Usually: less than you think)

## Direct Code Wins Every Time

**Over-engineered:**
```python
class DataProcessorFactory:
    def create_processor(self, type: str) -> BaseDataProcessor:
        return processors[type]()
```

**Ponytail:**
```python
def process_csv(data: str) -> list[dict]:
    return list(csv.DictReader(data.splitlines()))
```

## When Ponytail Is Wrong
Ponytail should be overridden when:
- You're building a public API or library
- Multiple concrete use cases exist NOW (not hypothetically)
- The domain is inherently complex and abstraction reduces cognitive load

## Security Note
⚠️ Ponytail's minimalism can accidentally eliminate:
- Input validation ("it's simple, what could go wrong?")
- Error handling ("won't fail in practice")
- Authentication ("internal service only")

**Always preserve security controls** even when simplifying.
