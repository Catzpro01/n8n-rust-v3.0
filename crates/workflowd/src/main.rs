// SPDX-License-Identifier: AGPL-3.0-or-later

mod app;
mod artifact;
mod artifact_http;
mod assets;
mod canonical;
mod cgroup;
mod compiler;
mod config;
mod database;
mod draft;
mod draft_http;
mod edit_fields;
mod error;
mod expression;
mod generate_engine;
mod governor;
mod identity;
mod if_node;
mod merge;
mod owner_http;
mod publication;
mod publication_http;
mod run;
mod run_engine;
mod run_http;
mod security;
mod summarize;
// Issue 04 wires the scraper engine into the run dispatch and the acceptance
// suite. Until then the engine is exercised by its own lib tests, so the
// binary would otherwise report its public surface as dead code.
#[allow(dead_code)]
mod universal_scraper;

use crate::app::AppState;
use crate::config::{ServeConfig, BLOCKING_THREADS_MAX, TOKIO_CORE_WORKERS};
use crate::database::DatabaseWorker;
use crate::governor::Governor;
use crate::error::AppError;
use crate::identity::{ReleaseIdentity, PRODUCT_NAME};
use axum_server::Handle;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Builder;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    if let Err(error) = dispatch() {
        eprintln!(
            "{}",
            serde_json::json!({"event":"startup_failed","error":error.to_string()})
        );
        std::process::exit(1);
    }
}

fn dispatch() -> Result<(), AppError> {
    match env::args().nth(1).as_deref() {
        Some("serve") | None => serve(),
        Some("version") | Some("--version") | Some("-V") => {
            println!("{PRODUCT_NAME} workflowd {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("benchmark-manifest") => benchmark_manifest(),
        Some(command) => Err(AppError::Configuration(format!(
            "unknown command {command:?}; expected serve, benchmark-manifest, or version"
        ))),
    }
}

fn benchmark_manifest() -> Result<(), AppError> {
    let config = ServeConfig::from_environment()?;
    let sample = cgroup::sample(
        config.cgroup_dir.as_deref(),
        Some(&config.state_dir),
        cgroup::DEFAULT_MANAGED_DISK_RESERVE_BYTES,
    );
    let manifest = serde_json::json!({
        "schema": "canopy.benchmark-manifest/v1alpha1",
        "product": PRODUCT_NAME,
        "version": env!("CARGO_PKG_VERSION"),
        "captured_at_epoch_seconds": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "eco_profile": governor::eco_profile_targets(),
        "observed": {
            "cpu_quota_cores": sample.cpu.quota_cores,
            "memory_max_bytes": sample.memory.max_bytes,
            "memory_current_bytes": sample.memory.current_bytes,
            "disk_available_bytes": sample.disk.available_bytes,
            "disk_reserved_bytes": sample.disk.reserved_bytes,
            "cpu_pressure_avg10": sample.pressure.cpu.some_avg10,
            "memory_pressure_avg10": sample.pressure.memory.some_avg10,
            "io_pressure_avg10": sample.pressure.io.some_avg10,
        },
        "accelerator_policy": "off",
        "scheduler": "adaptive-cgroup-aware-weighted-fair",
        "acceptance": {
            "correctness_digest_stable": true,
            "logical_order_deterministic": true,
            "oom_kills_zero": true,
            "throttle_tolerance_pct": 80.0,
        }
    });
    let path = config.state_dir.join("benchmark-profile.json");
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| AppError::Runtime(format!("cannot serialize benchmark manifest: {e}")))?;
    std::fs::write(&path, bytes)
        .map_err(|e| AppError::Runtime(format!("cannot write benchmark manifest: {e}")))?;
    println!(
        "{}",
        serde_json::json!({
            "event": "benchmark_manifest_written",
            "path": path.display().to_string()
        })
    );
    Ok(())
}

fn serve() -> Result<(), AppError> {
    let subscriber = FmtSubscriber::builder()
        .json()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|error| AppError::Runtime(format!("cannot initialize tracing: {error}")))?;

    let config = ServeConfig::from_environment()?;
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| AppError::Runtime("cannot install rustls ring provider".into()))?;

    // Shape the Tokio runtime against the cgroup CPU quota before constructing
    // services, so spawn_blocking and async tasks respect container limits.
    let resource_probe = cgroup::discover(config.cgroup_dir.as_deref(), Some(&config.state_dir));
    let quota_sample = cgroup::sample(
        config.cgroup_dir.as_deref(),
        Some(&config.state_dir),
        cgroup::DEFAULT_MANAGED_DISK_RESERVE_BYTES,
    );
    let core_workers = match quota_sample.cpu.quota_cores {
        Some(cores) if cores <= 0.5 => 1,
        Some(cores) => (cores.ceil() as usize).max(1).min(TOKIO_CORE_WORKERS.max(4)),
        None => TOKIO_CORE_WORKERS,
    };
    let blocking_threads = match quota_sample.cpu.quota_cores {
        Some(cores) if cores <= 1.0 => 2,
        Some(cores) => ((cores * 2.0).ceil() as usize).max(2).min(BLOCKING_THREADS_MAX),
        None => BLOCKING_THREADS_MAX,
    };

    let (database_worker, database_identity) =
        DatabaseWorker::start(&config.state_dir, config.sqlite_min_version)?;
    let security = Arc::new(security::SecurityService::initialize(
        &config.state_dir,
        &config,
    )?);
    let artifacts = Arc::new(
        artifact::ArtifactService::initialize(&config.state_dir, security.clone())
            .map_err(|error| AppError::Database(format!("Artifact schema: {error}")))?,
    );
    let drafts = Arc::new(
        draft::DraftService::initialize(&config)
            .map_err(|error| AppError::Database(format!("draft schema: {error}")))?,
    );
    let publications = Arc::new(
        publication::PublicationService::initialize(&config, security.clone())
            .map_err(|error| AppError::Database(format!("publication schema: {error}")))?,
    );
    let governor = governor::Governor::new(
        config.cgroup_dir.clone(),
        config.state_dir.clone(),
        cgroup::DEFAULT_MANAGED_DISK_RESERVE_BYTES,
    );
    let runs = Arc::new(
        run::RunService::initialize(&config, artifacts.clone(), governor.clone())
            .map_err(|error| AppError::Database(format!("Run schema: {error}")))?,
    );
    let state = AppState {
        _database_worker: Arc::new(database_worker),
        release: ReleaseIdentity::new(database_identity.runtime_version.clone()),
        database: database_identity,
        resources: resource_probe,
        security,
        artifacts,
        drafts,
        publications,
        runs,
        governor,
    };

    let runtime = Builder::new_multi_thread()
        .worker_threads(core_workers)
        .max_blocking_threads(blocking_threads)
        .thread_name("workflowd-core")
        .enable_all()
        .build()
        .map_err(|error| AppError::Runtime(format!("cannot build Tokio runtime: {error}")))?;
    info!(
        event = "tokio_runtime_shaped",
        tokio_core_workers = core_workers,
        blocking_threads_max = blocking_threads
    );
    runtime.block_on(run_server(config, state, core_workers, blocking_threads))
}

async fn run_server(
    config: ServeConfig,
    state: AppState,
    core_workers: usize,
    blocking_threads: usize,
) -> Result<(), AppError> {
    let router = app::router(state);
    let handle = Handle::new();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        info!(event = "shutdown_requested");
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(5)));
    });

    info!(
        event = "server_listening",
        address = %config.bind,
        transport = if config.tls.is_some() { "https" } else { "http" },
        tokio_core_workers = core_workers,
        blocking_threads_max = blocking_threads
    );

    let result = if let Some(tls) = config.tls {
        let rustls =
            axum_server::tls_rustls::RustlsConfig::from_pem_file(tls.certificate, tls.private_key)
                .await
                .map_err(|error| AppError::Server(format!("cannot load TLS identity: {error}")))?;
        axum_server::bind_rustls(config.bind, rustls)
            .handle(handle)
            .serve(router.into_make_service())
            .await
    } else {
        axum_server::bind(config.bind)
            .handle(handle)
            .serve(router.into_make_service())
            .await
    };

    result.map_err(|error| {
        error!(event = "server_failed", error = %error);
        AppError::Server(error.to_string())
    })
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
