# Frozen Decisions and Active Design Draft

## Ticket 07 — frozen Owner decisions

Do not reopen these unless the Owner requests revision.

### 07-A — deterministic item

Each generated logical item is `{index, value, data}` from signed `count`, `start`, and `step`, plus optional fixed JSON `data`. Arithmetic is checked and every item retains provenance.

### 07-B — logical versus physical transport

Each item is one logical Envelope. Adjacent Envelopes may be transported in bounded physical micro-batches without changing item identity, order, provenance, bytes, or correctness digest.

### 07-C — Artifact identity and encryption

Public `artifact_id` is opaque and random. Owner-keyed BLAKE3 provides private dedup identity. Encrypted metadata retains plaintext integrity evidence; a separate digest covers physical ciphertext. Every object gets a random data key and chunked XChaCha20-Poly1305.

### 07-D — crash-safe placement

Use same-filesystem staging, synchronization, atomic placement, directory synchronization, then SQLite reference commit. Leases, quarantine, and a shared mutation barrier prevent cleanup races.

### 07-E — hard limits

- count: 50,000
- inline JSON data: 4 KiB
- micro-batch: 64 items / 256 KiB
- queue: 256 items / 4 MiB
- encryption chunk: 64 KiB
- one Artifact: 16 MiB
- logical output per Generate Activation: 64 MiB
- newly durable unique Artifact content per Generate Activation: 32 MiB
- browser preview: 64 KiB
- Eco fixture: `count=49_998`, `start=0`, `step=1`, `data=null`

### 07-F — checkpoints and replay

Output is progressively checkpointed. Restart regenerates only the deterministic speculative suffix after the durable cursor and cannot duplicate already committed downstream outcomes.

### 07-G — API and editor

Provide same-origin Owner-authorized streaming upload/finalization and metadata/preview/range/content reads. Never expose paths or keys. Editor lazy-load is capped at 64 KiB.

### 07-H — typed outcomes

Queue saturation means backpressure. Invalid hard budgets or overflow mean Permanent Failure. Temporary disk pressure means Durable Suspension. Cancellation means Cancelled. Integrity failures quarantine and fail. Unauthorized reads are non-disclosing 403.

### 07-I — namespace, leases, quarantine

Use a random wrapped Owner namespace seed and domain-separated keys. Staging leases last five minutes and refresh. Safe orphans remain quarantined at least 24 hours. Suspect integrity evidence has no automatic deletion deadline. Young, active, pinned, or referenced content is protected.

## Ticket 08 — proposed, not authorized

The active local ticket records these recommendations. They are **not frozen** until explicit Owner approval.

### 08-A — proposed Eco transform

Retain `{index,value,data}` and add:

- fixed `eco=true`;
- `parity = $json.value % 2 === 0 ? "even" : "odd"`;
- `doubled = $json.value * 2`;
- `label = "eco-" + $json.index`.

### 08-B — proposed assignment model

Use ordered assignments tagged `fixed` or `expression`, explicit JSON path segments, `merge` and `replace` modes, Eco frozen to `merge`, duplicate/conflicting path rejection, and expression reads from the immutable original input rather than partially modified output.

### 08-C — proposed VM subset

JSON-compatible literals; `$json` and `$itemIndex`; field/array-index access; parentheses; unary `!`/`-`; checked `+ - * / %`; strict `=== !==`; typed `< <= > >=`; `&& || ??`; ternary. String concatenation only for two strings. No arbitrary JavaScript, mutation, prototypes, constructors, methods, loops, regex, eval, imports, or host access.

### 08-D — proposed Missing/time/random semantics

Missing is distinct from null. It propagates through `??`; otherwise use produces a structured error. Final Missing omits the target path. No time/random expression in the first contract revision; diagnose such use before publication. Add logical clock/seeded randomness only under a later locked revision.
