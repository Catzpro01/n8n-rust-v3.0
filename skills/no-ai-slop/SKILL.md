---
name: no-ai-slop
description: >-
  Quality filter for AI output. Detects and eliminates generic clichés,
  overused phrases, and hollow AI-speak patterns. Forces specific, natural,
  human-quality writing.
---

# No AI Slop — Output Quality Filter

## Purpose
AI models often produce recognizable "slop" — generic, hollow phrases that signal low-quality, unthinking output. This skill enforces quality writing standards.

## Banned Phrases (Auto-Replace)
| Banned | Replace With |
|--------|-------------|
| "delve into" | "explore" / "examine" / "look at" |
| "it's worth noting that" | (delete — just say it) |
| "in the realm of" | (delete — be specific) |
| "leverage" (when not about financial leverage) | "use" |
| "cutting-edge" | (be specific about what's novel) |
| "seamlessly" | (delete or describe what makes it seamless) |
| "robust solution" | (describe what makes it robust) |
| "at the end of the day" | (delete) |
| "moving forward" | (delete) |
| "game-changing" | (describe the actual change) |
| "I'd be happy to help" | (just help) |
| "Certainly!" / "Absolutely!" | (just answer) |
| "As an AI language model" | (never say this) |

## Quality Standards
1. **Be specific** — instead of "fast algorithm," say "O(log n) binary search"
2. **Show don't tell** — instead of "clean code," show the clean code
3. **Concrete over abstract** — instead of "user-friendly," describe the specific UX behavior
4. **Direct over diplomatic** — if something is wrong, say it's wrong

## Application
Apply this filter to:
- Documentation and comments
- User-facing messages and UI copy  
- Code review feedback
- Technical explanations
- Any written output to the user
