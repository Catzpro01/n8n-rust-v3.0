# n8n-rust-v3.0 / Canopy Workbench

Canopy Workbench is the recovered, independently implemented Rust workflow
automation baseline for this project. It is intended to provide familiar
workflow concepts without copying n8n source, Enterprise files, UI assets,
icons, product copy, or distinctive trade dress.

The recovered production slice contains a Rust execution daemon with embedded
TypeScript/Preact editor assets, bundled SQLite startup, versioned health,
release, and resource APIs, direct HTTP/HTTPS support, and a hardened native
systemd package. It authors immutable Workflow Revisions, executes pinned plans
durably through bounded Envelopes and encrypted Artifacts, exposes
reconnectable progress and cancellation, and retains checkpointed Causal Trace
evidence. The first six-node Eco fixture executes exactly 100,000 logical
Activations and produces a deterministic Output Digest.

## Continue the recovered project

The previous workspace was split into four uploaded archive parts and restored
from the branch. The source implementation is now at the repository root;
there is no need to restart from an empty scaffold.

Read in this order:

1. `AGENTS.md`
2. `CONTEXT.md`
3. `docs/legal/clean-room-policy.md`
4. `docs/agents/PROJECT-STATUS.md`
5. `.scratch/eco-100k-first-runnable/map.md`
6. the active ticket, currently Ticket 13
7. relevant ADRs under `docs/adr/`

Eco implementation Tickets 01–12 are complete. Ticket 11's exact six-node,
100,000-Activation Summarize journey passed its full pinned gate, and Ticket
12 proved bounded recovery of the same Run after two real `SIGKILL`s. Final
Ticket 12 head `838e1e5` passed the bare-metal `vps-baremetal/fast-ci` webhook
gate via `make check` in 19 seconds. Ticket 13, bounded work and cgroup-profile
scaling, is the current frontier but has not been started.

## Roadmap and delivery

The public phase plan is in [`ROADMAP.md`](ROADMAP.md). GitHub milestones roll
up core delivery, extension lanes, Hub packages, AI Agents, recovery, and the
large-editor/compatibility release gate. Detailed ticket contracts and evidence
remain versioned with the source under `.scratch/`.

## Builder quick start

The build is pinned to Rust 1.85.1 and Node.js 22.19.0.

```bash
make editor
make test
./scripts/build-release.sh
python3 -m unittest tests/acceptance/test_release_bundle.py
```

For Owner bootstrap and recovery-key handling, see
[`docs/operations/owner-bootstrap.md`](docs/operations/owner-bootstrap.md).
The public Run, SSE, cancellation, checkpoint, queue, and Causal Trace
contract is documented in [`docs/run-durability.md`](docs/run-durability.md).
For native installation, resource limits, HTTPS configuration, and
state-preserving uninstall, see
[`docs/operations/install-systemd.md`](docs/operations/install-systemd.md).

## Local codebase discovery

This repository also retains a small dependency-free incremental indexer for
fast navigation. It stores metadata and postings in `cache.kv`, never source
contents.

```bash
python3 tools/codebase_index.py index
python3 tools/codebase_index.py search "workflow execution"
python3 tools/codebase_index.py stats
python3 tools/arena_context.py "<what you are changing>"
```

The index is only a discovery accelerator. Checked-out source, tests, ADRs,
issues, and tickets remain authoritative.

## Tests

```bash
python3 -m unittest discover -s tests -v
make check
make test
```

The current sandbox may not have the pinned Rust toolchain or browser runtime.
When a command cannot run locally, report that limitation and use the approved
GitHub/Rust-enabled verification environment rather than lowering the gate.
