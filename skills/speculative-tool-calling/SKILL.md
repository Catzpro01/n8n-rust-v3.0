---
name: speculative-tool-calling
description: >-
  Speculative Tool-Calling & Execution using small draft models to predict tool parameters in parallel with primary model validation.
---

# Speculative Tool-Calling & Execution

## Guidelines
- Fast draft models predict likely next tool calls and arguments.
- Begin pre-fetching read-only data in parallel with main model reasoning.
- Validate speculative outputs before applying destructive actions.
