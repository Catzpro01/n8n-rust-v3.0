# Issue 01: Crate Dependencies & Cargo Workspace Setup

Type: feature
Status: completed
Blocked by: none

## Scope
Add the approved native Rust crates to `Cargo.toml` without triggering C/C++ build bloat. Ensure all crates compile with Rust 1.85.1 and pure rustls.

## Crates Verified & Active in workflowd
1. `backoff = "=0.4.0"`
2. `governor = "=0.10.4"`
3. `moka = { version = "=0.12.16", default-features = false, features = ["future"] }`
4. `reqwest = { version = "=0.12.12", default-features = false, features = ["rustls-tls"] }`
5. `reqwest-middleware = "=0.4.1"`
6. `reqwest-retry = "=0.7.0"`
7. `scraper = "=0.23.1"`
8. `tokio-cron-scheduler = "=0.15.1"`

## Acceptance Verification
- `crates/workflowd/Cargo.toml` references all 8 crates with `workspace = true`.
- Verified via `cargo check --workspace` on VPS 2-core runner: completed in 15.31s with return code 0.
