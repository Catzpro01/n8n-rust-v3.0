---
name: penpot
description: >-
  Open-source collaborative UI/UX design tool (AGPL-3.0). Self-hosted
  alternative to Figma. Supports design tokens, components, and developer
  handoff with CSS export.
---

# Penpot — Open Source Design Platform

> ⚠️ SELF-HOSTED NOTE: Penpot web UI has XSS vulnerabilities via SVG uploads.
> Always keep Penpot updated to latest version.

## Quick Start (Docker)
```bash
git clone https://github.com/penpot/penpot.git
cd penpot
docker compose -f docker/images/docker-compose.yaml up -d

# Access at: http://localhost:9090
# Default: create account on first run
```

## Design System Features
- **Components**: Reusable design elements with variants
- **Design tokens**: Colors, typography, spacing as named variables
- **Grid system**: Configurable columns and baseline grid
- **Prototyping**: Click-through wireframes with transitions
- **Developer handoff**: CSS/SVG export, specs view

## Agent Integration
```python
# Export component specs via Penpot API
import requests

headers = {"Authorization": f"Token {PENPOT_TOKEN}"}
project = requests.get(f"{PENPOT_URL}/api/rpc/command/get-project?id={PROJECT_ID}", 
                       headers=headers).json()
```

## Security Configuration
```yaml
# docker-compose.yaml - secure config
environment:
  PENPOT_FLAGS: "disable-registration"  # No public signups
  PENPOT_SECRET_KEY: "${SECRET_KEY}"    # Strong random key
  PENPOT_SMTP_ENABLED: "false"          # Disable if not needed
```

## Security Checklist
- [ ] Disable public registration (`disable-registration` flag)
- [ ] Set strong `SECRET_KEY` (32+ random chars)
- [ ] Run behind reverse proxy with HTTPS
- [ ] Restrict file upload types to approved formats
- [ ] Regular backups of Penpot database
- [ ] Keep updated — SVG parsing XSS vulnerabilities patched frequently
