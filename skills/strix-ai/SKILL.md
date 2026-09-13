---
name: strix-ai
description: >-
  Autonomous AI Pentesting Platform skill (strix.ai). Enforces strict authorization,
  target scope validation, and sandbox isolation before running pentest tasks.
---

# Strix AI — Autonomous Pentesting Framework

> ⚠️ CRITICAL SECURITY WARNING & LEGAL MANDATE:
> 1. ONLY execute pentests against systems/IPs where you have EXPLICIT WRITTEN PERMISSION.
> 2. NEVER execute against unauthorized external targets or production environments.
> 3. MUST run inside isolated containers/sandboxes.

## Guardrail Protocol (Mandatory Before Execution)

### Step 1: Target Verification
Before running any audit or exploit scan, verify:
- Is the target URL/IP explicitly in the authorized scope document?
- Is the target environment sandbox/staging (NOT production)?
- Is the scope bounded (e.g., `192.168.1.0/24` or `staging.example.com`)?

### Step 2: Safe Execution Rules
- Disable destructive modules (e.g., DoS, memory corruption, data wiping)
- Enable detailed logging (`--log-level DEBUG`) to track all outbound probes
- Limit rate of scan requests to prevent accidental service degradation

## Usage Guidelines
```bash
# Example: Run pentest audit on local staging sandbox only
strix audit --target http://localhost:8080 --scope local-staging --no-destructive
```
