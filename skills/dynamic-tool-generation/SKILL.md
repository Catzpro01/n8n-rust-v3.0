---
name: dynamic-tool-generation
description: >-
  On-the-fly Python script writing, testing, and self-registration of custom tools.
---

# Dynamic Tool Generation & Discovery

## Protocol
- Detect missing tool capability for current task.
- Write isolated Python helper script in scratch directory.
- Test script with synthetic inputs.
- Expose script as dynamically callable tool within session.
