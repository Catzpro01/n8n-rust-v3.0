# Session handoff — 2026-09-14 (branch `arena/01a0a14f-n8n-rust-v3-0`)

## Branch and push state

- Fixed working branch: `arena/01a0a14f-n8n-rust-v3-0`, branched from
  `24bc3671a38c7687d1efba1c676c998cd6440a1c` (the merge of PR #15 into `main`).
- GitHub auth in this sandbox works (`gh auth status` reports
  `arena-ai-coding-agent[bot]`); pushes and `gh` API reads succeeded.
- No merge has been performed and none is authorized without explicit Owner
  approval.

## What this session changed and why

Ticket 10's remaining item turned out to be already satisfied by CI, and the
actual blocker was a Node-side memory explosion in the last red CI step.

1. **Ticket 10 fault-injection acceptance is verified, not outstanding.**
   `If runtime acceptance` run
   [34881819422](https://github.com/Catzpro01/n8n-rust-v3.0/actions/runs/34881819422)
   (job `if-runtime`, conclusion `success`, 18:42:49Z–19:01:13Z, head
   `46f675bc79530af1ed9304215f7605818b8e129f`) ran
   `python3 -m unittest tests.acceptance.test_if_runtime -v`. The blob at that
   ref contains `test_merge_cancellation_is_terminal_and_traceable` (line 507),
   `test_merge_branch_failure_is_typed_and_traceable` (572), and
   `test_merge_spool_read_failure_is_typed_and_traceable` (621). Ticket 10,
   `map.md`, and `PROJECT-STATUS.md` now record it as complete and Ticket 11 as
   the frontier.
2. **Measured root cause of the exit-137 artifact-generation step.** The kill
   is not host memory. `assert.deepEqual` on two differing Buffers builds a
   byte-by-byte diff synchronously; measured on Node 22.22.3 in this sandbox:
   4,096 differing bytes → 57,405-character message and 572 MB RSS, and from
   16,384 bytes the process is SIGKILLed. The committed baselines are 107,596
   and 74,089 bytes, and the frozen event loop explains the missing
   `generate-ui=heartbeat` lines. `Eco 100K Summarize acceptance` keeps
   Chromium resident through the same 49,998-item run and passes, which is why
   the earlier resident-browser / heap-cap / smaller-count theories changed
   nothing.

## Files changed

- `editor/tests/lib/visual-baseline.mjs` (new): bounded comparison via
  `Buffer#equals`; writes `<name>.actual.png`; bounded message with byte
  lengths, sha256 prefixes, and the first differing offset.
- `editor/tests/lib/visual-baseline.test.mjs` (new): 5 `node:test` cases,
  including the exact 107,596-byte regression.
- `editor/tests/generate-artifact.mjs`: imports the shared comparison; the
  local `assert.deepEqual` helper is gone.
- `editor/package.json`: `test:node` script.
- `.github/workflows/validate.yml`: new `Run dependency-free editor tests`
  step, corrected comment on the artifact-generation step, and an
  `if: failure()` upload of `editor/tests/baselines/*.actual.png`.
- `.gitignore`: ignore rendered `*.actual.png` evidence.
- Status/ticket docs: `.scratch/eco-100k-first-runnable/map.md`, tickets 10
  and 11, `docs/agents/PROJECT-STATUS.md`, this handoff.

## Verification actually run in this checkout

- `npm ci --ignore-scripts`, `npm run test:node` (5/5 pass),
  `npm run typecheck` (exit 0), `npm run build` (exit 0).
- `node --check editor/tests/generate-artifact.mjs`.
- `python3 -m unittest discover -s tests -v` — 3 tests OK.
- YAML parse of all three workflows plus `host-prereqs/action.yml`.
- `git diff --check` — clean.
- Red/green proof: restoring the old `assert.deepEqual` body makes the new test
  file exit with `signal: 'SIGKILL'`; the bounded comparison passes.
- **Not verified here:** `cargo`/`rustc`/`rustfmt` are absent and
  `crates.io`/`static.rust-lang.org` are unreachable from this sandbox
  (`curl` fails with `SSL_ERROR_SYSCALL`), so the Rust gates, the browser
  acceptance suite, and the release bundle remain CI-only claims.

## Next actions

1. Let the pinned `Validate Rust workflow platform` run for this branch reach
   `Browser acceptance: artifact generation`. The single self-hosted runner
   serializes the three workflows; at handoff time it was busy with
   `Eco 100K Summarize acceptance` run `34881819418` and several runs were
   queued, so expect a long wait rather than a stall.
2. If the step now fails with a readable baseline mismatch, download the
   `visual-baseline-actual-<sha>` artifact, review the PNG, and approve it with
   a deliberate `UPDATE_VISUAL_BASELINE=1` commit. Do not weaken the assertion.
3. When the whole Validate workflow is green, promote Ticket 11 in
   `map.md`/`PROJECT-STATUS.md` with the run id, then report to the Owner.
4. Wait for explicit Owner permission before any merge action.

## Scope guardrails

Preserve the frozen Eco six-node topology, complete Ticket 08-A transform,
approved digest and logical-byte vector, 100,000 Activation equation,
private-first/clean-room constraints, and no credential or secret material in
this handoff.
