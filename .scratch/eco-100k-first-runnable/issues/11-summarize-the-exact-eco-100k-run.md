# 11: Summarize the exact Eco 100K Run

**What to build:** Summarize consumes the merged stream, emits count/branch counters and a frozen algorithm-tagged Output Digest, and completes the full six-node Eco journey with exactly 100,000 logical Activations.

**Blocked by:** 10: Merge closed branch streams without unbounded memory

**Status:** `complete` — pinned-verified 2026-09-14T15:45:02Z via `validate` green `34864197586` (commit `7876aa6`), `eco-acceptance` `34864197595` and `if-runtime` `34864197532` both `success`, plus `fast-check` parallel. Ticket 10 fault-injection gate now closed (spool-write `canopy.merge.spool_storage` + spool-read + branch-failure + cancellation).

- [x] Summarize declares Barrier/Reducer Activation Shape, Pure deterministic effects, one output, bounded state/spill policy, and the Output Digest operation.
- [x] The fixture equation is enforced: 1 Manual Trigger + 1 Generate Items + 49,998 Edit Fields + 49,998 If + 1 Merge + 1 Summarize = 100,000 Activations. Compiler validation now rejects any non-exact six-edge Eco topology.
- [x] The independent fixture definition freezes input range, transform, predicate, merge order, canonicalization, branch/total counts, digest algorithm, and expected digest before completion is claimed.
- [x] Summarize verifies total and branch counters and emits a small typed summary without materializing the merged payload in browser memory.
- [x] The browser renders aggregate wall/CPU timing, durable checkpoint progress, branch counts, and Output Digest.
- [x] Causal Trace exposes directly locatable retained Merge segment references and the retained ordinal range.
- [x] Route provenance is typed at the If, Merge, and Summarize seams; malformed or cross-port provenance is rejected instead of being coerced from an untyped JSON field.
- [x] Compiler connection validation rejects incompatible declared port schemas with structured diagnostics.
- [x] Focused regression coverage was added for exact Eco edge rejection, schema incompatibility, typed provenance, concurrent login lockout, timing/segment rendering, and the existing full-transform vector.
- [x] The full browser/API test now asserts aggregate timing and directly locatable retained Merge evidence in addition to create, publish, run, inspect, and non-destructive rollback.
- [x] Rust/workspace and full browser acceptance evidence for this revision is pinned-verified: `validate` `34864197586` (`7876aa6`, `fast-check` + `validate` both `success`, `rust-cache`, `cargo check` 40s, `artifact` skipped), `eco-acceptance` `34864197595` (`test_eco_100k_summary_is_durable_and_rollback_is_non_destructive` + `eco-summarize.mjs` `npm run test:browser:eco`), `if-runtime` `34864197532` (all 9 tests), and `release-bundle` via `make release` (`out/tracer-bundle.tar.gz` + `checksums.sha256`).

## Implementation evidence

- `crates/workflowd/src/compiler.rs` enforces the exact six-node/six-edge topology and typed source/target schema compatibility during compile/preview/publish.
- `crates/workflowd/src/if_node.rs`, `merge.rs`, and `summarize.rs` carry typed If route provenance through the Merge and Summary reducers.
- `crates/workflowd/src/run.rs` persists aggregate wall time, cgroup-v2 CPU deltas when readable, Merge segment references, and Causal Trace links.
- `editor/src/editing-types.ts` and `editor/src/main.tsx` render timing and retained segment links without loading the merged payload.
- `crates/workflowd/src/security.rs` serializes the login read/verify/update state machine and has a concurrent-failure regression test.
- `.github/workflows/validate.yml` runs the workspace build, installs Chromium, and invokes the complete browser acceptance suite before Python tests.

## Verification recorded for this workspace (pinned 2026-09-14T15:45:02Z, commit 7876aa6)

- `npm run typecheck` — passed (validate).
- `npm run build` — passed (validate).
- `node --check tests/eco-summarize.mjs` — passed.
- `cargo fmt --all -- --check` — passed (fast-check + validate).
- `cargo check --workspace --all-targets` — passed (fast-check).
- `cargo test --workspace --locked` — passed (validate, 50+ tests).
- `cargo build --workspace --locked` — passed (validate).
- `python3 -m unittest tests.acceptance.test_if_runtime` — passed 9/9 (if-runtime 34864197532, including 100k + merge spool-write).
- `python3 -m unittest test_eco_100k_summary` — passed (eco-acceptance 34864197595).
- `node tests/eco-summarize.mjs` — passed via `eco-acceptance` `npm run test:browser:eco` (34864197595, 100k activations, digest verified, rollback non-destructive).
- `make release` — verified via `release-bundle` job (`out/tracer-bundle.tar.gz`, `checksums.sha256`, `strip` binary).
- Changed contract JSON files parse successfully.
- `git diff --check` — passed.

Ticket 10's runtime cancellation, branch-failure, and spill/read fault-injection gate is now closed (validate 34864197586), so Ticket 11 is promoted to `complete`.
