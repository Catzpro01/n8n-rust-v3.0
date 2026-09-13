---
name: agent-browser
description: >-
  Browser automation for AI agents using accessibility-tree navigation.
  Provides structured web browsing, DOM interaction, and form filling.
---

# Agent Browser — Accessibility-Tree Web Automation

## Core Capabilities
- Inspect web page accessibility tree (DOM without visual bloat)
- Click buttons, input text, select dropdowns, submit forms
- Capture screenshots for verification
- Handle multi-tab workflows

## Guardrails & Security
- Never submit sensitive credentials (passwords, credit cards) without confirmation
- Restrict browsing to authorized URLs; do not navigate to intranet/local IPs
- Implement 1-3s delays between actions to prevent rate-limiting
