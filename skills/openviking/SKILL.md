---
name: openviking
description: >-
  Long-term memory and context database for AI agents using filesystem-based
  viking:// protocol. Stores skills, knowledge, and agent state persistently.
---

# OpenViking — Agent Context Database

> ⚠️ CONTEXT POISONING RISK: Untrusted data stored in agent memory can
> influence future decisions. Only store vetted, trusted information.

## Purpose
Provides persistent memory for AI agents across sessions using a structured filesystem database accessible via `viking://` protocol.

## Storage Structure
```
~/.viking/
├── skills/          # Agent skill definitions
├── knowledge/       # Domain knowledge base
├── projects/        # Project-specific context
├── personas/        # Agent personas and roles
└── conversations/   # Historical conversation context
```

## Usage

### Store Knowledge
```bash
viking store "knowledge/python/asyncio"   "Python asyncio uses an event loop. Use async/await for I/O bound tasks..."

viking store "projects/agent-os/architecture"   "Agent OS uses SQLite for task management. REST API on port 8080..."
```

### Retrieve Context
```bash
# Get specific knowledge
viking get "knowledge/python/asyncio"

# Search across all knowledge
viking search "event loop" --type knowledge

# List all project context
viking list "projects/agent-os/*"
```

### Agent Integration (MCP)
```json
{
  "mcpServers": {
    "openviking": {
      "command": "openviking-mcp",
      "args": ["--db", "~/.viking"]
    }
  }
}
```

## Security Rules
⚠️ Never store API keys, passwords, or secrets in Viking DB (no encryption)
⚠️ Sanitize all content before storing to prevent context poisoning
✅ Viking DB is local-only — no external network access
✅ Back up `~/.viking` directory with the same care as your codebase
