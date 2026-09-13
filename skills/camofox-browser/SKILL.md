---
name: camofox-browser
description: >-
  Anti-detect headless browser (Camofox) skill. Enforces ethical scraping guidelines,
  robots.txt compliance, and rate limiting to prevent IP bans.
---

# Camofox Browser — Anti-Detect Scraping Guardrails

> ⚠️ ETHICAL USE & TOS NOTICE:
> Camofox overrides default browser fingerprints. Use responsibly and in accordance
> with target site Terms of Service and applicable privacy laws.

## Operating Rules

### 1. Rate Limiting & Delays
- Always add randomized delays between navigation requests (2–5 seconds).
- Do not make rapid concurrent requests that mimic a DDoS attack.

### 2. Credential & Data Protection
- Do NOT use Camofox to attempt credential stuffing or unauthorized login bypass.
- Never store session tokens or cookies in unencrypted plain text.

### 3. Robots & ToS Respect
- Check and respect site crawl delay directives.
- Restrict automated scraping to public data.

## Integration Pattern
```python
# Safe invocation pattern with delay
import time
import random

def fetch_page(camofox_client, url):
    time.sleep(random.uniform(2.0, 5.0))  # Polite delay
    return camofox_client.navigate(url)
```
