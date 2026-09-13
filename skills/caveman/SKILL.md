---
name: caveman
description: >-
  Simple, effective debugging via strategic print/log statement placement.
  When debuggers fail or are unavailable, caveman debugging traces execution
  flow to isolate bugs methodically.
---

# Caveman Debugging

## Philosophy
"The bug is always where you least expect it. So trace everything."

## Methodology

### Step 1: Hypothesis First
Before adding any print statements, write down your hypothesis:
- "I think the bug is in function X because..."
- "I expect variable Y to be Z at this point"

### Step 2: Binary Search
Add prints to halve the search space:
- Confirmed working? → Move print forward
- Bug already visible? → Move print backward
- Never add 10 prints at once

### Step 3: Trace Points Pattern
```python
# Mark entry and exit
print(f"[CAVEMAN] ENTER: function_name | args={args!r}")
result = do_work(args)
print(f"[CAVEMAN] EXIT: function_name | result={result!r}")

# Trace state at key decision points
print(f"[CAVEMAN] CHECKPOINT: condition={condition!r}, branch={'if' if condition else 'else'}")
```

### Step 4: Log Levels
Use prefixes to filter noise:
- `[CAVEMAN-CRITICAL]` — things that should never be None/empty
- `[CAVEMAN-STATE]` — variable state at key moments
- `[CAVEMAN-FLOW]` — execution path tracing

### Step 5: Clean Up
ALWAYS remove caveman logs before commit:
```bash
grep -rn "CAVEMAN" . --include="*.py"
```

## Security Warning
⚠️ NEVER caveman-log: API keys, passwords, tokens, PII, session cookies
⚠️ Check that log output doesn't go to production log aggregators
⚠️ Use log level filtering in production to prevent accidental exposure
