---
name: http-cors-debugger
description: Network protocol and HTTP error debugger. Solves CORS preflight failures, OAuth/JWT auth issues, reverse proxy misconfigs, and status code mismatches.
---

# HTTP, CORS & Network Protocol Debugger

## Purpose
Eliminates frustrating web networking errors: CORS preflight blocks, mysterious 401/403 auth failures, reverse proxy header drops (Nginx/Cloudflare), and TLS/SSL certificate bugs.

## Core Capabilities:
1. **CORS Preflight (OPTIONS) Untangler**:
   - Clarifies Origin, Access-Control-Allow-Origin, and Allowed Headers configurations.
2. **Auth & Cookie Boundary Inspector**:
   - Debugs SameSite, Secure, HttpOnly cookie flags and Bearer Token header propagation.
3. **Reverse Proxy & Gateway Headers**:
   - Ensures X-Forwarded-For, X-Forwarded-Proto, and WebSocket upgrade headers pass correctly.
