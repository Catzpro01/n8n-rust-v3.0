# Run observability and replay

Every Run produces a Causal Trace relating trigger receipt, revision, plan, Activations, retries, suspensions, Envelopes, Artifacts, policy decisions, Secret Lease identifiers, resource usage, external calls, output validation, and errors. Sensitive values are redacted before persistence.

The Run projection exposes aggregate `elapsed_wall_micros` and `cpu_micros`
values. Wall time is measured from executor start to the durable terminal
commit; CPU time is the delta of the readable cgroup-v2 `cpu.stat`
`usage_usec` counter and is explicitly unavailable when that controller cannot
be sampled. Generation progress persists the same timing facts so a browser
restart does not turn a live estimate into authoritative history.

When a reducer retains physical evidence, Causal Trace includes the exact
owner-authorized `ArtifactReference` list and retained ordinal range. These
references are directly locatable from the trace; the browser does not fetch or
materialize the merged item stream merely to render evidence.

Replay starts from a selected Activation using recorded references or user-provided fixtures. External writes are mocked by default; an approved live replay receives new idempotency and audit context so it cannot masquerade as the original attempt.
