---
name: api-reverse-engineer
description: >-
  Network traffic to SDK reverse-engineering engine. Converts cURL logs, browser HAR files, and raw HTTP payloads into strongly-typed TypeScript/Python client SDKs.
---

# API Reverse Engineer — Traffic-to-SDK Generator

## Purpose
Reconstructs undocumented or private web endpoints into fully typed, production-ready client SDKs by analyzing raw HTTP requests, cURL commands, and HAR network archives.

## Capabilities
- Extract query params, request headers, authentication headers, and payload schemas from network captures.
- Infer strict TypeScript/Pydantic response models from dynamic JSON payloads.
- Generate complete SDK wrappers with built-in retry logic, authentication handlers, and type safety.
