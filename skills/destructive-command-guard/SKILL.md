---
name: destructive-command-guard
description: >-
  Circuit-breaker and guardrail system preventing accidental execution of destructive shell, filesystem, database, or registry commands.
---

# Destructive Command Guard — System Safety Circuit-Breaker

## Purpose
Intercepts and blocks dangerous or irreversible operating system commands, requiring explicit user approval before execution.

## Blocked Command Signatures
- **Filesystem wipe:** 
mdir /s /q C:\, 
m -rf /, wildcard root deletions.
- **Drive / Partition formatting:** Format-Volume, diskpart clean, dd if=/dev/zero.
- **Database drops:** DROP DATABASE, TRUNCATE TABLE without transaction wraps.
- **System registry changes:** Unsafe 
eg delete or tampering with system boot configurations.
- **Git destructive rewrites:** git push --force to protected upstream branches without explicit confirmation.

## Enforcement
If a blocked signature is detected: Pause execution, display warning explanation, and wait for explicit user confirmation.
