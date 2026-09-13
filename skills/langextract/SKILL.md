---
name: langextract
description: >-
  Python library for structured data extraction from free text with source
  grounding. Each extracted field is traceable to its source passage,
  preventing hallucinated extractions.
---

# LangExtract — Grounded Data Extraction

## Key Feature: Source Grounding
Unlike naive LLM extraction, LangExtract verifies each extracted field against the source text, providing evidence for every value.

## Usage

### Basic Extraction
```python
from langextract import extract

schema = {
    "company_name": "string",
    "founded_year": "integer",
    "headquarters": "string",
    "revenue": "string"
}

result = extract(
    text="Apple Inc. was founded in 1976 in Cupertino, California...",
    schema=schema
)

print(result.data)
# {'company_name': 'Apple Inc.', 'founded_year': 1976, ...}

print(result.evidence)
# {'company_name': 'Apple Inc. was founded...', 'founded_year': '...in 1976...'}
```

### Batch Extraction
```python
results = extract_batch(documents, schema, parallel=True)
```

### Validation
```python
# Reject low-confidence extractions
result = extract(text, schema, min_confidence=0.8)
if result.confidence < 0.8:
    raise ValueError(f"Low confidence extraction: {result.confidence}")
```

## When to Use
- Invoice/receipt parsing
- Legal document data extraction
- Research paper metadata extraction
- Form field population from free text

## Rules
- ALWAYS validate schema types after extraction
- NEVER trust extractions without checking `result.evidence`
- For financial data: always have a human verify before use
- For PII: apply data minimization — only extract what you need
