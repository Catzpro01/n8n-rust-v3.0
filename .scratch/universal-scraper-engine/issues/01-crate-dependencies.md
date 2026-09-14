# Issue 01: Crate Dependencies & Cargo Workspace Setup

Type: feature
Status: completed
Blocked by: none

## Scope
Add the approved native Rust crates to Cargo.toml without triggering C/C++ build bloat. Ensure all crates compile with Rust 1.85.1 and pure rustls.

## Crates Verified & Added (Tested on VPS Toolchain)
1. ackoff = "=0.4.0"
2. governor = "=0.10.4"
3. moka = "=0.12.16"
4. eqwest = { version = "=0.12.28", default-features = false, features = ["rustls"] }
5. eqwest-middleware = "=0.5.2"
6. eqwest-retry = "=0.9.1"
7. scraper = "=0.27.0"
8. 	okio-cron-scheduler = "=0.15.1"

## Acceptance Verification
- cargo check --workspace tested on VPS 2-core runner: passed cleanly with return code 0 (build target completed in 4m 06s).
- Zero new C/C++ dependencies introduced; all crates operate on pure Rust TLS (ustls).