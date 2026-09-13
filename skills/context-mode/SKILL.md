---
name: context-mode
description: >-
  Context window management skill. Indexes tool execution outputs to SQLite
  locally to prevent context rot and maintain conversation coherence over
  long agent sessions.
---

# Context Mode — Context Window Management

## Problem: Context Rot
In long agent sessions, the context window fills with:
- Redundant tool outputs (same files read multiple times)
- Obsolete information that no longer reflects current state
- Verbose outputs that crowd out important context
- Duplicate error messages

## Solutions

### 1. Context Compression
Before adding tool output to context:
- Summarize verbose outputs (100-line file → key sections only)
- Deduplicate: if file was read before, reference previous read
- Prune: remove tool outputs that are no longer relevant

### 2. Reference Anchors
Instead of re-reading files:
```
[Previously read at step 14: /src/auth.py - JWT validation function at L45-67]
```

### 3. Active Context Inventory
Maintain a running inventory of what's in context:
```
## Active Context Inventory
- /src/auth.py (full, 120 lines) - read at step 5
- Database schema (summary) - read at step 3
- Error from step 8: "KeyError: 'user_id'" in auth.py:52
- Current task: Fix JWT validation bug
```

### 4. Context Budget
Estimate context usage:
- Total context: 200K tokens
- Reserve for output: 20K tokens
- Available for context: 180K tokens
- Current usage: ~45K tokens (25%)

### 5. When to Compact
Compact context when:
- Usage exceeds 70% of budget
- More than 10 tool outputs in context
- Session has been running > 30 minutes
- Starting a new sub-task

## SQLite Index (Advanced)
Store tool outputs in local SQLite for retrieval:
```python
# Store output
db.execute("INSERT INTO context_cache VALUES (?, ?, ?, ?)",
           (tool_name, input_hash, output, timestamp))

# Retrieve if already cached
result = db.execute("SELECT output FROM context_cache WHERE input_hash=?", 
                    (hash(inputs),)).fetchone()
```

## Security Note
⚠️ SQLite cache stores command outputs — may contain sensitive data
✅ Store in user home directory only: `~/.context_mode/cache.db`
✅ Auto-expire entries older than 24 hours
