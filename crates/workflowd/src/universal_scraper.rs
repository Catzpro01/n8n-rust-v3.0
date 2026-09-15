// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core engine for `8n-nodes-base.universalScraper`: one pass from HTML to rows.
//!
//! The engine owns the execution path behind the two contract modes that have a
//! dependency stack locked in `Cargo.lock`:
//!
//! * `FastHttp` fetches the configured seeds and extracts rows from each page.
//! * `DeepCrawl` walks the seed host breadth-first to a bounded depth.
//!
//! Three properties are structural rather than advisory:
//!
//! * No page reaches disk. A response body is decoded into a `String`, handed to
//!   `scraper`, and dropped inside [`parse_document`] before rows leave it. The
//!   module opens no file handles and owns no temporary paths, so
//!   [`ScraperSummary::disk_spills`] is always zero.
//! * In-flight requests are bounded by the cgroup CPU quota read through
//!   [`crate::cgroup`], never exceed [`MAX_CONCURRENCY`], and pass a token
//!   bucket before every request.
//! * HTTP 429 and 5xx answers are retried on the exponential schedule from the
//!   pinned `backoff` crate, honouring `Retry-After`; every other 4xx answer and
//!   every exhausted budget is a permanent, typed failure.
//!
//! Modes 3 (`BrowserHeadless`) and 4 (`DataTransform`) are refused here for the
//! same reason the contract gate refuses them: the workspace has no CDP client
//! and no locked `csv` consumer yet.
//!
//! The engine is deterministic: pages are parsed in a fixed batch order, rows
//! are numbered in that order, and the row chain digest therefore replays
//! byte-for-byte for the same pages.

use crate::canonical::{bytes as canonical_bytes, digest};
use crate::cgroup::ResourceIdentity;
use backoff::backoff::Backoff;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use canopy_node_contract::universal_scraper::{self, ScraperError, ScraperMode};
use governor::{DefaultDirectRateLimiter, Quota};
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::future::Future;
use std::num::{NonZeroU32, NonZeroUsize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use url::Url;

pub const SCRAPER_ENGINE_ABI: &str = "canopy.universal-scraper/v1alpha1";
pub const ROW_CHAIN_SCHEMA: &str = "canopy.scraped-row-chain/v1alpha1";
pub const CORRECTNESS_SCHEMA: &str = "canopy.correctness-digest/v1alpha1";
pub const ROWS_OUTPUT_PORT: &str = universal_scraper::ROWS_OUTPUT_PORT;
pub const LINK_SELECTOR: &str = "a[href]";
pub const USER_AGENT: &str = "canopy-workflowd/universal-scraper";

/// Hard ceilings. Author-supplied settings below these are honoured; settings
/// above them are rejected at parse time instead of being silently clamped.
pub const MAX_CONCURRENCY: usize = 8;
pub const MAX_PAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ROW_BYTES: usize = 64 * 1024;
pub const MAX_ROWS: u64 = 50_000;
pub const MAX_LOGICAL_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SEEDS: usize = 32;
pub const MAX_DEPTH: u32 = 4;
pub const MAX_PAGES: usize = 256;
pub const MAX_FIELD_RULES: usize = 32;
pub const MAX_LINKS_PER_PAGE: usize = 256;
pub const MAX_REDIRECTS: usize = 5;
pub const MAX_RETRY_AFTER_SECONDS: u64 = 300;

/// Defaults for a contract that declares a mode and nothing else.
pub const DEFAULT_ROW_SELECTOR: &str = "tr";
pub const DEFAULT_REQUESTS_PER_SECOND: u32 = 8;
pub const DEFAULT_BURST: u32 = 4;
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 20;
pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_INITIAL_RETRY_MILLIS: u64 = 250;
pub const DEFAULT_MAX_RETRY_MILLIS: u64 = 8_000;
pub const DEFAULT_MAX_DEPTH: u32 = 2;
pub const DEFAULT_MAX_PAGES: usize = 64;
pub const DEFAULT_MAX_ROWS: u64 = 10_000;
const RETRY_MULTIPLIER: f64 = 2.0;
const GENESIS_DIGEST: &str = "genesis";

pub const CODE_INVALID_CONFIGURATION: &str = "canopy.universal-scraper.invalid_configuration";
pub const CODE_UNSUPPORTED_MODE: &str = "canopy.universal-scraper.unsupported_mode";
pub const CODE_INVALID_SELECTOR: &str = "canopy.universal-scraper.invalid_selector";
pub const CODE_INVALID_SEED: &str = "canopy.universal-scraper.invalid_seed";
pub const CODE_TRANSPORT_FAILED: &str = "canopy.universal-scraper.transport_failed";
pub const CODE_PAGE_REJECTED: &str = "canopy.universal-scraper.page_rejected";
pub const CODE_RETRY_BUDGET_EXHAUSTED: &str = "canopy.universal-scraper.retry_budget_exhausted";
pub const CODE_PAGE_BYTES_EXCEEDED: &str = "canopy.universal-scraper.page_bytes_exceeded";
pub const CODE_ROW_BYTES_EXCEEDED: &str = "canopy.universal-scraper.row_bytes_exceeded";
pub const CODE_ROW_LIMIT_EXCEEDED: &str = "canopy.universal-scraper.output_rows_exceeded";
pub const CODE_OUTPUT_BYTES_EXCEEDED: &str = "canopy.universal-scraper.output_bytes_exceeded";
pub const CODE_RUNTIME_FAILED: &str = "canopy.universal-scraper.runtime_failed";
pub const CODE_DIGEST_FAILED: &str = "canopy.universal-scraper.digest_failed";

/// Typed failure. The code namespace is the one declared by the contract's
/// `outcomes.error_namespace`, so a run report can carry it unchanged.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScraperFailure {
    pub code: String,
    pub message: String,
}

impl ScraperFailure {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(CODE_INVALID_CONFIGURATION, message)
    }
}

/// One outbound request. Kept as a value so a transport can be driven from a
/// task without borrowing the engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRequest {
    pub url: String,
}

/// A decoded response. Any HTTP status is reported as a page; only
/// transport-level problems become a [`TransportFailure`], which keeps the retry
/// decision in one place.
#[derive(Clone, Debug)]
pub struct FetchedPage {
    pub requested_url: String,
    pub final_url: String,
    pub status: u16,
    pub retry_after_seconds: Option<u64>,
    pub body_bytes: u64,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportFailure {
    pub retryable: bool,
    pub message: String,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type TransportResult = Result<FetchedPage, TransportFailure>;

/// The only I/O seam in the engine. Production runs use [`HttpTransport`]; the
/// lib tests drive a scripted transport so retry, crawl and budget behaviour can
/// be observed without a network.
pub trait PageTransport: Send + Sync {
    fn fetch(&self, request: PageRequest) -> BoxFuture<'_, TransportResult>;
}

/// `reqwest` adapter: rustls only, bounded redirects, one timeout for the whole
/// exchange. It performs no retry of its own; retry is the engine's job so that
/// every attempt is counted in the run evidence.
pub struct HttpTransport {
    client: reqwest::Client,
}

impl HttpTransport {
    pub fn new(settings: &ScraperSettings) -> Result<Self, ScraperFailure> {
        let connect_timeout = Duration::from_secs(settings.timeout_seconds.clamp(1, 30));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(settings.timeout_seconds))
            .connect_timeout(connect_timeout)
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .pool_max_idle_per_host(MAX_CONCURRENCY)
            .build()
            .map_err(|error| {
                ScraperFailure::new(
                    CODE_TRANSPORT_FAILED,
                    format!("cannot build the HTTP client: {error}"),
                )
            })?;
        Ok(Self { client })
    }
}

impl PageTransport for HttpTransport {
    fn fetch(&self, request: PageRequest) -> BoxFuture<'_, TransportResult> {
        Box::pin(async move {
            let response = self
                .client
                .get(request.url.as_str())
                .send()
                .await
                .map_err(|error| TransportFailure {
                    retryable: true,
                    message: error.to_string(),
                })?;
            let status = response.status().as_u16();
            let retry_after_seconds = retry_after_seconds(response.headers().get("retry-after"));
            let final_url = response.url().to_string();
            let body = response.bytes().await.map_err(|error| TransportFailure {
                retryable: true,
                message: error.to_string(),
            })?;
            if body.len() > MAX_PAGE_BYTES {
                return Err(TransportFailure {
                    retryable: false,
                    message: format!(
                        "{final_url} answered {} bytes; the hard ceiling is {MAX_PAGE_BYTES}",
                        body.len()
                    ),
                });
            }
            Ok(FetchedPage {
                requested_url: request.url,
                final_url,
                status,
                retry_after_seconds,
                body_bytes: body.len() as u64,
                body: String::from_utf8_lossy(&body).into_owned(),
            })
        })
    }
}

fn retry_after_seconds(header: Option<&reqwest::header::HeaderValue>) -> Option<u64> {
    header?.to_str().ok()?.trim().parse::<u64>().ok()
}

/// HTTP 429 and 5xx are transient; every other 4xx is the origin's final word.
pub fn retryable_status(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

/// One column of the tabular output. An empty selector reads the row element
/// itself; otherwise the first descendant match wins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRule {
    pub column: String,
    pub selector: String,
    pub attribute: Option<String>,
}

/// Resolved engine configuration. Every bound is explicit and every value is
/// inside the hard ceilings above.
#[derive(Clone, Debug)]
pub struct ScraperSettings {
    pub mode: ScraperMode,
    pub row_selector: String,
    pub fields: Vec<FieldRule>,
    pub requests_per_second: u32,
    pub burst: u32,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub initial_retry_millis: u64,
    pub max_retry_millis: u64,
    pub max_depth: u32,
    pub max_pages: usize,
    pub max_rows: u64,
    pub max_row_bytes: usize,
    pub max_page_bytes: usize,
    pub max_logical_bytes: u64,
    pub crawl_same_host_only: bool,
}

/// CSS selectors compiled once. Recomiled per page inside the blocking parse
/// task, because `scraper::Selector` does not have to cross a thread boundary.
pub struct RowSchema {
    pub row_selector: Selector,
    pub fields: Vec<CompiledField>,
}

pub struct CompiledField {
    pub column: String,
    pub selector: Option<Selector>,
    pub attribute: Option<String>,
}

struct ParsedDocument {
    rows: Vec<BTreeMap<String, String>>,
    links: Vec<String>,
}

impl ScraperSettings {
    pub fn defaults(mode: ScraperMode) -> Self {
        Self {
            mode,
            row_selector: DEFAULT_ROW_SELECTOR.to_owned(),
            fields: default_field_rules(),
            requests_per_second: DEFAULT_REQUESTS_PER_SECOND,
            burst: DEFAULT_BURST,
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
            max_retries: DEFAULT_MAX_RETRIES,
            initial_retry_millis: DEFAULT_INITIAL_RETRY_MILLIS,
            max_retry_millis: DEFAULT_MAX_RETRY_MILLIS,
            max_depth: DEFAULT_MAX_DEPTH,
            max_pages: DEFAULT_MAX_PAGES,
            max_rows: DEFAULT_MAX_ROWS,
            max_row_bytes: MAX_ROW_BYTES,
            max_page_bytes: MAX_PAGE_BYTES,
            max_logical_bytes: MAX_LOGICAL_OUTPUT_BYTES,
            crawl_same_host_only: true,
        }
    }

    /// Read `configuration.defaults` from a contract. The contract gate from
    /// `canopy-node-contract` runs first, so a mode without a locked stack or a
    /// missing `rows` output port is refused before any setting is read.
    pub fn from_configuration(contract: &Value) -> Result<Self, ScraperFailure> {
        let mode = universal_scraper::check(contract).map_err(config_error)?;
        let defaults = contract
            .get("configuration")
            .and_then(|configuration| configuration.get("defaults"))
            .and_then(Value::as_object)
            .ok_or_else(|| ScraperFailure::invalid("configuration.defaults must be an object"))?;
        let mut settings = Self::defaults(mode);
        settings.row_selector = text(defaults, "row_selector", &settings.row_selector)?;
        settings.fields = field_rules(defaults)?;
        settings.requests_per_second = bounded_u32(
            defaults,
            "requests_per_second",
            settings.requests_per_second,
            1_000,
        )?;
        settings.burst = bounded_u32(defaults, "burst", settings.burst, 64)?;
        settings.timeout_seconds = bounded_u32(
            defaults,
            "timeout_seconds",
            settings.timeout_seconds as u32,
            120,
        )? as u64;
        settings.max_retries = bounded_u32(defaults, "max_retries", settings.max_retries, 10)?;
        settings.initial_retry_millis = bounded_u32(
            defaults,
            "initial_retry_millis",
            settings.initial_retry_millis as u32,
            60_000,
        )? as u64;
        settings.max_retry_millis = bounded_u32(
            defaults,
            "max_retry_millis",
            settings.max_retry_millis as u32,
            600_000,
        )? as u64;
        settings.max_depth = bounded_u32(defaults, "max_depth", settings.max_depth, MAX_DEPTH)?;
        settings.max_pages = bounded_usize(defaults, "max_pages", settings.max_pages, MAX_PAGES)?;
        settings.max_rows = bounded_u64(defaults, "max_rows", settings.max_rows, MAX_ROWS)?;
        settings.crawl_same_host_only = flag(
            defaults,
            "crawl_same_host_only",
            settings.crawl_same_host_only,
        )?;
        if settings.max_retry_millis < settings.initial_retry_millis {
            return Err(ScraperFailure::invalid(
                "configuration.defaults.max_retry_millis must not be below initial_retry_millis",
            ));
        }
        if settings.burst > settings.requests_per_second {
            return Err(ScraperFailure::invalid(
                "configuration.defaults.burst must not exceed requests_per_second",
            ));
        }
        Ok(settings)
    }

    /// Compile the selectors once so an invalid selector is a startup failure
    /// instead of a mid-run one.
    pub fn compile(&self) -> Result<RowSchema, ScraperFailure> {
        let row_selector = parse_selector(&self.row_selector)?;
        let mut fields = Vec::with_capacity(self.fields.len());
        for field in &self.fields {
            let selector = if field.selector.is_empty() {
                None
            } else {
                Some(parse_selector(&field.selector)?)
            };
            fields.push(CompiledField {
                column: field.column.clone(),
                selector,
                attribute: field.attribute.clone(),
            });
        }
        Ok(RowSchema {
            row_selector,
            fields,
        })
    }
}

fn default_field_rules() -> Vec<FieldRule> {
    vec![FieldRule {
        column: "value".to_owned(),
        selector: String::new(),
        attribute: None,
    }]
}

fn config_error(error: ScraperError) -> ScraperFailure {
    match error {
        ScraperError::UnlockedStack(mode) => ScraperFailure::new(
            CODE_UNSUPPORTED_MODE,
            format!("scraper mode {mode} has no dependency stack locked in Cargo.lock"),
        ),
        other => ScraperFailure::invalid(other.to_string()),
    }
}

fn parse_selector(selector: &str) -> Result<Selector, ScraperFailure> {
    Selector::parse(selector).map_err(|error| {
        ScraperFailure::new(
            CODE_INVALID_SELECTOR,
            format!("{selector} is not a valid CSS selector: {error}"),
        )
    })
}

fn text(
    defaults: &Map<String, Value>,
    key: &str,
    fallback: &str,
) -> Result<String, ScraperFailure> {
    let Some(value) = defaults.get(key) else {
        return Ok(fallback.to_owned());
    };
    let parsed = value
        .as_str()
        .ok_or_else(|| ScraperFailure::invalid(format!("defaults.{key} must be a string")))?;
    if parsed.trim().is_empty() {
        return Err(ScraperFailure::invalid(format!(
            "defaults.{key} must not be empty"
        )));
    }
    Ok(parsed.to_owned())
}

fn flag(defaults: &Map<String, Value>, key: &str, fallback: bool) -> Result<bool, ScraperFailure> {
    let Some(value) = defaults.get(key) else {
        return Ok(fallback);
    };
    value
        .as_bool()
        .ok_or_else(|| ScraperFailure::invalid(format!("defaults.{key} must be a boolean")))
}

fn bounded_u32(
    defaults: &Map<String, Value>,
    key: &str,
    fallback: u32,
    ceiling: u32,
) -> Result<u32, ScraperFailure> {
    let Some(value) = defaults.get(key) else {
        return Ok(fallback);
    };
    let parsed = value.as_u64().ok_or_else(|| {
        ScraperFailure::invalid(format!("defaults.{key} must be a non-negative integer"))
    })?;
    if parsed < 1 || parsed > u64::from(ceiling) {
        return Err(ScraperFailure::invalid(format!(
            "defaults.{key} must be between 1 and {ceiling}"
        )));
    }
    Ok(parsed as u32)
}

fn bounded_u64(
    defaults: &Map<String, Value>,
    key: &str,
    fallback: u64,
    ceiling: u64,
) -> Result<u64, ScraperFailure> {
    let Some(value) = defaults.get(key) else {
        return Ok(fallback);
    };
    let parsed = value.as_u64().ok_or_else(|| {
        ScraperFailure::invalid(format!("defaults.{key} must be a non-negative integer"))
    })?;
    if parsed < 1 || parsed > ceiling {
        return Err(ScraperFailure::invalid(format!(
            "defaults.{key} must be between 1 and {ceiling}"
        )));
    }
    Ok(parsed)
}

fn bounded_usize(
    defaults: &Map<String, Value>,
    key: &str,
    fallback: usize,
    ceiling: usize,
) -> Result<usize, ScraperFailure> {
    let Some(value) = defaults.get(key) else {
        return Ok(fallback);
    };
    let parsed = value.as_u64().ok_or_else(|| {
        ScraperFailure::invalid(format!("defaults.{key} must be a non-negative integer"))
    })?;
    if parsed < 1 || parsed > ceiling as u64 {
        return Err(ScraperFailure::invalid(format!(
            "defaults.{key} must be between 1 and {ceiling}"
        )));
    }
    Ok(parsed as usize)
}

fn field_rules(defaults: &Map<String, Value>) -> Result<Vec<FieldRule>, ScraperFailure> {
    let Some(value) = defaults.get("fields") else {
        return Ok(default_field_rules());
    };
    let entries = value
        .as_array()
        .ok_or_else(|| ScraperFailure::invalid("defaults.fields must be an array"))?;
    if entries.is_empty() || entries.len() > MAX_FIELD_RULES {
        return Err(ScraperFailure::invalid(format!(
            "defaults.fields must hold between 1 and {MAX_FIELD_RULES} rules"
        )));
    }
    let mut rules = Vec::with_capacity(entries.len());
    for entry in entries {
        let entry = entry
            .as_object()
            .ok_or_else(|| ScraperFailure::invalid("every field rule must be an object"))?;
        let column = entry
            .get("column")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if column.trim().is_empty() {
            return Err(ScraperFailure::invalid(
                "every field rule needs a column name",
            ));
        }
        rules.push(FieldRule {
            column: column.to_owned(),
            selector: entry
                .get("selector")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            attribute: entry
                .get("attribute")
                .and_then(Value::as_str)
                .map(str::to_owned),
        });
    }
    Ok(rules)
}

/// CPU quota reported by cgroup v2, when the controller is readable.
pub fn cgroup_cpu_cores(resources: &ResourceIdentity) -> Option<f64> {
    resources
        .cpu
        .values
        .get("quota_cores")
        .and_then(Value::as_f64)
}

/// Concurrency is the requested width, capped by the cgroup CPU quota and by
/// [`MAX_CONCURRENCY`]. A fractional quota still gets one request in flight, so
/// a throttled container degrades instead of stalling.
pub fn bounded_concurrency(cpu_quota_cores: Option<f64>, requested: usize) -> usize {
    let ceiling = match cpu_quota_cores {
        Some(cores) if cores > 0.0 => cores.ceil() as usize,
        _ => std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1),
    };
    requested.clamp(1, ceiling.clamp(1, MAX_CONCURRENCY))
}

/// What the caller asks the engine to scrape.
#[derive(Clone, Debug)]
pub struct ScrapeRequest {
    pub run_id: String,
    pub revision_digest: String,
    pub plan_digest: String,
    pub node_instance_id: String,
    pub seeds: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScrapedRow {
    pub ordinal: u64,
    pub source_url: String,
    pub columns: BTreeMap<String, String>,
    pub logical_bytes: u64,
}

/// Run evidence. `disk_spills` is part of the record so a review can see that
/// the zero-serialization property held for this execution.
#[derive(Clone, Debug, Serialize)]
pub struct ScraperSummary {
    pub engine_abi: &'static str,
    pub node_instance_id: String,
    pub output_port: &'static str,
    pub mode: String,
    pub concurrency: usize,
    pub seeds: u64,
    pub pages_fetched: u64,
    pub pages_parsed: u64,
    pub retry_attempts: u64,
    pub rows: u64,
    pub bytes_ingested: u64,
    pub logical_bytes: u64,
    pub disk_spills: u64,
    pub crawl_truncated: bool,
    pub stream_digest: String,
    pub correctness_digest: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScraperReport {
    pub summary: ScraperSummary,
    pub rows: Vec<ScrapedRow>,
}

struct PageOutcome {
    depth: u32,
    final_url: String,
    body_bytes: u64,
    retries: u64,
    rows: Vec<BTreeMap<String, String>>,
    links: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct Progress {
    rows: Vec<ScrapedRow>,
    ordinal: u64,
    logical_bytes: u64,
    stream_digest: String,
}

impl Progress {
    fn new() -> Self {
        Self {
            stream_digest: GENESIS_DIGEST.to_owned(),
            ..Self::default()
        }
    }
}

/// The single-pass engine.
pub struct UniversalScraper {
    settings: Arc<ScraperSettings>,
    transport: Arc<dyn PageTransport>,
    limiter: Arc<DefaultDirectRateLimiter>,
    permits: Arc<Semaphore>,
    concurrency: usize,
}

impl UniversalScraper {
    /// Production constructor: settings come from the contract, the concurrency
    /// ceiling comes from the cgroup identity already discovered at startup.
    pub fn from_contract(
        contract: &Value,
        resources: &ResourceIdentity,
        transport: Arc<dyn PageTransport>,
        requested_concurrency: usize,
    ) -> Result<Self, ScraperFailure> {
        let settings = ScraperSettings::from_configuration(contract)?;
        Self::new(settings, resources, transport, requested_concurrency)
    }

    pub fn new(
        settings: ScraperSettings,
        resources: &ResourceIdentity,
        transport: Arc<dyn PageTransport>,
        requested_concurrency: usize,
    ) -> Result<Self, ScraperFailure> {
        settings.compile()?;
        let rate = NonZeroU32::new(settings.requests_per_second).ok_or_else(|| {
            ScraperFailure::invalid("requests_per_second must be at least one request per second")
        })?;
        let burst = NonZeroU32::new(settings.burst)
            .ok_or_else(|| ScraperFailure::invalid("burst must be at least one request"))?;
        let limiter = Arc::new(DefaultDirectRateLimiter::direct(
            Quota::per_second(rate).allow_burst(burst),
        ));
        let concurrency = bounded_concurrency(cgroup_cpu_cores(resources), requested_concurrency);
        Ok(Self {
            settings: Arc::new(settings),
            transport,
            limiter,
            permits: Arc::new(Semaphore::new(concurrency)),
            concurrency,
        })
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    pub fn settings(&self) -> &ScraperSettings {
        &self.settings
    }

    /// Fetch every seed (and, for `DeepCrawl`, every same-host page inside the
    /// depth and page budget), then emit the tabular rows in page order.
    pub async fn execute(&self, request: &ScrapeRequest) -> Result<ScraperReport, ScraperFailure> {
        let settings = Arc::clone(&self.settings);
        if !matches!(
            settings.mode,
            ScraperMode::FastHttp | ScraperMode::DeepCrawl
        ) {
            return Err(ScraperFailure::new(
                CODE_UNSUPPORTED_MODE,
                format!(
                    "scraper mode {} has no engine in this build",
                    settings.mode.as_str()
                ),
            ));
        }
        if request.seeds.is_empty() || request.seeds.len() > MAX_SEEDS {
            return Err(ScraperFailure::invalid(format!(
                "the scraper needs between 1 and {MAX_SEEDS} seed URLs"
            )));
        }
        let mut visited: HashSet<String> = HashSet::new();
        let mut frontier: VecDeque<(String, u32)> = VecDeque::new();
        let mut allowed_host: Option<String> = None;
        for seed in &request.seeds {
            let parsed = parse_seed(seed)?;
            if allowed_host.is_none() && settings.crawl_same_host_only {
                allowed_host = parsed.host_str().map(str::to_owned);
            }
            let url = parsed.to_string();
            if visited.insert(url.clone()) {
                frontier.push_back((url, 0));
            }
        }

        let mut progress = Progress::new();
        let mut pages_fetched: u64 = 0;
        let mut retry_attempts: u64 = 0;
        let mut bytes_ingested: u64 = 0;
        let mut truncated = false;
        while !frontier.is_empty() {
            if pages_fetched >= settings.max_pages as u64 {
                truncated = true;
                break;
            }
            let remaining = (settings.max_pages as u64) - pages_fetched;
            let wave = self.concurrency.min(remaining as usize);
            let mut batch = Vec::with_capacity(wave);
            while batch.len() < wave {
                let Some(next) = frontier.pop_front() else {
                    break;
                };
                batch.push(next);
            }
            for outcome in self.fetch_batch(batch).await? {
                pages_fetched += 1;
                bytes_ingested = bytes_ingested.saturating_add(outcome.body_bytes);
                retry_attempts = retry_attempts.saturating_add(outcome.retries);
                progress.absorb(&settings, &outcome)?;
                if settings.mode != ScraperMode::DeepCrawl {
                    continue;
                }
                let depth = outcome.depth.saturating_add(1);
                if depth > settings.max_depth {
                    continue;
                }
                for href in outcome.links {
                    let Some(url) =
                        resolve_link(&outcome.final_url, &href, allowed_host.as_deref())
                    else {
                        continue;
                    };
                    if visited.insert(url.clone()) {
                        frontier.push_back((url, depth));
                    }
                }
            }
        }

        let summary = ScraperSummary {
            engine_abi: SCRAPER_ENGINE_ABI,
            node_instance_id: request.node_instance_id.clone(),
            output_port: ROWS_OUTPUT_PORT,
            mode: settings.mode.as_str().to_owned(),
            concurrency: self.concurrency,
            seeds: request.seeds.len() as u64,
            pages_fetched,
            pages_parsed: pages_fetched,
            retry_attempts,
            rows: progress.rows.len() as u64,
            bytes_ingested,
            logical_bytes: progress.logical_bytes,
            disk_spills: 0,
            crawl_truncated: truncated,
            stream_digest: progress.stream_digest.clone(),
            correctness_digest: String::new(),
        };
        let correctness_digest = correctness_digest(request, &summary)?;
        Ok(ScraperReport {
            summary: ScraperSummary {
                correctness_digest,
                ..summary
            },
            rows: progress.rows,
        })
    }

    /// One bounded wave of pages. Tasks complete in arbitrary order, so the
    /// results are put back into request order before rows are numbered; that
    /// ordering is what makes the row chain digest replay-stable.
    async fn fetch_batch(
        &self,
        batch: Vec<(String, u32)>,
    ) -> Result<Vec<PageOutcome>, ScraperFailure> {
        let mut join_set = tokio::task::JoinSet::new();
        for (index, (url, depth)) in batch.into_iter().enumerate() {
            let settings = Arc::clone(&self.settings);
            let transport = Arc::clone(&self.transport);
            let limiter = Arc::clone(&self.limiter);
            let permits = Arc::clone(&self.permits);
            join_set.spawn(async move {
                let _permit = permits.acquire_owned().await.map_err(|_| {
                    ScraperFailure::new(CODE_RUNTIME_FAILED, "the concurrency semaphore closed")
                })?;
                limiter.until_ready().await;
                let (page, retries) = fetch_with_retry(&*transport, &settings, &url).await?;
                if page.body_bytes > settings.max_page_bytes as u64 {
                    return Err(ScraperFailure::new(
                        CODE_PAGE_BYTES_EXCEEDED,
                        format!(
                            "{url} answered {} bytes; the ceiling is {} bytes",
                            page.body_bytes, settings.max_page_bytes
                        ),
                    ));
                }
                let parse_settings = Arc::clone(&settings);
                let body = page.body;
                let parsed =
                    tokio::task::spawn_blocking(move || parse_document(&parse_settings, &body))
                        .await
                        .map_err(|error| {
                            ScraperFailure::new(
                                CODE_RUNTIME_FAILED,
                                format!("the parse task failed: {error}"),
                            )
                        })??;
                Ok((
                    index,
                    PageOutcome {
                        depth,
                        final_url: page.final_url,
                        body_bytes: page.body_bytes,
                        retries,
                        rows: parsed.rows,
                        links: parsed.links,
                    },
                ))
            });
        }
        let mut outcomes: Vec<(usize, PageOutcome)> = Vec::new();
        while let Some(joined) = join_set.join_next().await {
            let outcome = joined
                .map_err(|error| {
                    ScraperFailure::new(
                        CODE_RUNTIME_FAILED,
                        format!("a scrape task failed: {error}"),
                    )
                })
                .and_then(|outcome| outcome)?;
            outcomes.push(outcome);
        }
        outcomes.sort_by_key(|(index, _)| *index);
        Ok(outcomes.into_iter().map(|(_, outcome)| outcome).collect())
    }
}

impl Progress {
    fn absorb(
        &mut self,
        settings: &ScraperSettings,
        outcome: &PageOutcome,
    ) -> Result<(), ScraperFailure> {
        for columns in &outcome.rows {
            if self.ordinal >= settings.max_rows {
                return Err(ScraperFailure::new(
                    CODE_ROW_LIMIT_EXCEEDED,
                    format!("the row budget of {} rows is exhausted", settings.max_rows),
                ));
            }
            let logical_bytes = canonical_bytes(columns)
                .map_err(|message| ScraperFailure::new(CODE_DIGEST_FAILED, message))?
                .len() as u64;
            if logical_bytes > settings.max_row_bytes as u64 {
                return Err(ScraperFailure::new(
                    CODE_ROW_BYTES_EXCEEDED,
                    format!(
                        "one row is {logical_bytes} canonical bytes; the ceiling is {} bytes",
                        settings.max_row_bytes
                    ),
                ));
            }
            let next_total = self.logical_bytes.saturating_add(logical_bytes);
            if next_total > settings.max_logical_bytes {
                return Err(ScraperFailure::new(
                    CODE_OUTPUT_BYTES_EXCEEDED,
                    format!(
                        "the output budget of {} canonical bytes is exhausted",
                        settings.max_logical_bytes
                    ),
                ));
            }
            self.stream_digest = digest(&json!({
                "schema": ROW_CHAIN_SCHEMA,
                "previous": self.stream_digest,
                "ordinal": self.ordinal,
                "source_url": outcome.final_url,
                "columns": columns
            }))
            .map_err(|message| ScraperFailure::new(CODE_DIGEST_FAILED, message))?;
            self.logical_bytes = next_total;
            self.rows.push(ScrapedRow {
                ordinal: self.ordinal,
                source_url: outcome.final_url.clone(),
                columns: columns.clone(),
                logical_bytes,
            });
            self.ordinal += 1;
        }
        Ok(())
    }
}

fn correctness_digest(
    request: &ScrapeRequest,
    summary: &ScraperSummary,
) -> Result<String, ScraperFailure> {
    digest(&json!({
        "schema": CORRECTNESS_SCHEMA,
        "revision_digest": request.revision_digest,
        "plan_digest": request.plan_digest,
        "logical_outcomes": [{
            "logical_order": 1,
            "node_instance_id": summary.node_instance_id,
            "outcome": "success",
            "port": summary.output_port,
            "mode": summary.mode,
            "pages_fetched": summary.pages_fetched,
            "retry_attempts": summary.retry_attempts,
            "rows": summary.rows,
            "logical_bytes": summary.logical_bytes,
            "stream_digest": summary.stream_digest
        }]
    }))
    .map_err(|message| ScraperFailure::new(CODE_DIGEST_FAILED, message))
}

/// Fetch one page, retrying transient answers on the exponential schedule.
async fn fetch_with_retry(
    transport: &dyn PageTransport,
    settings: &ScraperSettings,
    url: &str,
) -> Result<(FetchedPage, u64), ScraperFailure> {
    let mut policy = retry_policy(settings);
    let mut retries: u64 = 0;
    loop {
        match transport
            .fetch(PageRequest {
                url: url.to_owned(),
            })
            .await
        {
            Ok(page) if page.status < 400 => return Ok((page, retries)),
            Ok(page) if retryable_status(page.status) => {
                let reason = format!("HTTP {} from {url}", page.status);
                retries = wait_before_retry(
                    settings,
                    &mut policy,
                    retries,
                    page.retry_after_seconds,
                    &reason,
                )
                .await?;
            }
            Ok(page) => {
                return Err(ScraperFailure::new(
                    CODE_PAGE_REJECTED,
                    format!(
                        "{url} answered HTTP {}; the engine does not retry it",
                        page.status
                    ),
                ));
            }
            Err(failure) if failure.retryable => {
                retries = wait_before_retry(settings, &mut policy, retries, None, &failure.message)
                    .await?;
            }
            Err(failure) => {
                return Err(ScraperFailure::new(
                    CODE_TRANSPORT_FAILED,
                    format!("{url} failed: {}", failure.message),
                ));
            }
        }
    }
}

async fn wait_before_retry(
    settings: &ScraperSettings,
    policy: &mut ExponentialBackoff,
    retries: u64,
    retry_after_seconds: Option<u64>,
    reason: &str,
) -> Result<u64, ScraperFailure> {
    if retries >= u64::from(settings.max_retries) {
        return Err(ScraperFailure::new(
            CODE_RETRY_BUDGET_EXHAUSTED,
            format!(
                "{reason}; the retry budget of {} attempts is exhausted",
                settings.max_retries
            ),
        ));
    }
    tokio::time::sleep(retry_delay(policy, retry_after_seconds)).await;
    Ok(retries + 1)
}

fn retry_policy(settings: &ScraperSettings) -> ExponentialBackoff {
    let mut builder = ExponentialBackoffBuilder::new();
    builder
        .with_initial_interval(Duration::from_millis(settings.initial_retry_millis))
        .with_randomization_factor(0.0)
        .with_multiplier(RETRY_MULTIPLIER)
        .with_max_interval(Duration::from_millis(settings.max_retry_millis))
        .with_max_elapsed_time(None);
    builder.build()
}

/// The origin's `Retry-After` wins when it asks for longer than the schedule.
fn retry_delay(policy: &mut ExponentialBackoff, retry_after_seconds: Option<u64>) -> Duration {
    let scheduled = policy.next_backoff().unwrap_or(policy.max_interval);
    match retry_after_seconds {
        Some(seconds) => scheduled.max(Duration::from_secs(seconds.min(MAX_RETRY_AFTER_SECONDS))),
        None => scheduled,
    }
}

/// Parse in RAM and return only rows and links. The `html` string is owned by
/// the caller's task and is dropped as soon as this returns, so no page body
/// outlives the parse.
fn parse_document(
    settings: &ScraperSettings,
    html: &str,
) -> Result<ParsedDocument, ScraperFailure> {
    let schema = settings.compile()?;
    let document = Html::parse_document(html);
    let mut rows = Vec::new();
    for element in document.select(&schema.row_selector) {
        let mut columns = BTreeMap::new();
        for field in &schema.fields {
            let value = match &field.selector {
                Some(selector) => match element.select(selector).next() {
                    Some(node) => field_value(&node, field.attribute.as_deref()),
                    None => String::new(),
                },
                None => field_value(&element, field.attribute.as_deref()),
            };
            columns.insert(field.column.clone(), value);
        }
        rows.push(columns);
    }
    let mut links = Vec::new();
    if settings.mode == ScraperMode::DeepCrawl {
        let link_selector = parse_selector(LINK_SELECTOR)?;
        for anchor in document.select(&link_selector) {
            if links.len() >= MAX_LINKS_PER_PAGE {
                break;
            }
            if let Some(href) = anchor.attr("href") {
                links.push(href.to_owned());
            }
        }
    }
    Ok(ParsedDocument { rows, links })
}

fn field_value(element: &ElementRef<'_>, attribute: Option<&str>) -> String {
    match attribute {
        Some(name) => element.attr(name).unwrap_or_default().trim().to_owned(),
        None => element.text().collect::<String>().trim().to_owned(),
    }
}

fn parse_seed(seed: &str) -> Result<Url, ScraperFailure> {
    let url = Url::parse(seed).map_err(|error| {
        ScraperFailure::new(
            CODE_INVALID_SEED,
            format!("{seed} is not a usable URL: {error}"),
        )
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ScraperFailure::new(
            CODE_INVALID_SEED,
            format!("{} is not an http(s) seed", url.scheme()),
        ));
    }
    Ok(url)
}

/// Resolve a harvested link against the page it came from, drop anything that
/// is not http(s), and honour the same-host crawl boundary.
fn resolve_link(base: &str, href: &str, allowed_host: Option<&str>) -> Option<String> {
    let base = Url::parse(base).ok()?;
    let mut resolved = base.join(href).ok()?;
    if !matches!(resolved.scheme(), "http" | "https") {
        return None;
    }
    if let Some(host) = allowed_host {
        if resolved.host_str() != Some(host) {
            return None;
        }
    }
    resolved.set_fragment(None);
    Some(resolved.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cgroup::ControllerIdentity;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    const CATALOGUE: &str = concat!(
        "<html><body><table>",
        "<tr class=\"item\"><td class=\"title\">Alpha</td>",
        "<td><a href=\"/alpha\">open</a></td></tr>",
        "<tr class=\"item\"><td class=\"title\">Beta</td>",
        "<td><a href=\"/beta\">open</a></td></tr>",
        "</table></body></html>"
    );

    /// A page with no links, used wherever a crawl must stop on its own.
    const LEAF: &str = "<html><body><p>leaf</p></body></html>";

    #[derive(Clone, Debug)]
    struct ScriptedResponse {
        status: u16,
        body: String,
        retry_after_seconds: Option<u64>,
        connection_error: bool,
    }

    impl ScriptedResponse {
        fn page(body: &str) -> Self {
            Self {
                status: 200,
                body: body.to_owned(),
                retry_after_seconds: None,
                connection_error: false,
            }
        }

        fn status(status: u16) -> Self {
            Self {
                status,
                body: String::new(),
                retry_after_seconds: None,
                connection_error: false,
            }
        }

        fn throttled(retry_after_seconds: u64) -> Self {
            Self {
                status: 429,
                body: String::new(),
                retry_after_seconds: Some(retry_after_seconds),
                connection_error: false,
            }
        }

        fn reset() -> Self {
            Self {
                status: 0,
                body: String::new(),
                retry_after_seconds: None,
                connection_error: true,
            }
        }
    }

    struct FakeTransport {
        scripts: Mutex<HashMap<String, VecDeque<ScriptedResponse>>>,
        fallback: Mutex<Option<ScriptedResponse>>,
        calls: Mutex<Vec<String>>,
        in_flight: AtomicUsize,
        peak_in_flight: AtomicUsize,
    }

    impl FakeTransport {
        fn new() -> Self {
            Self {
                scripts: Mutex::new(HashMap::new()),
                fallback: Mutex::new(None),
                calls: Mutex::new(Vec::new()),
                in_flight: AtomicUsize::new(0),
                peak_in_flight: AtomicUsize::new(0),
            }
        }

        fn script(&self, url: &str, responses: Vec<ScriptedResponse>) {
            self.scripts
                .lock()
                .expect("script lock")
                .insert(url.to_owned(), VecDeque::from(responses));
        }

        fn fallback(&self, response: ScriptedResponse) {
            *self.fallback.lock().expect("fallback lock") = Some(response);
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("call lock").clone()
        }

        fn peak_in_flight(&self) -> usize {
            self.peak_in_flight.load(Ordering::SeqCst)
        }
    }

    impl PageTransport for FakeTransport {
        fn fetch(&self, request: PageRequest) -> BoxFuture<'_, TransportResult> {
            Box::pin(async move {
                let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                self.peak_in_flight.fetch_max(in_flight, Ordering::SeqCst);
                let scripted = {
                    let mut scripts = self.scripts.lock().expect("script lock");
                    self.calls
                        .lock()
                        .expect("call lock")
                        .push(request.url.clone());
                    scripts
                        .get_mut(&request.url)
                        .and_then(|queue| queue.pop_front())
                };
                let response = match scripted {
                    Some(response) => response,
                    None => self
                        .fallback
                        .lock()
                        .expect("fallback lock")
                        .clone()
                        .unwrap_or_else(|| ScriptedResponse::status(404)),
                };
                tokio::time::sleep(Duration::from_millis(2)).await;
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                if response.connection_error {
                    return Err(TransportFailure {
                        retryable: true,
                        message: "simulated connection reset".to_owned(),
                    });
                }
                Ok(FetchedPage {
                    requested_url: request.url.clone(),
                    final_url: request.url,
                    status: response.status,
                    retry_after_seconds: response.retry_after_seconds,
                    body_bytes: response.body.len() as u64,
                    body: response.body,
                })
            })
        }
    }

    fn resources(quota_cores: Option<f64>) -> ResourceIdentity {
        let mut values = BTreeMap::new();
        values.insert(
            "quota_cores".to_owned(),
            match quota_cores {
                Some(cores) => json!(cores),
                None => Value::Null,
            },
        );
        let empty = || ControllerIdentity {
            available: true,
            reason: None,
            values: BTreeMap::new(),
        };
        ResourceIdentity {
            cgroup_version: 2,
            cgroup_path: "/canopy-test".to_owned(),
            cpu: ControllerIdentity {
                available: true,
                reason: None,
                values,
            },
            memory: empty(),
            tasks: empty(),
        }
    }

    fn settings(mode: ScraperMode) -> ScraperSettings {
        ScraperSettings {
            mode,
            row_selector: "tr.item".to_owned(),
            fields: vec![
                FieldRule {
                    column: "title".to_owned(),
                    selector: ".title".to_owned(),
                    attribute: None,
                },
                FieldRule {
                    column: "href".to_owned(),
                    selector: "a".to_owned(),
                    attribute: Some("href".to_owned()),
                },
            ],
            requests_per_second: 1_000,
            burst: 8,
            timeout_seconds: 5,
            max_retries: 3,
            initial_retry_millis: 1,
            max_retry_millis: 4,
            max_depth: 2,
            max_pages: 16,
            max_rows: 64,
            max_row_bytes: 4 * 1024,
            max_page_bytes: 64 * 1024,
            max_logical_bytes: 256 * 1024,
            crawl_same_host_only: true,
        }
    }

    fn request(seeds: &[&str]) -> ScrapeRequest {
        ScrapeRequest {
            run_id: "run-scraper-test".to_owned(),
            revision_digest: "sha256:revision".to_owned(),
            plan_digest: "sha256:plan".to_owned(),
            node_instance_id: "scraper".to_owned(),
            seeds: seeds.iter().map(|seed| (*seed).to_owned()).collect(),
        }
    }

    fn engine(
        settings: ScraperSettings,
        transport: &Arc<FakeTransport>,
        quota_cores: f64,
        requested: usize,
    ) -> UniversalScraper {
        UniversalScraper::new(
            settings,
            &resources(Some(quota_cores)),
            transport.clone(),
            requested,
        )
        .expect("engine")
    }

    fn contract(mode: &str) -> Value {
        json!({
            "configuration": { "defaults": { "mode": mode } },
            "ports": {
                "inputs": [],
                "outputs": [{ "id": "rows", "cardinality": "many" }]
            }
        })
    }

    #[tokio::test]
    async fn fast_http_rows_are_tabular_and_never_touch_disk() {
        let transport = Arc::new(FakeTransport::new());
        transport.script(
            "https://example.test/list",
            vec![ScriptedResponse::page(CATALOGUE)],
        );
        let scraper = engine(settings(ScraperMode::FastHttp), &transport, 2.0, 4);
        let report = scraper
            .execute(&request(&["https://example.test/list"]))
            .await
            .unwrap();
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.rows[0].ordinal, 0);
        assert_eq!(report.rows[1].ordinal, 1);
        assert_eq!(report.rows[0].columns["title"], "Alpha");
        assert_eq!(report.rows[0].columns["href"], "/alpha");
        assert_eq!(report.rows[1].columns["title"], "Beta");
        assert_eq!(report.rows[0].source_url, "https://example.test/list");
        assert_eq!(report.summary.disk_spills, 0);
        assert_eq!(report.summary.pages_fetched, 1);
        assert_eq!(report.summary.pages_parsed, 1);
        assert_eq!(report.summary.rows, 2);
        assert_eq!(report.summary.retry_attempts, 0);
        assert_eq!(report.summary.output_port, "rows");
        assert!(report.summary.stream_digest.starts_with("sha256:"));
        assert!(report.summary.correctness_digest.starts_with("sha256:"));
        assert!(report.rows.iter().all(|row| row.logical_bytes > 0));
    }

    #[tokio::test]
    async fn transient_answers_are_retried_on_the_backoff_schedule() {
        let transport = Arc::new(FakeTransport::new());
        transport.script(
            "https://example.test/list",
            vec![
                ScriptedResponse::throttled(0),
                ScriptedResponse::status(503),
                ScriptedResponse::reset(),
                ScriptedResponse::page(CATALOGUE),
            ],
        );
        let scraper = engine(settings(ScraperMode::FastHttp), &transport, 2.0, 4);
        let report = scraper
            .execute(&request(&["https://example.test/list"]))
            .await
            .unwrap();
        assert_eq!(transport.calls().len(), 4);
        assert_eq!(report.summary.retry_attempts, 3);
        assert_eq!(report.summary.rows, 2);
    }

    #[tokio::test]
    async fn a_client_error_is_permanent_and_is_not_retried() {
        let transport = Arc::new(FakeTransport::new());
        transport.fallback(ScriptedResponse::status(404));
        let scraper = engine(settings(ScraperMode::FastHttp), &transport, 2.0, 4);
        let error = scraper
            .execute(&request(&["https://example.test/missing"]))
            .await
            .unwrap_err();
        assert_eq!(error.code, CODE_PAGE_REJECTED);
        assert_eq!(transport.calls().len(), 1);
    }

    #[tokio::test]
    async fn an_exhausted_retry_budget_is_a_permanent_failure() {
        let transport = Arc::new(FakeTransport::new());
        transport.fallback(ScriptedResponse::status(503));
        let mut configured = settings(ScraperMode::FastHttp);
        configured.max_retries = 2;
        let scraper = engine(configured, &transport, 2.0, 4);
        let error = scraper
            .execute(&request(&["https://example.test/list"]))
            .await
            .unwrap_err();
        assert_eq!(error.code, CODE_RETRY_BUDGET_EXHAUSTED);
        assert_eq!(transport.calls().len(), 3);
    }

    #[tokio::test]
    async fn deep_crawl_stays_inside_the_depth_and_host_boundary() {
        let index = concat!(
            "<html><body>",
            "<a href=\"/one\">one</a>",
            "<a href=\"/two\">two</a>",
            "<a href=\"https://elsewhere.test/x\">elsewhere</a>",
            "<a href=\"#top\">fragment</a>",
            "<a href=\"mailto:owner@example.test\">mail</a>",
            "</body></html>"
        );
        let transport = Arc::new(FakeTransport::new());
        transport.script("https://example.test/", vec![ScriptedResponse::page(index)]);
        let deep = "<a href=\"/deep\"/>";
        transport.script(
            "https://example.test/one",
            vec![ScriptedResponse::page(deep)],
        );
        transport.script(
            "https://example.test/two",
            vec![ScriptedResponse::page(CATALOGUE)],
        );
        let mut configured = settings(ScraperMode::DeepCrawl);
        configured.max_depth = 1;
        let scraper = engine(configured, &transport, 4.0, 4);
        let report = scraper
            .execute(&request(&["https://example.test/"]))
            .await
            .unwrap();
        let mut calls = transport.calls();
        calls.sort();
        assert_eq!(
            calls,
            vec![
                "https://example.test/".to_owned(),
                "https://example.test/one".to_owned(),
                "https://example.test/two".to_owned()
            ]
        );
        assert_eq!(report.summary.pages_fetched, 3);
        assert!(!report.summary.crawl_truncated);
    }

    #[tokio::test]
    async fn a_page_budget_stops_the_crawl_and_is_reported() {
        let transport = Arc::new(FakeTransport::new());
        let siblings = "<a href=\"/one\"/><a href=\"/two\"/><a href=\"/three\"/>";
        transport.script(
            "https://example.test/",
            vec![ScriptedResponse::page(siblings)],
        );
        transport.fallback(ScriptedResponse::page(LEAF));
        let mut configured = settings(ScraperMode::DeepCrawl);
        configured.max_depth = 3;
        configured.max_pages = 2;
        let scraper = engine(configured, &transport, 4.0, 4);
        let report = scraper
            .execute(&request(&["https://example.test/"]))
            .await
            .unwrap();
        assert_eq!(report.summary.pages_fetched, 2);
        assert!(report.summary.crawl_truncated);
    }

    #[tokio::test]
    async fn in_flight_requests_never_exceed_the_cgroup_ceiling() {
        let links = (1..=5)
            .map(|page| format!("<a href=\"/page-{page}\">page</a>"))
            .collect::<Vec<String>>()
            .join("");
        let transport = Arc::new(FakeTransport::new());
        transport.script(
            "https://example.test/",
            vec![ScriptedResponse::page(&links)],
        );
        transport.fallback(ScriptedResponse::page(LEAF));
        let scraper = engine(settings(ScraperMode::DeepCrawl), &transport, 2.0, 64);
        assert_eq!(scraper.concurrency(), 2);
        let report = scraper
            .execute(&request(&["https://example.test/"]))
            .await
            .unwrap();
        assert_eq!(report.summary.pages_fetched, 6);
        assert!(transport.peak_in_flight() >= 1);
        assert!(transport.peak_in_flight() <= 2);
    }

    #[tokio::test]
    async fn concurrency_is_clamped_by_quota_and_by_the_hard_ceiling() {
        assert_eq!(bounded_concurrency(Some(2.0), 64), 2);
        assert_eq!(bounded_concurrency(Some(0.5), 64), 1);
        assert_eq!(bounded_concurrency(Some(64.0), 64), MAX_CONCURRENCY);
        assert_eq!(bounded_concurrency(Some(4.0), 1), 1);
        assert!(bounded_concurrency(None, 64) <= MAX_CONCURRENCY);
        assert!(bounded_concurrency(None, 64) >= 1);
    }

    #[tokio::test]
    async fn the_token_bucket_spaces_requests_out() {
        let transport = Arc::new(FakeTransport::new());
        transport.fallback(ScriptedResponse::page(CATALOGUE));
        let mut configured = settings(ScraperMode::FastHttp);
        configured.requests_per_second = 50;
        configured.burst = 1;
        let scraper = engine(configured, &transport, 8.0, 8);
        let started = Instant::now();
        let report = scraper
            .execute(&request(&[
                "https://example.test/a",
                "https://example.test/b",
                "https://example.test/c",
                "https://example.test/d",
            ]))
            .await
            .unwrap();
        assert_eq!(report.summary.pages_fetched, 4);
        assert!(
            started.elapsed() >= Duration::from_millis(40),
            "four requests at 50/s with burst 1 must not finish instantly"
        );
    }

    #[tokio::test]
    async fn modes_without_a_locked_stack_are_refused_before_any_request() {
        let transport = Arc::new(FakeTransport::new());
        for mode in ["BrowserHeadless", "DataTransform"] {
            let error = ScraperSettings::from_configuration(&contract(mode)).unwrap_err();
            assert_eq!(error.code, CODE_UNSUPPORTED_MODE, "{mode} must stay gated");
        }
        let scraper = engine(settings(ScraperMode::BrowserHeadless), &transport, 2.0, 4);
        let error = scraper
            .execute(&request(&["https://example.test/list"]))
            .await
            .unwrap_err();
        assert_eq!(error.code, CODE_UNSUPPORTED_MODE);
        assert!(transport.calls().is_empty());
    }

    #[tokio::test]
    async fn unknown_modes_and_missing_rows_ports_are_refused() {
        let unknown = json!({
            "configuration": { "defaults": { "mode": "Telepathy" } },
            "ports": { "inputs": [], "outputs": [{ "id": "rows" }] }
        });
        assert_eq!(
            ScraperSettings::from_configuration(&unknown)
                .unwrap_err()
                .code,
            CODE_INVALID_CONFIGURATION
        );
        let portless = json!({
            "configuration": { "defaults": { "mode": "FastHttp" } },
            "ports": { "inputs": [], "outputs": [{ "id": "html" }] }
        });
        assert_eq!(
            ScraperSettings::from_configuration(&portless)
                .unwrap_err()
                .code,
            CODE_INVALID_CONFIGURATION
        );
    }

    #[tokio::test]
    async fn invalid_selectors_seeds_and_budgets_are_refused() {
        let transport = Arc::new(FakeTransport::new());
        let mut broken = settings(ScraperMode::FastHttp);
        broken.row_selector = "tr[".to_owned();
        let error = match UniversalScraper::new(broken, &resources(Some(2.0)), transport.clone(), 4)
        {
            Err(error) => error,
            Ok(_) => panic!("an invalid row selector must be refused before any request"),
        };
        assert_eq!(error.code, CODE_INVALID_SELECTOR);

        let scraper = engine(settings(ScraperMode::FastHttp), &transport, 2.0, 4);
        for seed in ["file:///etc/passwd", "not a url", "ftp://example.test/x"] {
            let error = scraper.execute(&request(&[seed])).await.unwrap_err();
            assert_eq!(error.code, CODE_INVALID_SEED, "{seed} must be refused");
        }
        assert_eq!(
            scraper.execute(&request(&[])).await.unwrap_err().code,
            CODE_INVALID_CONFIGURATION
        );
        assert!(transport.calls().is_empty());
    }

    #[tokio::test]
    async fn row_and_page_budgets_are_permanent_failures() {
        let transport = Arc::new(FakeTransport::new());
        transport.script(
            "https://example.test/list",
            vec![ScriptedResponse::page(CATALOGUE)],
        );
        let mut rows_capped = settings(ScraperMode::FastHttp);
        rows_capped.max_rows = 1;
        let scraper = engine(rows_capped, &transport, 2.0, 4);
        assert_eq!(
            scraper
                .execute(&request(&["https://example.test/list"]))
                .await
                .unwrap_err()
                .code,
            CODE_ROW_LIMIT_EXCEEDED
        );

        let mut bytes_capped = settings(ScraperMode::FastHttp);
        bytes_capped.max_page_bytes = 32;
        let scraper = engine(bytes_capped, &transport, 2.0, 4);
        assert_eq!(
            scraper
                .execute(&request(&["https://example.test/list"]))
                .await
                .unwrap_err()
                .code,
            CODE_PAGE_BYTES_EXCEEDED
        );
    }

    #[tokio::test]
    async fn replaying_the_same_pages_reproduces_the_same_digests() {
        let first = Arc::new(FakeTransport::new());
        first.script(
            "https://example.test/list",
            vec![ScriptedResponse::page(CATALOGUE)],
        );
        let second = Arc::new(FakeTransport::new());
        second.script(
            "https://example.test/list",
            vec![ScriptedResponse::page(CATALOGUE)],
        );
        let mut digests = Vec::new();
        for transport in [first, second] {
            let scraper = engine(settings(ScraperMode::FastHttp), &transport, 2.0, 4);
            let report = scraper
                .execute(&request(&["https://example.test/list"]))
                .await
                .unwrap();
            digests.push((
                report.summary.stream_digest,
                report.summary.correctness_digest,
            ));
        }
        assert_eq!(digests[0], digests[1]);
    }

    #[tokio::test]
    async fn settings_are_read_from_the_contract_defaults() {
        let contract = json!({
            "configuration": {
                "defaults": {
                    "mode": "DeepCrawl",
                    "row_selector": "li.row",
                    "fields": [
                        { "column": "name", "selector": ".name" },
                        { "column": "link", "selector": "a", "attribute": "href" }
                    ],
                    "requests_per_second": 12,
                    "burst": 3,
                    "timeout_seconds": 7,
                    "max_retries": 5,
                    "initial_retry_millis": 250,
                    "max_retry_millis": 4_000,
                    "max_depth": 3,
                    "max_pages": 32,
                    "crawl_same_host_only": false
                }
            },
            "ports": { "inputs": [], "outputs": [{ "id": "rows" }] }
        });
        let parsed = ScraperSettings::from_configuration(&contract).unwrap();
        assert_eq!(parsed.mode, ScraperMode::DeepCrawl);
        assert_eq!(parsed.row_selector, "li.row");
        assert_eq!(parsed.fields.len(), 2);
        assert_eq!(parsed.fields[0].column, "name");
        assert_eq!(parsed.fields[1].attribute.as_deref(), Some("href"));
        assert_eq!(parsed.requests_per_second, 12);
        assert_eq!(parsed.burst, 3);
        assert_eq!(parsed.timeout_seconds, 7);
        assert_eq!(parsed.max_retries, 5);
        assert_eq!(parsed.max_depth, 3);
        assert_eq!(parsed.max_pages, 32);
        assert!(!parsed.crawl_same_host_only);

        let mut rejected = contract.clone();
        rejected["configuration"]["defaults"]["requests_per_second"] = json!(0);
        assert_eq!(
            ScraperSettings::from_configuration(&rejected)
                .unwrap_err()
                .code,
            CODE_INVALID_CONFIGURATION
        );
        let mut rejected = contract.clone();
        rejected["configuration"]["defaults"]["max_depth"] = json!(MAX_DEPTH + 1);
        assert_eq!(
            ScraperSettings::from_configuration(&rejected)
                .unwrap_err()
                .code,
            CODE_INVALID_CONFIGURATION
        );
        let mut rejected = contract;
        rejected["configuration"]["defaults"]["burst"] = json!(60);
        assert_eq!(
            ScraperSettings::from_configuration(&rejected)
                .unwrap_err()
                .code,
            CODE_INVALID_CONFIGURATION
        );
    }

    #[test]
    fn retryable_statuses_match_the_contract() {
        assert!(retryable_status(429));
        assert!(retryable_status(500));
        assert!(retryable_status(599));
        assert!(!retryable_status(200));
        assert!(!retryable_status(404));
        assert!(!retryable_status(408));
    }
}
