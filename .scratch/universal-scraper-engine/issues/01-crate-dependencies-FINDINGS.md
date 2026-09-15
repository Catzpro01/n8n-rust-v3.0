# Issue 01 Findings — Empirically Verified on VPS Toolchain (Rust 1.85.1)

Documented in response to Matt's verification findings. All versions below were compiled and tested directly in `/tmp/verify-58ffa58` on the 2-core VPS runner.

## Verified Resolutions & Root Causes

### 1. `reqwest` & feature names
- In `reqwest 0.12`, the feature name is `rustls-tls` (not `rustls`).
- In `reqwest 0.13`, transitively pulling `url 2.5.8 -> idna 1.1.0 -> icu 2.3.0` fails under Rust 1.85.1 because `icu 2.3.0` requires rustc 1.88.
- **Resolution**: `reqwest = { version = "=0.12.12", default-features = false, features = ["rustls-tls"] }`, with `reqwest-middleware = "=0.4.1"` and `reqwest-retry = "=0.7.0"`.

### 2. `moka` compilation requirements
- `moka 0.12.16` produces a `compile_error!` if `default-features = false` is specified without explicitly enabling either `sync` or `future`.
- **Resolution**: `moka = { version = "=0.12.16", default-features = false, features = ["future"] }`.

### 3. `scraper` MSRV compliance
- `scraper 0.27.0` uses unstable `let_chains` (`&& let Node::Text(...)`) which fails with compiler error `E0658` on rustc 1.85.1.
- **Resolution**: `scraper = "=0.23.1"` (the latest stable release compatible with Rust 1.85.1).

### 4. `spider` and `polars` MSRV analysis
- `spider 2.53.9` transitively pulls `sysinfo 0.39.6` which explicitly requires **rustc >= 1.95**.
- `polars 0.55.2` transitively pulls `multiversion 0.9.0` and `simd-json` which require **rustc >= 1.86 & 1.88**.
- **Architectural Directive**:
  - Mode 1 (`FastHttp`) and Mode 2 (`DeepCrawl`) do not require the heavy `spider` crate; they are fully achieved with `reqwest` + `scraper` + `governor` + `tokio::sync` + `backoff`.
  - Mode 4 (`DataTransform`) is achieved with `csv = "=1.3.1"` and `serde_json`, avoiding C++ Arrow bloat on 2 vCPU cores.

## Final Compilation Gate
With the above pins, `cargo check --workspace` compiled `workflowd` (with all 8 dependencies enabled as `workspace = true`) in **15.31 seconds** with **return code 0**.
