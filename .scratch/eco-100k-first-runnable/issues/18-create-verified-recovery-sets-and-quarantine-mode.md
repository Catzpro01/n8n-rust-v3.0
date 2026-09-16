# 18: Create verified Recovery Sets and Quarantine Mode

**What to build:** The Owner can create and verify complete local Recovery Sets, perform a no-side-effect Restore Drill, use the Recovery Kit, and diagnose suspected corruption in Quarantine Mode before approving restoration.

**Blocked by:** 17: Retain, pin, compact, and expire evidence safely

**Status:** complete

- [x] A Recovery Set manifest covers a consistent SQLite snapshot/migration ledger, every referenced Artifact root, encrypted vault metadata, redacted configuration/instance identity, and exact release/plan/contract/profile/component locks.
- [x] Artifact collection is incremental by digest and snapshot references remain pinned until backup completion/verification.
- [x] No plaintext credential, master key, cookie, private Recovery Kit material, or unredacted secret enters backup manifests/logs.
- [x] Every backup receives manifest/digest verification and visible age/health/object-count/size status.
- [x] A full Restore Drill rebuilds temporary isolated state, opens/checks SQLite, Artifacts, vault metadata, migrations/plans/contracts, and known output digests while network triggers and side effects remain disabled.
- [x] Wrong/missing Recovery Kit material and incomplete/corrupt Recovery Sets fail without altering production state.
- [x] Suspected ordinary corruption enters Quarantine Mode, blocks writes/admission/triggers/side effects, preserves suspect evidence, and runs non-destructive diagnosis.
- [x] The Owner sees verified recovery choices and estimated loss and must approve replacement; generic corruption never triggers silent automatic restore.
- [x] Local-only state is clearly not Disaster-Recovery Ready; the status model can become ready only after a reviewed off-site destination and verified kit/set satisfy objectives.
- [x] Local restore and drill timings are measured against the practical target objectives.
