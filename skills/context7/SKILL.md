---
name: context7
description: >-
  MCP server (Upstash Context7) that resolves library names to their latest
  documentation and injects it directly into LLM context. Eliminates hallucinated
  API calls by providing accurate, version-specific documentation.
---

# Context7 — Real-Time Library Documentation

## Purpose
LLMs hallucinate API methods that don't exist because their training data is outdated. Context7 fixes this by fetching current documentation at query time.

## How It Works
1. You mention a library name in your prompt (e.g., "using React Query v5")
2. Context7 MCP server resolves the library to its current documentation
3. Relevant docs are injected into context before the LLM generates code
4. The LLM generates code against actual, version-correct APIs

## Setup (MCP Configuration)
Add to your `mcp_config.json`:
```json
{
  "mcpServers": {
    "context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp"]
    }
  }
}
```

## Supported Libraries
Covers thousands of popular npm, PyPI, and Go packages including:
- React, Next.js, Vue, Svelte
- Express, FastAPI, Django
- Prisma, Drizzle, TypeORM
- Tailwind CSS, shadcn/ui
- And many more

## Security Notes
⚠️ This tool depends on Upstash's external servers. If Upstash servers are unavailable, documentation injection fails silently — LLM falls back to training data.
⚠️ Risk of poisoned documentation if Upstash's endpoint is compromised. Mitigate by pinning specific library versions in your prompts.

## Best Practice
Always specify the version:
- ✅ "using React Query v5.0"
- ❌ "using React Query" (may resolve to wrong version)
