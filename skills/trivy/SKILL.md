---
name: trivy
description: >-
  Security scanner (Aqua Security) for detecting CVEs, leaked secrets, and
  misconfigurations in containers, filesystems, IaC, and git repositories.
  Apache-2.0, actively maintained.
---

# Trivy — Security Scanner

## Scan Types

### Container Images
```bash
trivy image nginx:latest
trivy image --severity HIGH,CRITICAL myapp:v1.0
```

### Filesystem / Source Code
```bash
trivy fs .
trivy fs --scanners secret,vuln .
```

### Git Repositories
```bash
trivy repo https://github.com/user/repo
trivy repo --branch main .
```

### Infrastructure as Code
```bash
trivy config .
trivy config --tf-vars terraform.tfvars .
```

## Severity Levels
| Level | Action |
|-------|--------|
| CRITICAL | Block deployment immediately |
| HIGH | Fix within 24h or document exception |
| MEDIUM | Fix within sprint |
| LOW | Track in backlog |
| UNKNOWN | Investigate before ignoring |

## Secret Detection
Trivy detects:
- AWS/GCP/Azure credentials
- GitHub/GitLab tokens
- Private keys (RSA, EC, PGP)
- Database connection strings
- Generic API keys

## CI/CD Integration
```bash
# Fail pipeline on CRITICAL
trivy image --exit-code 1 --severity CRITICAL $IMAGE

# Generate SARIF for GitHub
trivy fs --format sarif --output trivy-results.sarif .
```

## Agent Rules
- Run `trivy fs .` before any code commit
- Run `trivy image` before any container deployment
- Never ignore CRITICAL findings without written justification
- Check `trivy repo` when adding new dependencies
