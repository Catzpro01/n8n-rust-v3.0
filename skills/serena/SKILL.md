---
name: serena
description: >-
  AI coding assistant built on Language Server Protocol (LSP) supporting 30+ languages
  via MCP. Provides semantic code navigation, symbol lookup, diagnostics, and 
  refactoring for coding agents.
---

# Serena — LSP-Powered Coding Assistant

## Supported Languages (30+)
Python, JavaScript, TypeScript, Rust, Go, Java, Kotlin, Swift, C, C++, C#, Ruby, PHP, Scala, Haskell, Elixir, Erlang, Julia, R, and more.

## Core Capabilities

### Symbol Navigation
- Find definition of any symbol
- Find all references/usages
- Rename symbol across entire codebase
- Navigate call hierarchy

### Diagnostics
- Real-time error and warning detection
- Type checking integration
- Lint rule enforcement

### Refactoring
- Extract function/method
- Inline variable
- Move symbol between files
- Generate boilerplate (constructors, getters, etc.)

## MCP Integration
```json
{
  "mcpServers": {
    "serena": {
      "command": "uvx",
      "args": ["serena-mcp"],
      "env": {
        "PROJECT_ROOT": "${workspaceFolder}"
      }
    }
  }
}
```

## Security Rules
⚠️ Serena has READ/WRITE access to your workspace files
⚠️ Ensure LSP backends (clangd, rust-analyzer, etc.) are updated to avoid RCE vulnerabilities
✅ Restrict Serena to workspace directory — do not point at system directories
✅ Review all refactoring suggestions before applying to production code
