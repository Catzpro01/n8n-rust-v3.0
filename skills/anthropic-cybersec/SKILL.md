---
name: anthropic-cybersec
description: >-
  Cybersecurity playbooks based on NIST frameworks and MITRE ATT&CK. 
  Provides structured security assessment, incident response, and threat 
  modeling guidance for AI coding agents.
---

# Anthropic Cybersecurity Skills

> ⚠️ DUAL-USE WARNING: These playbooks contain offensive security knowledge.
> Only use on systems you own or have explicit written permission to test.
> Unauthorized use is illegal under computer fraud laws worldwide.

## Threat Modeling (STRIDE)

### Spoofing
- Can an attacker impersonate a legitimate user or service?
- Mitigations: MFA, certificate pinning, request signing

### Tampering
- Can an attacker modify data in transit or at rest?
- Mitigations: HMAC signatures, TLS, database integrity constraints

### Repudiation
- Can users deny performing an action?
- Mitigations: Audit logging, digital signatures, non-repudiation controls

### Information Disclosure
- Can an attacker access data they shouldn't?
- Mitigations: Encryption at rest, least privilege, data classification

### Denial of Service
- Can an attacker degrade or stop service?
- Mitigations: Rate limiting, circuit breakers, resource quotas

### Elevation of Privilege
- Can an attacker gain more access than authorized?
- Mitigations: Principle of least privilege, RBAC, input validation

## OWASP Top 10 Checklist (Web Apps)

| # | Vulnerability | Test Method | Mitigation |
|---|--------------|-------------|------------|
| 1 | Broken Access Control | Test auth bypass, IDOR | RBAC, server-side checks |
| 2 | Cryptographic Failures | Check TLS, key storage | Modern TLS, key management |
| 3 | Injection (SQL, XSS, cmd) | Input fuzzing | Parameterized queries, sanitization |
| 4 | Insecure Design | Architecture review | Threat modeling, secure SDLC |
| 5 | Security Misconfiguration | Config audit | Hardening, least privilege |
| 6 | Vulnerable Components | Dependency scan (trivy) | Keep deps updated |
| 7 | Auth Failures | Session testing | MFA, secure session management |
| 8 | Integrity Failures | Supply chain review | Code signing, SRI |
| 9 | Logging Failures | Log audit | Centralized logging, SIEM |
| 10 | SSRF | URL parameter testing | Allowlist outbound URLs |

## Incident Response Playbook

### Phase 1: Detection
- Identify indicators of compromise (IoCs)
- Preserve evidence (do not power off — memory forensics)
- Notify security team immediately

### Phase 2: Containment
- Isolate affected systems from network
- Revoke compromised credentials
- Block attacker IPs/domains at firewall

### Phase 3: Eradication
- Remove malware/backdoors
- Patch exploited vulnerability
- Reset all potentially compromised credentials

### Phase 4: Recovery
- Restore from clean backup
- Monitor for re-infection
- Gradually restore services

### Phase 5: Lessons Learned
- Document timeline and root cause
- Update detection rules
- Improve controls to prevent recurrence
