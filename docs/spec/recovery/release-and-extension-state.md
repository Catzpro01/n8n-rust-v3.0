# Release and extension-state recovery contract

**Contract:** `canopy.recovery.release-extension/v1alpha1`
**Decision:** ADRs 0055 and 0062

## Release slots

The deployment has immutable `Current` and `Previous` slots and a temporary
verified `Staging` slot. A slot contains one signed Production Bundle and its
Release Manifest. Staging is never public and is removed after acceptance or
failure.

A candidate must pass trust, anti-rollback, bundle, migration/schema,
Recovery Reserve, and active-state compatibility checks before it can enter
Staging. It must boot privately and pass no-side-effect readiness checks before
it becomes Current.

## Upgrade transaction

```text
verify candidate
  -> close Run Admission and public side-effect ingress
  -> finish safely or checkpoint/suspend work
  -> create and verify pre-upgrade Recovery Set
  -> migration preflight and controlled migration
  -> boot Staging privately
  -> readiness: state, Artifact, vault metadata, plans/Blueprints, API/editor
  -> switch Current and retain Previous
```

A required check or Recovery Set failure aborts the transaction. Active Runs
retain their original immutable plan, package, Blueprint, Model Route, and
worker identities; an upgrade never changes their semantics in place.

## Rollback boundary

Before new candidate traffic, startup/migration/readiness failure may trigger
verified restoration and automatic Previous-slot activation. After traffic has
opened, the system must stop admission/side effects and enter Quarantine or
owner-approved Recovery. It must preserve newer state and trace evidence and
must not destructively restore an older snapshot merely to recover availability.

## Recovery Set manifest

A complete set covers:

- consistent SQLite state and migration ledger;
- every referenced immutable Artifact root;
- encrypted vault metadata, never plaintext secret values;
- redacted configuration and instance-identity digests;
- release, plan, Node Contract, Compatibility Profile, Workflow/Skill locks;
- Agent Blueprint, Model Route, and optional-worker identity/manifest locks;
- schema, restore-tool, retention, integrity, and drill evidence.

Immutable content is copied incrementally by digest and pinned while the set is
collected. A set is complete only after every manifest reference verifies.
Unreviewed worker binaries and provider credentials are not Recovery Set
payloads.

## Readiness levels

- **Local recovery capable:** a recent local set verifies and an approved
  local restore path exists.
- **Disaster-Recovery Ready:** the local requirements plus an encrypted
  off-site set, an owner-held separately encrypted Recovery Kit, and a recent
  isolated Restore Drill that disabled ingress, triggers, credentials, and
  side effects.

The product must display the weaker local-only status rather than implying
host-loss readiness.

## Quarantine

Integrity, migration, lock, Artifact, vault, or package corruption stops
writes, Run Admission, schedules, triggers, and side effects. Diagnosis is
non-destructive. Recovery choices, estimated loss, and verified sets are
shown for explicit owner approval. Stale packages/workers remain evidence but
cannot silently receive new authority.
