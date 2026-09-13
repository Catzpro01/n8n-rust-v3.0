---
name: nidhinjs-prompt-master
description: >-
  Prompt engineering skill for optimizing AI prompt formulation. Makes prompts
  more precise, token-efficient, and effective at eliciting high-quality responses.
---

# Prompt Master — Prompt Engineering Excellence

## Core Principles

### 1. Be Specific and Concrete
❌ "Write a function to handle errors"
✅ "Write a Python function that catches `requests.HTTPError` and `requests.ConnectionError`, logs the error with `logging.error()`, and returns `None` with a user-friendly string message"

### 2. Specify Output Format
❌ "Explain how OAuth works"
✅ "Explain OAuth 2.0 Authorization Code flow in exactly 5 bullet points, each under 20 words, targeted at a junior developer"

### 3. Provide Context
❌ "Fix this bug"
✅ "In this Django REST Framework view, the authentication is failing for JWT tokens. Here is the relevant code and the error traceback..."

### 4. Constrain Scope
❌ "Improve this code"
✅ "Improve ONLY the error handling in this function. Do not change the business logic, types, or function signature."

## Token Efficiency Techniques

### Compression Patterns
- Use abbreviations for repeated concepts (define once): "Use `[FN]` to refer to function name"
- Reference line numbers instead of quoting full code
- Ask for summaries first, details on request
- "In <20 words:" prefix forces concise responses

### Context Management
- State constraints upfront: "No external libraries", "Python 3.10+", "Must be backwards compatible"
- Use delimiters: `<code>...</code>`, `<context>...</context>`, `<task>...</task>`
- Prioritize information: most important constraints first

## Prompt Templates

### Code Review
```
Review the code in <code> below for:
1. Security vulnerabilities (list CVE type if applicable)
2. Performance issues (Big-O analysis where relevant)
3. Code quality issues (maintainability)

Respond in format: Issue | Severity (HIGH/MED/LOW) | Fix

<code>
{code}
</code>
```

### Debugging
```
Debug this issue:
- Environment: {env}
- Expected: {expected}
- Actual: {actual}
- Error: {error}
- Attempts already tried: {attempts}

Provide: Root cause + minimal fix
```
