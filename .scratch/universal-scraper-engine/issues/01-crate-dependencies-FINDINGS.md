# Issue 01 Findings — crate stack cannot be added blind

Recorded by the repository agent after attempting ticket
[01-crate-dependencies.md](01-crate-dependencies.md). None of this is
speculation: every line below is either read from this repository or fetched
from lib.rs / crates.io on 2026-09-15.

## Blocker: no Rust toolchain outside the VPS

The ticket's acceptance criterion is `cargo check --workspace passes cleanly`.
This sandbox has no `cargo` and no `rustc`, and `static.rust-lang.org` is
unreachable from it, so the criterion cannot be evaluated anywhere except the
`rust-check` CI job. Any version pin written here is therefore unverified at
the moment it is written.

That matters more than usual because the workspace pins every dependency with
`=` (see `[workspace.dependencies]` in the root `Cargo.toml`). Exact pins turn
a wrong guess into a hard resolution failure rather than a silent downgrade,
so an unverified pin here is likely to break `cargo check` for every job.

## Finding 1: the acceptance criterion is vacuous in this ticket's scope

Entries under `[workspace.dependencies]` are inherited only when a member crate
declares `dep.workspace = true`. Ticket 03 is what puts the scraper module into
`crates/workflowd`, so after ticket 01 alone no member references any of the new
crates. Cargo never resolves or compiles them, and `cargo check --workspace`
passes without ever touching the new stack.

A green `cargo check` after ticket 01 would therefore prove nothing. Either the
criterion moves to ticket 03, or ticket 01 must also add a member crate that
depends on the stack.

## Finding 2: "without triggering C/C++ build bloat" is already false

`rusqlite` is pinned with `features = ["bundled"]`, which compiles the SQLite C
amalgamation as part of every build. The workspace is not pure Rust today, so
this criterion needs restating as "no *new* C/C++ dependencies" to be testable.

## Finding 3: the stack contradicts the 2-core budget

`.scratch/INFRASTRUCTURE-HANDOFF.md` records the host as **2 vCPU, 3.8 GB RAM**.

- `chromiumoxide` 0.9.1 (lib.rs, released 2026-02-25, edition 2024) generates
  roughly 60K lines of Rust in `chromiumoxide_cdp` and its own README says to
  "expect the compiling to take some time".
- `polars` 0.55.2 (crates.io `max_stable_version`) pulls a very large tree.

Both are pure Rust, so they satisfy the letter of the ticket while putting a
large compile on 2 cores. Worth measuring on the VPS before committing to the
stack, not after.

## Finding 4: `reqwest` itself is missing from the list

The ticket lists `reqwest-middleware` and `reqwest-retry` but not `reqwest`.
Both are wrappers around a `reqwest` client, so `reqwest` must be added too,
and it must be pinned with `default-features = false, features = ["rustls-tls"]`
— the default enables `native-tls`, which links OpenSSL and breaks the pure-Rust
TLS criterion. The same applies to `chromiumoxide`, which requires either the
`rustls` or the `native-tls` feature; only `rustls` is acceptable here.

## Finding 5: BrowserHeadless still needs a browser binary

Ticket 04 aims to remove Playwright from the CI gate, but mode 3
(`BrowserHeadless`) drives Chromium over CDP, so a Chromium binary is still
required at runtime. Removing the Node.js dependency does not remove the
browser. Ticket 04's acceptance criteria should say which binary the native
test uses.

## Versions verified so far

| crate | version | source |
| --- | --- | --- |
| `polars` | 0.55.2 | crates.io `max_stable_version` |
| `chromiumoxide` | 0.9.1 | lib.rs release list |

`spider`, `scraper`, `reqwest`, `reqwest-middleware`, `reqwest-retry`, `backoff`,
`governor`, `moka` and `tokio-cron-scheduler` were **not** resolved. Guessing
them under `=` pinning is the specific failure mode this document exists to
prevent.

## Recommendation

Resolve the remaining versions on a machine with `cargo` (`cargo add --dry-run`
against the real registry, or `cargo search`), then let the `rust-check` CI job
be the verification loop. Do not land exact pins that no toolchain has resolved.
