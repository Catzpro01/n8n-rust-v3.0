---
name: pii-sanitizer
description: >-
  Real-time Personally Identifiable Information (PII) detection and redaction engine (Microsoft Presidio & GDPR compliant). Automatically anonymizes emails, phone numbers, IP addresses, and personal identities.
---

# PII Sanitizer — Privacy & Identity Guard

## Purpose
Scans all text, conversation context, and exported artifacts in real-time to detect and redact Personally Identifiable Information (PII) before it is sent to external APIs or stored in public repositories.

## Redaction Patterns
- Email Addresses: [REDACTED_EMAIL]
- Phone Numbers & National IDs: [REDACTED_ID]
- Private IP Addresses & Mac Addresses: [REDACTED_IP]
- Personal Names in Raw Logs: [REDACTED_NAME]

## Guidelines
- Never alter code identifiers, variable names, package imports, or technical parameters.
- Only redact human personal identifiers in prose, error dumps, and user-provided inputs.
