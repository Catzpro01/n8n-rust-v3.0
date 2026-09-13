---
name: browser-harness
description: >-
  Chrome DevTools Protocol (CDP) harness connecting AI agents directly to browser instances.
---

# Browser Harness — CDP Automation Interface

## Core Capabilities
- Direct CDP event listening and dispatch
- Network request inspection and mocking
- Console error log monitoring
- Performance trace and DOM snapshot capture

## Guardrails
- Run inside headless Docker containers for isolation
- Do not expose open CDP ports (9222) to external networks
