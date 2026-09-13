# Continuation Runbook

## First action in a future chat

Read, in order:

```text
/home/user/session-archive/SESSION_STATE.md
/home/user/session-archive/USER_INSTRUCTIONS.md
/home/user/session-archive/DECISIONS.md
/home/user/workflow-rust/AGENTS.md
/home/user/workflow-rust/CONTEXT.md
/home/user/workflow-rust/docs/legal/clean-room-policy.md
/home/user/workflow-rust/.scratch/eco-100k-first-runnable/issues/08-transform-items-with-edit-fields-and-the-safe-expression-vm.md
```

For Ticket 08 architecture also read:

```text
docs/adr/0025-compile-compatible-expressions-to-a-safe-rust-vm.md
docs/adr/0053-separate-the-deterministic-engine-from-transactional-storage.md
docs/adr/0054-publish-a-minimal-node-contract-and-staged-conformance-matrix.md
docs/adr/0056-keep-native-node-identities-with-visible-compatibility-aliases.md
```

## Ticket 08 gate

Ask for one explicit confirmation of recommended decisions 08-A through 08-D. Do not implement production code before approval. The Owner skipped two prior UI prompts, so do not claim they were approved.

After approval:

1. update Ticket 08 with frozen decisions and authorization timestamp;
2. sync the decision document to the VPS;
3. create red public-seam/compiler/VM tests first;
4. implement the pure parser/compiler/VM separately from SQLite/HTTP;
5. add the locked Edit Fields Node Contract and compiler validation;
6. extend bounded per-item execution/checkpointing without materializing the stream;
7. preserve Artifact spill/backpressure/cancellation from Ticket 07;
8. add editor configuration/diagnostic UI, activating only relevant design skills;
9. run focused tests, strict Clippy, audit if dependencies change, full `make test`, release acceptance, clean release, systemd smoke, final review, commit, push, and head verification.

## VPS access

Use:

```bash
bash /home/user/session-archive/vps-workflow-ssh 'command here'
```

The wrapper repairs the key mode before connecting. Do not assume the stored file mode remains `0600` between tool calls.

VPS project path:

```text
/home/matt1/projects/workflow-rust
```

Project tool PATH for manual commands:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/node-v22.19.0-linux-x64/bin:$PATH"
```

Use SSH keepalives for long builds:

```text
-o ServerAliveInterval=15 -o ServerAliveCountMax=120
```

## Safe synchronization

The VPS Git tree is authoritative for committed source/build formatting. The workspace tree contains the newer unapproved Ticket 08 decision draft. Do not overwrite that draft when importing the VPS tree.

To recover the complete current workspace source as a Git checkout:

```bash
git clone /home/user/session-archive/workflow-rust-session-snapshot.bundle restored-workflow-rust
```

That bundle is a clearly labeled synthetic snapshot, not the original history. If `workflow-rust-e358023.bundle` is later exported from the VPS, clone it instead for original history, then copy the current Ticket 08 decision document from `/home/user/workflow-rust` into that checkout.

## Known hazards

- Never run `pkill -f` with a full daemon command inside the same SSH shell; it can kill the invoking shell.
- Shared VPS disk contention from another user's package installation previously delayed SQLite startup. Do not kill other users' processes.
- Browser and Python daemon readiness waits are now 20 seconds to tolerate legitimate shared-disk contention.
- `cargo-audit 0.21.2` cannot parse current CVSS 4.0 advisories; use 0.22.2 with the audit-only 1.88 toolchain.
- Local `/home/user` has no usable Cargo/Node toolchain; compile on the VPS.
- Never remove SSH keys by truncating `authorized_keys`; match exact public-key algorithm and blob.
- Keep denied Artifact reads non-disclosing and never expose paths, keys, nonces, dedup identities, or cross-Owner equality.
- Do not allow physical batching/spill to change logical ordering, bytes, provenance, item linking, or digest.
