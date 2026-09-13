---
name: google-agent-skills
description: >-
  Structured agent instruction standards for Google Cloud environments.
  Covers GCP services, BigQuery, GKE, and enterprise coding patterns.
---

# Google Agent Skills — Cloud & Enterprise Patterns

## GCP Resource Naming
Follow Google's naming conventions:
- Projects: `{company}-{env}-{purpose}` (e.g., `acme-prod-backend`)
- Buckets: globally unique, lowercase, hyphens only
- Service accounts: `{role}-sa@{project}.iam.gserviceaccount.com`
- Resources: lowercase with hyphens, max 63 chars

## IAM Least Privilege
```yaml
# GOOD: Minimal binding
bindings:
  - role: roles/storage.objectViewer
    members:
      - serviceAccount:reader-sa@project.iam.gserviceaccount.com
  
# BAD: Over-privileged
bindings:
  - role: roles/editor
    members:
      - allUsers
```

## BigQuery Best Practices
```sql
-- GOOD: Partition + cluster for cost control
CREATE TABLE dataset.events
PARTITION BY DATE(event_time)
CLUSTER BY user_id, event_type
AS SELECT ...;

-- Always use column selection (never SELECT *)
SELECT user_id, event_type, event_time
FROM dataset.events
WHERE DATE(event_time) = '2026-09-07'  -- Partition pruning
  AND user_id = 'u123';                -- Cluster pruning
```

## GKE Security Checklist
- [ ] Workload Identity enabled (no node SA credentials)
- [ ] Network policies restricting pod-to-pod traffic
- [ ] Pod Security Standards enforced (baseline or restricted)
- [ ] Container images from approved registry only
- [ ] No `privileged: true` in pod specs
- [ ] Resource requests and limits set on all containers
- [ ] Secrets stored in Secret Manager, not env vars

## Cloud Run Best Practices
```yaml
# cloud-run.yaml
apiVersion: serving.knative.dev/v1
kind: Service
spec:
  template:
    spec:
      serviceAccountName: myapp-sa  # Least-privilege SA
      containers:
        - image: gcr.io/project/app:v1
          resources:
            limits:
              cpu: "1000m"
              memory: "512Mi"
```
