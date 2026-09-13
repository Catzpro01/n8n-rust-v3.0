---
name: gitleaks-shield
description: >-
  Automated secret and credential scanner preventing API tokens, private keys, and passwords from leaking into git commits, transcripts, or context logs.
---

# GitLeaks Shield — Secret & Credential Leak Prevention

## Purpose
Scans all outgoing git commits, code snippets, and context logs for sensitive API keys, access tokens, SSH private keys, and passwords, masking them before exposure.

## Monitored Secret Signatures
- GitHub Tokens: ghp_[a-zA-Z0-9]{36}, gho_..., github_pat_...
- AWS Access Keys: AKIA[0-9A-Z]{16}
- OpenAI / Anthropic Keys: sk-[a-zA-Z0-9_-]{32,}
- Private Keys: -----BEGIN (RSA|EC|OPENSSH) PRIVATE KEY-----
- Database URI Credentials: postgres://user:password@host...

## Actions
- Automatically mask detected secrets with [REDACTED_SECRET].
- Prevent accidental commits of unmasked .env or credential files.
