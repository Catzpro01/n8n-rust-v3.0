---
name: mcp-builder
description: >-
  Development framework for building MCP (Model Context Protocol) servers.
  Provides templates, validation, and tooling to accelerate MCP server creation
  from text descriptions.
---

# MCP Builder — Server Development Framework

## What is MCP?
Model Context Protocol (MCP) is an open standard for connecting AI agents to external tools and data sources. MCP servers expose capabilities that agents can call.

## MCP Server Structure
```typescript
// Minimal MCP server template
import { Server } from "@modelcontextprotocol/sdk/server/index.js";

const server = new Server({
  name: "my-tool-server",
  version: "1.0.0"
});

// Register a tool
server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [{
    name: "my_tool",
    description: "What this tool does",
    inputSchema: {
      type: "object",
      properties: {
        input: { type: "string", description: "The input" }
      },
      required: ["input"]
    }
  }]
}));

// Handle tool calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  if (request.params.name === "my_tool") {
    const result = doWork(request.params.arguments.input);
    return { content: [{ type: "text", text: result }] };
  }
});
```

## Security Checklist for Every MCP Server
- [ ] Validate ALL input parameters before processing
- [ ] Never execute shell commands with unsanitized input
- [ ] Implement rate limiting on expensive operations
- [ ] Require authentication for servers exposing sensitive data
- [ ] Log all tool calls for audit trail
- [ ] Never return credentials or secrets in tool responses
- [ ] Test with adversarial inputs (injection attempts)

## Registration in Antigravity
```json
{
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["dist/server.js"],
      "env": { "API_KEY": "${MY_API_KEY}" }
    }
  }
}
```
