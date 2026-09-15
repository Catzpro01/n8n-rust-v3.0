# Project status

**Last updated:** 2026-09-15
**Stage:** recovered implementation baseline; full platform expansion mapped
**Current branch:** `arena/01a0a352-n8n-rust-v3-0`
**Current PR:** none open yet for this branch; the merged predecessor is
[#16](https://github.com/Catzpro01/n8n-rust-v3.0/pull/16)
**Recovered source baseline:** Canopy Workbench / `workflow-rust`

## Destination

Continue the original Rust workflow-automation product rather than restarting
from the earlier empty scaffold. The target is an independently authored,
self-hosted workflow platform with a browser editor, durable revisions,
reliable execution, public compatibility seams, and a small-resource default
profile.

## Current truth

- The four uploaded split parts are present in the branch as
  `workspace-split.zip.001.pdf` through `.004.pdf`.
- Concatenating the parts produced a valid outer ZIP; its inner workspace ZIP
  also passed `unzip -t`.
- The recovered `workflow-rust` implementation has been merged into the
  repository root, including Rust crates, Preact editor, contracts, ADRs,
  operations/security/spec docs, `.scratch` tickets, acceptance tests, and
  release packaging.
- Historical session records are preserved under
  `docs/legacy/session-archive/`. They are evidence, not freshly rerun checks.
- Historical `.ssh` material and other recovery-only secrets were intentionally
  not imported.
- The local sandbox has Node/npm/Python but no `cargo`; the pinned GitHub
  workflow is the Rust/editor verification environment.
- The current context preserves both the earlier Arena decisions and the
  recovered project's canonical glossary and ADR decisions.

## Retained implementation frontier

Tickets 01–07 in `.scratch/eco-100k-first-runnable/` are recorded as complete
by the recovered session evidence. Ticket 08 is implemented and pinned-verified
within ADRs 0057 and 0058, including the bounded v1alpha2 integer-label rule.
Ticket 09's native If slice is complete. **Ticket 10 is complete**: its bounded
native success, empty-branch, Artifact-spooling, cleanup, durable-progress,
restart, and trace slice plus the dedicated runtime cancellation,
one-branch-failure, and spill/read fault-injection acceptance all passed in the
pinned `If runtime acceptance` run `34881819422`. Ticket 11's compiler
topology gate, typed route provenance, aggregate timing, retained-segment
Causal Trace links, login-race fix, UI, and focused tests are implemented in
the current working tree; Ticket 11 is now the frontier and only needs the
pinned full `Validate` gate, including the artifact-generation browser step
that has never completed in CI (see the next section).

- **Ticket 08:** `08-transform-items-with-edit-fields-and-the-safe-expression-vm.md`
- **Status:** `implemented-and-pinned-verified`
- **Expansion map:** `.scratch/canopy-platform-expansion/map.md` and GitHub
  issue [#8](https://github.com/Catzpro01/n8n-rust-v3.0/issues/8)
- **Resolved expansion decisions:** issue #9 is recorded as ADR 0059 for the
  language-neutral contract/lane boundary; issue #10 is recorded as ADR 0060
  for Workflow/Skill Package lifecycle and safe retirement; issue #11 is
  recorded as ADR 0061 for the durable, capability-bound Agent Turn contract;
  issue #12 is recorded as ADR 0062 for verified release slots and extension
  state recovery; issue #13 is recorded as ADR 0063 for the integrated
  private-first dual-lane release gate; research issue #14 is recorded in
  `docs/research/external-agent-package-adapters-2026-09.md`.
- **Next action:** Begin implementation in the approved phase order, starting
  with core completion; future expansion still requires the ADRs, contracts,
  tests, documentation, and release evidence recorded above.

The earlier Wayfinder selection of “architecture spike first” now means a
reversible audit/reconciliation of the recovered Rust + connected Preact
baseline. It does not authorize throwing away the existing implementation or
starting a new frontend framework from scratch.

## Active work

- **Owner:** none
- **Map:** Canopy Workbench full platform expansion, GitHub issue #8.
- **Frontier:** implementation phase 1, core completion; Ticket 11 needs one
  pinned `Validate Rust workflow platform` run whose `Browser acceptance:
  artifact generation` step completes. The expansion decision map (#9–#14) is
  resolved locally through ADRs 0059–0063 and the external-adapter research
  note.
- **Blockers:** local Cargo remains unavailable, so pinned GitHub CI is the Rust
  verification environment; the single self-hosted VPS runner serializes the
  three workflows, so a push queues behind any run already in flight. Hub and
  Agent production work remains ordered after core and extension foundation.

## Universal Scraper engine (2026-09-15)

`.scratch/universal-scraper-engine/` Issue 03 is implemented as
`crates/workflowd/src/universal_scraper.rs`, declared in
`crates/workflowd/src/main.rs`, with the operational description in
`docs/operations/universal-scraper.md`.

- Modes 1 (`FastHttp`) and 2 (`DeepCrawl`) execute: single pass from a
  response body in RAM to tabular rows, no disk serialization, cgroup-bounded
  concurrency behind a `governor` token bucket, and exponential-backoff retry
  on 429/5xx and transport faults with `Retry-After` honoured.
- Modes 3 (`BrowserHeadless`) and 4 (`DataTransform`) stay refused by both the
  contract gate and the engine.
- The mode-4 unlock sequence recorded in the Issue 02 ticket — add `csv` to
  `crates/workflowd/Cargo.toml`, run `cargo check --workspace`, commit the
  regenerated `Cargo.lock`, then flip the gate — still needs a cargo-enabled
  host. This sandbox has no Cargo toolchain and cannot reach crates.io, and
  editing `Cargo.toml` without a regenerated lockfile breaks
  `cargo test --workspace --locked`, which is what commit `248fba8` showed.

## Two separate defects behind the CI red — and a correction

**Correction (2026-09-14, later the same day).** An earlier revision of this
file asserted that host memory was *not* the cause of the
`Browser acceptance: artifact generation` kill. That claim does not hold and is
withdrawn. Step-level evidence collected afterwards shows the runner host
killing jobs at arbitrary points, so the artifact step's `cancelled`
conclusion is **not** established to be the assertion defect described below.

**Defect 1 — real, locally reproduced, fixed.** `assert.deepEqual` on two
differing Buffers renders a byte-by-byte diff synchronously. Measured on Node
22.22.3: 4,096 differing bytes produce a 57,405-character message at 572 MB
RSS, and from 16,384 bytes the process is SIGKILLed (exit 137) with the event
loop frozen. The committed baselines are 107,596 and 74,089 bytes. This is a
genuine bug regardless of CI: a visual mismatch could never be reported. Fix is
`editor/tests/lib/visual-baseline.mjs` (`Buffer#equals`, retain
`<name>.actual.png`, bounded message); red/green was demonstrated by restoring
the old comparison, under which the regression test dies with
`signal: 'SIGKILL'`.

**Defect 2 — the dominant one, infrastructure.** The self-hosted VPS is being
oversubscribed and killing runner agents. Observed on heads `913b9b3` and
`864a825`:

| run | job | step where it died |
| --- | --- | --- |
| 34893577745 | validate | `Set up job` (only step recorded) |
| 34893577745 | release-bundle | failure with **zero** steps recorded |
| 34893577745 | audit | `Audit npm and Rust dependencies` |
| 34893577904 | if-runtime | `Run public If acceptance` |
| 34893577863 | eco-acceptance | `Install pinned Node toolchain` |
| 34888651861 | validate | `Run Rust tests` |

Five unrelated steps plus a job with no steps at all is the runner dying, not a
test failing. Contributing load: `validate` ran three cargo-heavy jobs in
parallel, `if-acceptance` and `eco-acceptance` from the same push ran beside
them, and every push queued a `push` twin and a `pull_request` twin of all
three workflows.

The Owner's commit `b96ce5c` addresses this by collapsing the three workflow
files into one `validate.yml` with five jobs (`validate`, `if-runtime`,
`eco-acceptance`, `audit`, `release-bundle`), fanning `release-bundle` in
behind all four, dropping the duplicate `push: arena/**` trigger, and adding
`workflow_dispatch`. That removes the twin runs and the cross-workflow
collisions. Whether the four fan-out jobs fit the host at once depends on how
many self-hosted runners are registered — this sandbox cannot read
`/actions/runners` (HTTP 403), so that is **unverified here**.

One genuine test failure is also on record and still unexplained: run
`34888653360` (head `864a825`), step `Run Eco browser/API acceptance`,
conclusion `failure`, annotation "Process completed with exit code 1". Its log
was unreachable, which is why both browser steps now tee their output into the
check-run summary.

## Last verified in this checkout

Fresh on branch `arena/01a0a14f-n8n-rust-v3-0` (2026-09-14):

- `cd editor && npm ci --ignore-scripts` — succeeded against the lockfile.
- `npm run test:node` — 5/5 pass (`editor/tests/lib/visual-baseline.test.mjs`).
- `npm run typecheck` — exit 0.
- `npm run build` — exit 0.
- `node --check editor/tests/generate-artifact.mjs` — passed.
- `python3 -m unittest discover -s tests -v` — 3 tests OK.
- All three workflow YAMLs and `host-prereqs/action.yml` parse; the new
  `Run dependency-free editor tests` and `Upload rendered images for visual
  baseline review` steps are present in the Validate job.
- `git diff --check` — clean.
- Red/green proof for the fix: with the previous `assert.deepEqual` body
  restored, the new test file exits with `signal: 'SIGKILL'`; with the bounded
  comparison it passes.
- Not verifiable here: `cargo`, `rustc`, and `rustfmt` are absent and
  `crates.io`/`static.rust-lang.org` are unreachable from this sandbox, so the
  Rust gates and the browser acceptance suite remain CI-only claims.

Earlier evidence retained below.

- Combined four-part archive: `unzip -t` passed for the outer and inner ZIPs.
- `python3 tools/codebase_index.py index` — 218 documents indexed.
- `python3 -m unittest discover -s tests -v` — 3 indexer tests passed after
  the recovery and Ticket 08 foundation changes.
- `node --check` passed for the changed editor build script and browser test.
- JSON/Python metadata checks passed for the changed contract/release paths.
- Pinned GitHub workflow run `34757520909` passed editor build and Rust
  formatting/compilation, then exposed the 08-A/08-C label conflict.
- Pinned GitHub workflow run `34758186650` passed editor typecheck/build, Rust
  formatting, `cargo test --workspace --locked`, and the dependency-free
  repository tests after ADR 0058's v1alpha2 revision.
- Pinned GitHub workflow run `34759106854` passed the durable transform
  regression and public runtime build; run `34759178607` passed the focused
  browser/public-seam test for v1alpha2 configuration diagnostics, 17-item
  transform metrics, Artifact spill, three Activation trace order, and mobile
  overflow.
- `git diff --check` — clean before this status update.
- Pinned run `34779941760` passed compilation, 4 Node Contract tests, 50
  workflowd tests, doc tests, and the three public Merge acceptance cases.
- Final evidence-clean tip runs `34780397609` (public If/Merge acceptance) and
  `34780397639` (editor build/typecheck, rustfmt, workspace tests, and
  dependency-free repository tests) passed; `34779941770` independently passed
  rustfmt before the temporary helper was removed.
- `git diff --check` — clean before this status update.
- Pinned `If runtime acceptance` run `34881819422` (job `if-runtime`, head
  `46f675b`, 18:42:49Z–19:01:13Z) passed the whole public If/Merge module,
  including Ticket 10's runtime cancellation, one-branch-failure, and
  spill/read fault-injection cases; `Eco 100K Summarize acceptance` run
  `34871356127` (head `7cc727c`) passed every step, API and browser.
- The historical Ticket 07 full gate is recorded in
  `docs/legacy/session-archive/VERIFICATION.md`; it has not been re-claimed as
  a fresh local result.

## Resume protocol

A new agent should read, in order:

1. `AGENTS.md`
2. `.agents/skills/arena/SKILL.md`
3. this file
4. `CONTEXT.md`
5. `docs/legal/clean-room-policy.md`
6. `.scratch/eco-100k-first-runnable/map.md`
7. the active ticket and relevant ADRs
8. only then the relevant source paths from `cache.kv` or Codebase Memory

Do not paste recovery secrets or the entire archive into a handoff. Update this
file with durable state, decisions, ownership, blockers, and fresh evidence.
