---
name: skill-sh
description: >-
  Standardized shell scripting patterns and POSIX-compliant automation for agent environments.
---

# Skill Shell — POSIX Automation Standard

## Guidelines
- Write strictly POSIX-compliant shell scripts (`#!/bin/sh`)
- Robust error handling: `set -euo pipefail`
- Safe quoting: Always quote variable expansions `"$VAR"`
- Defensive temporary file creation using `mktemp`
