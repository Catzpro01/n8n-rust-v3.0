// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    artifact::{ArtifactError, ArtifactReference, ArtifactService},
    canonical::{bytes as canonical_bytes, digest, CANONICALIZATION, DIGEST_ALGORITHM},
    compiler::ExecutionPlan,
    config::ServeConfig,
    edit_fields::{self, CompiledConfiguration},
    generate_engine::{
        self, GenerateFailure, GenerateResume, GenerateSession, GenerateStart, GenerateSummary,
        GeneratedEnvelope,
    },
    if_node, merge,
    run_engine::{self, ActivationOutcome, ManualActivationInput, ManualActivationResult},
    summarize,
};
use rand_core::{OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc as async_mpsc, OwnedSemaphorePermit, Semaphore};
use tracing::{error, info, warn};

pub const RUN_SCHEMA: &str = "canopy.run/v1alpha1";
pub const TRACE_SCHEMA: &str = "canopy.causal-trace/v1alpha1";
pub const QUEUE_PROFILE: &str = "eco-balanced/v1alpha1";
pub const MAX_NONTERMINAL_RUNS: usize = 64;
pub const MAX_HOT_RUNS: usize = 16;
pub const READY_QUEUE_COUNT: usize = 1_024;
pub const READY_QUEUE_BYTES: usize = 4 * 1024 * 1024;
pub const RESULT_QUEUE_COUNT: usize = 256;
pub const RESULT_QUEUE_BYTES: usize = 16 * 1024 * 1024;
pub const WRITER_QUEUE_COUNT: usize = 32;
pub const WRITER_QUEUE_BYTES: usize = 8 * 1024 * 1024;
pub const LIVE_RING_COUNT: usize = 256;
pub const LIVE_RING_BYTES: usize = 1024 * 1024;
pub const MAX_SSE_SUBSCRIBERS: usize = 32;
pub const SUBSCRIBER_QUEUE_COUNT: usize = 64;
pub const SUBSCRIBER_QUEUE_BYTES: usize = 256 * 1024;
pub const MAX_INVOCATION_BYTES: usize = 8 * 1024;
pub const ENVELOPE_QUEUE_COUNT: usize = 256;
pub const ENVELOPE_QUEUE_BYTES: usize = 4 * 1024 * 1024;
pub const ENVELOPE_MICRO_BATCH_COUNT: usize = 64;
const ENVELOPE_BATCH_SLOTS: usize = ENVELOPE_QUEUE_COUNT / ENVELOPE_MICRO_BATCH_COUNT;
pub const CHECKPOINT_MAX_OUTCOMES: usize = 1_024;
pub const CHECKPOINT_MAX_BYTES: usize = 1024 * 1024;
pub const CHECKPOINT_MAX_LATENCY_MILLIS: u64 = 250;
const MAX_SSE_FRAME_BYTES: usize = SUBSCRIBER_QUEUE_BYTES / SUBSCRIBER_QUEUE_COUNT;
// Reserve space for the event name, bounded Run cursor, and SSE framing overhead.
const MAX_SSE_DATA_BYTES: usize = MAX_SSE_FRAME_BYTES - 512;
const SCHEDULER_TICK: Duration = Duration::from_millis(10);
// A short governed admission window makes Queued state observable and gives a just-issued
// cancellation a deterministic chance to reach the writer before pure work is dispatched.
const MINIMUM_QUEUE_VISIBILITY: Duration = Duration::from_millis(CHECKPOINT_MAX_LATENCY_MILLIS);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmitRunRequest {
    pub run_request_id: String,
    pub publication_event_id: String,
    pub revision_id: String,
    pub plan_digest: String,
    pub captured_invocation: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRunRequest {
    pub cancellation_request_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdmissionResult {
    pub created: bool,
    pub run: RunView,
}

#[derive(Clone, Debug, Serialize)]
pub struct CancellationResult {
    pub accepted: bool,
    pub already_terminal: bool,
    pub run: RunView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunView {
    pub schema: String,
    pub run_id: String,
    pub run_request_id: String,
    pub workflow_id: String,
    pub publication_event_id: String,
    pub revision_id: String,
    pub revision_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub durable: DurableProgress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<LiveProgress>,
    pub correctness: CorrectnessView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<GenerationProgress>,
    pub admitted_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_at: Option<i64>,
    pub queue_profile: QueueProfileView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DurableProgress {
    pub state: String,
    pub checkpoint_sequence: u64,
    pub logical_order: u64,
    pub terminal: bool,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveProgress {
    pub state: String,
    pub speculative: bool,
    pub boot_epoch: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrectnessView {
    pub canonicalization: String,
    pub algorithm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub complete: bool,
    pub attempted: u64,
    pub succeeded: u64,
    pub cancelled: u64,
    pub failed: u64,
    pub output_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationProgress {
    pub state: String,
    pub generated_count: u64,
    pub logical_bytes: u64,
    pub stream_digest: String,
    pub backpressure_events: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<ArtifactReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<TransformProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<MergeProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<SummaryProgress>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformProgress {
    pub node_instance_id: String,
    pub transformed_count: u64,
    pub logical_bytes: u64,
    pub stream_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchProgress {
    pub node_instance_id: String,
    pub true_count: u64,
    pub false_count: u64,
    pub stream_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeProgress {
    pub node_instance_id: String,
    pub mode: String,
    pub true_count: u64,
    pub false_count: u64,
    pub output_count: u64,
    pub logical_bytes: u64,
    pub stream_digest: String,
    pub physical_spool_bytes: u64,
    pub true_segments: Vec<ArtifactReference>,
    pub false_segments: Vec<ArtifactReference>,
    pub output_segments: Vec<ArtifactReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummaryProgress {
    pub node_instance_id: String,
    pub operation: String,
    pub total_count: u64,
    pub true_count: u64,
    pub false_count: u64,
    pub logical_bytes: u64,
    pub output_digest: String,
    pub first_ordinal: Option<u64>,
    pub last_ordinal: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueProfileView {
    pub profile: String,
    pub maximum_nonterminal_runs: usize,
    pub maximum_hot_runs: usize,
    pub ready: QueueLimitView,
    pub envelopes: QueueLimitView,
    pub results: QueueLimitView,
    pub writer: QueueLimitView,
    pub live_ring_per_run: QueueLimitView,
    pub subscribers: SubscriberLimitView,
    pub maximum_inline_invocation_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueLimitView {
    pub count: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriberLimitView {
    pub count: usize,
    pub mailbox: QueueLimitView,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceView {
    pub schema: String,
    pub run: TraceRunIdentity,
    pub terminal_state: String,
    pub correctness: CorrectnessView,
    pub checkpoints: Vec<CheckpointView>,
    pub activations: Vec<ActivationView>,
    pub events: Vec<TraceEventView>,
    pub safe_resource_facts: Value,
    pub integrity_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceRunIdentity {
    pub run_id: String,
    pub workflow_id: String,
    pub publication_event_id: String,
    pub revision_id: String,
    pub revision_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckpointView {
    pub sequence: u64,
    pub state: String,
    pub logical_order: u64,
    pub snapshot: Value,
    pub trace_head_hash: String,
    pub checkpoint_hash: String,
    pub committed_at: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActivationView {
    pub activation_id: String,
    pub node_instance_id: String,
    pub logical_order: u64,
    pub attempt: u32,
    pub outcome: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    pub input_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
    pub provenance: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Value>,
    pub checkpoint_sequence: u64,
    pub timing: TimingView,
}

#[derive(Clone, Debug, Serialize)]
pub struct TimingView {
    pub started_at: i64,
    pub completed_at: i64,
    pub elapsed_micros: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceEventView {
    pub event_sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_order: Option<u64>,
    pub checkpoint_sequence: u64,
    pub phase: String,
    pub event_type: String,
    pub payload: Value,
    pub previous_hash: String,
    pub event_hash: String,
    pub occurred_at: i64,
}

#[derive(Debug)]
pub enum RunError {
    NotFound,
    NoCurrentPublication,
    StalePublication,
    RequestIdentityConflict,
    AdmissionFull,
    SubscriberFull,
    Invalid(&'static str),
    TooLarge(&'static str),
    Integrity(String),
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct StreamFrame {
    pub event: String,
    pub id: String,
    pub data: String,
}

pub struct RunSubscription {
    pub receiver: async_mpsc::Receiver<StreamFrame>,
    pub permit: OwnedSemaphorePermit,
}

pub struct RunService {
    database: PathBuf,
    writer: WriterClient,
    wake: SyncSender<SchedulerSignal>,
    controls: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    live: Arc<LiveHub>,
    subscriber_slots: Arc<Semaphore>,
    scheduler_thread: Mutex<Option<JoinHandle<()>>>,
    executor_thread: Mutex<Option<JoinHandle<()>>>,
    writer_thread: Mutex<Option<JoinHandle<()>>>,
}

impl RunService {
    pub fn initialize(
        config: &ServeConfig,
        artifacts: Arc<ArtifactService>,
    ) -> Result<Self, String> {
        let database = config.state_dir.join("workflow.sqlite3");
        let (writer, writer_thread) = start_writer(database.clone())?;
        let live = Arc::new(LiveHub::new());
        let controls = Arc::new(Mutex::new(HashMap::new()));
        let ready_budget = Arc::new(ByteBudget::new(READY_QUEUE_BYTES));
        let result_budget = Arc::new(ByteBudget::new(RESULT_QUEUE_BYTES));
        let (ready_sender, ready_receiver) = mpsc::sync_channel(READY_QUEUE_COUNT);
        let (result_sender, result_receiver) = mpsc::sync_channel(RESULT_QUEUE_COUNT);
        let (envelope_sender, envelope_receiver) = mpsc::sync_channel(ENVELOPE_BATCH_SLOTS);
        let (wake_sender, wake_receiver) = mpsc::sync_channel(1);

        let executor_thread = start_executor(
            ready_receiver,
            result_sender,
            envelope_sender,
            artifacts.clone(),
        )?;
        let scheduler_thread = start_scheduler(SchedulerContext {
            database: database.clone(),
            writer: writer.clone(),
            ready: ready_sender,
            ready_budget,
            result_budget,
            results: result_receiver,
            envelopes: envelope_receiver,
            wake: wake_receiver,
            artifacts: artifacts.clone(),
            controls: controls.clone(),
            live: live.clone(),
        })?;
        let service = Self {
            database,
            writer,
            wake: wake_sender,
            controls,
            live,
            subscriber_slots: Arc::new(Semaphore::new(MAX_SSE_SUBSCRIBERS)),
            scheduler_thread: Mutex::new(Some(scheduler_thread)),
            executor_thread: Mutex::new(Some(executor_thread)),
            writer_thread: Mutex::new(Some(writer_thread)),
        };
        service.notify_scheduler();
        Ok(service)
    }

    pub fn existing_admission(
        &self,
        workflow_id: &str,
        request: &AdmitRunRequest,
    ) -> Result<Option<AdmissionResult>, RunError> {
        validate_identifier(workflow_id, "workflow_id")?;
        validate_identifier(&request.run_request_id, "run_request_id")?;
        let request_digest = admission_request_digest(workflow_id, request)?;
        let connection = connect(&self.database).map_err(RunError::Storage)?;
        let existing = connection
            .query_row(
                "SELECT run_id,request_digest FROM runs WHERE run_request_id=?1",
                params![request.run_request_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let Some((run_id, stored_digest)) = existing else {
            return Ok(None);
        };
        if stored_digest != request_digest {
            return Err(RunError::RequestIdentityConflict);
        }
        let mut run = load_run(&connection, &run_id)?;
        run.live = self.live.snapshot(&run_id);
        Ok(Some(AdmissionResult {
            created: false,
            run,
        }))
    }

    pub fn admit(
        &self,
        workflow_id: &str,
        request: AdmitRunRequest,
    ) -> Result<AdmissionResult, RunError> {
        validate_identifier(workflow_id, "workflow_id")?;
        validate_identifier(&request.run_request_id, "run_request_id")?;
        validate_identifier(&request.publication_event_id, "publication_event_id")?;
        validate_identifier(&request.revision_id, "revision_id")?;
        validate_tagged_digest(&request.plan_digest, "plan_digest")?;
        reject_sensitive_keys(&request.captured_invocation)?;
        let invocation_bytes = canonical_bytes(&request.captured_invocation)
            .map_err(|_| RunError::Invalid("captured_invocation"))?;
        if invocation_bytes.len() > MAX_INVOCATION_BYTES {
            return Err(RunError::TooLarge("captured_invocation"));
        }
        let weight = invocation_bytes.len().saturating_add(4 * 1024);
        let result = self.writer.call(
            WriterOperation::Admit {
                workflow_id: workflow_id.into(),
                request,
            },
            weight,
        )?;
        let WriterReply::Admission(mut result) = result else {
            return Err(RunError::Storage("writer returned the wrong reply".into()));
        };
        result.run.live = self.live.snapshot(&result.run.run_id);
        if result.created {
            self.notify_scheduler();
        }
        Ok(result)
    }

    pub fn status(&self, run_id: &str) -> Result<RunView, RunError> {
        validate_identifier(run_id, "run_id")?;
        let connection = connect(&self.database).map_err(RunError::Storage)?;
        let mut run = load_run(&connection, run_id)?;
        run.live = self.live.snapshot(run_id);
        Ok(run)
    }

    pub fn cancel(
        &self,
        run_id: &str,
        request: CancelRunRequest,
    ) -> Result<CancellationResult, RunError> {
        validate_identifier(run_id, "run_id")?;
        validate_identifier(&request.cancellation_request_id, "cancellation_request_id")?;
        let active = self
            .controls
            .lock()
            .map_err(|_| RunError::Storage("run controls are poisoned".into()))?
            .contains_key(run_id);
        let reply = self.writer.call(
            WriterOperation::Cancel {
                run_id: run_id.into(),
                request,
                active,
            },
            4 * 1024,
        )?;
        let WriterReply::Cancellation(mut result) = reply else {
            return Err(RunError::Storage("writer returned the wrong reply".into()));
        };
        if result.accepted {
            if let Some(control) = self
                .controls
                .lock()
                .map_err(|_| RunError::Storage("run controls are poisoned".into()))?
                .get(run_id)
                .cloned()
            {
                control.store(true, Ordering::Release);
            }
            self.notify_scheduler();
            let terminal = result.run.durable.terminal;
            self.live.emit(
                run_id,
                result.run.durable.checkpoint_sequence,
                if terminal { "terminal" } else { "durable" },
                stream_payload(&result.run, "durable", &result.run.durable.state, terminal),
                terminal,
            );
        }
        result.run.live = self.live.snapshot(run_id);
        Ok(result)
    }

    pub fn trace(&self, run_id: &str) -> Result<TraceView, RunError> {
        validate_identifier(run_id, "run_id")?;
        let connection = connect(&self.database).map_err(RunError::Storage)?;
        load_trace(&connection, run_id)
    }

    pub fn subscribe(
        &self,
        run_id: &str,
        last_event_id: Option<&str>,
    ) -> Result<RunSubscription, RunError> {
        let snapshot = self.status(run_id)?;
        let permit = self
            .subscriber_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| RunError::SubscriberFull)?;
        let receiver = self.live.subscribe(run_id, last_event_id, &snapshot)?;
        Ok(RunSubscription { receiver, permit })
    }

    fn notify_scheduler(&self) {
        let _ = self.wake.try_send(SchedulerSignal::Wake);
    }
}

impl Drop for RunService {
    fn drop(&mut self) {
        let _ = self.wake.try_send(SchedulerSignal::Stop);
        if let Ok(thread) = self.scheduler_thread.get_mut() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
        if let Ok(thread) = self.executor_thread.get_mut() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
        let _ = self.writer.call(WriterOperation::Stop, 0);
        if let Ok(thread) = self.writer_thread.get_mut() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[derive(Clone)]
struct WriterClient {
    sender: SyncSender<WeightedWriterCommand>,
    bytes: Arc<ByteBudget>,
}

struct WeightedWriterCommand {
    operation: WriterOperation,
    response: SyncSender<Result<WriterReply, RunError>>,
    _bytes: BytePermit,
}

enum WriterOperation {
    Admit {
        workflow_id: String,
        request: AdmitRunRequest,
    },
    Cancel {
        run_id: String,
        request: CancelRunRequest,
        active: bool,
    },
    GenerationProgress(GeneratedProgressCommit),
    PreparationFailed {
        run_id: String,
        reason: String,
        suspended: bool,
    },
    Complete(CompletedWork),
    CompleteGenerated(CompletedGeneratedWork),
    Stop,
}

enum WriterReply {
    Admission(AdmissionResult),
    Cancellation(CancellationResult),
    Run(RunView),
    Stopped,
}

impl WriterClient {
    fn call(&self, operation: WriterOperation, weight: usize) -> Result<WriterReply, RunError> {
        let permit = self
            .bytes
            .reserve(weight)
            .ok_or(RunError::TooLarge("database_command"))?;
        let (sender, receiver) = mpsc::sync_channel(1);
        self.sender
            .send(WeightedWriterCommand {
                operation,
                response: sender,
                _bytes: permit,
            })
            .map_err(|_| RunError::Storage("database writer stopped".into()))?;
        receiver
            .recv()
            .map_err(|_| RunError::Storage("database writer dropped its response".into()))?
    }
}

fn start_writer(database: PathBuf) -> Result<(WriterClient, JoinHandle<()>), String> {
    let (sender, receiver) = mpsc::sync_channel::<WeightedWriterCommand>(WRITER_QUEUE_COUNT);
    let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
    let thread = thread::Builder::new()
        .name("workflowd-run-writer".into())
        .spawn(move || {
            let opened = connect(&database).and_then(|mut connection| {
                initialize_schema(&connection)?;
                recover_cancellations(&mut connection)?;
                // ArtifactService initialization and startup reconciliation completed before
                // RunService starts. That successful boot is the revalidation barrier for
                // work that was durably suspended by transient storage pressure.
                connection
                    .execute("DELETE FROM run_suspensions", [])
                    .map_err(|error| error.to_string())?;
                Ok(connection)
            });
            match opened {
                Ok(mut connection) => {
                    if startup_sender.send(Ok(())).is_err() {
                        return;
                    }
                    while let Ok(command) = receiver.recv() {
                        let stop = matches!(command.operation, WriterOperation::Stop);
                        let result = handle_writer_operation(&mut connection, command.operation);
                        let _ = command.response.send(result);
                        if stop {
                            break;
                        }
                    }
                }
                Err(reason) => {
                    let _ = startup_sender.send(Err(reason));
                }
            }
        })
        .map_err(|error| format!("cannot start Run SQLite writer: {error}"))?;
    match startup_receiver.recv() {
        Ok(Ok(())) => Ok((
            WriterClient {
                sender,
                bytes: Arc::new(ByteBudget::new(WRITER_QUEUE_BYTES)),
            },
            thread,
        )),
        Ok(Err(error)) => {
            let _ = thread.join();
            Err(error)
        }
        Err(error) => {
            let _ = thread.join();
            Err(format!("Run SQLite writer ended during startup: {error}"))
        }
    }
}

fn handle_writer_operation(
    connection: &mut Connection,
    operation: WriterOperation,
) -> Result<WriterReply, RunError> {
    match operation {
        WriterOperation::Admit {
            workflow_id,
            request,
        } => admit_transaction(connection, &workflow_id, request).map(WriterReply::Admission),
        WriterOperation::Cancel {
            run_id,
            request,
            active,
        } => {
            cancel_transaction(connection, &run_id, request, active).map(WriterReply::Cancellation)
        }
        WriterOperation::GenerationProgress(progress) => {
            generation_progress_transaction(connection, progress).map(WriterReply::Run)
        }
        WriterOperation::PreparationFailed {
            run_id,
            reason,
            suspended,
        } => preparation_failed_transaction(connection, &run_id, &reason, suspended)
            .map(WriterReply::Run),
        WriterOperation::Complete(result) => {
            complete_transaction(connection, result).map(WriterReply::Run)
        }
        WriterOperation::CompleteGenerated(result) => {
            complete_generated_transaction(connection, result).map(WriterReply::Run)
        }
        WriterOperation::Stop => Ok(WriterReply::Stopped),
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS runs(
                run_id TEXT PRIMARY KEY,
                run_request_id TEXT NOT NULL UNIQUE,
                request_digest TEXT NOT NULL,
                workflow_id TEXT NOT NULL REFERENCES workflow_drafts(workflow_id),
                publication_event_id TEXT NOT NULL REFERENCES publication_events(event_id),
                revision_id TEXT NOT NULL REFERENCES workflow_revisions(revision_id),
                revision_digest TEXT NOT NULL,
                plan_id TEXT NOT NULL REFERENCES execution_plans(plan_id),
                plan_digest TEXT NOT NULL,
                captured_invocation_json TEXT NOT NULL,
                state TEXT NOT NULL CHECK(state IN ('queued','cancel_requested','succeeded','failed','cancelled')),
                checkpoint_sequence INTEGER NOT NULL,
                logical_order INTEGER NOT NULL,
                attempted INTEGER NOT NULL,
                succeeded INTEGER NOT NULL,
                cancelled INTEGER NOT NULL,
                failed INTEGER NOT NULL,
                output_count INTEGER NOT NULL,
                correctness_digest TEXT,
                digest_complete INTEGER NOT NULL CHECK(digest_complete IN (0,1)),
                cancellation_request_id TEXT,
                cancellation_request_digest TEXT,
                trace_head_hash TEXT NOT NULL,
                admitted_at INTEGER NOT NULL,
                started_at INTEGER,
                updated_at INTEGER NOT NULL,
                terminal_at INTEGER
            ) STRICT;
            CREATE INDEX IF NOT EXISTS runs_scheduler
                ON runs(state, admitted_at, run_id);
            CREATE TABLE IF NOT EXISTS run_activations(
                activation_id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES runs(run_id),
                node_instance_id TEXT NOT NULL,
                logical_order INTEGER NOT NULL,
                attempt INTEGER NOT NULL,
                outcome TEXT NOT NULL CHECK(outcome IN ('success','cancelled','permanent_failure')),
                input_json TEXT NOT NULL,
                output_json TEXT,
                input_digest TEXT NOT NULL,
                output_digest TEXT,
                provenance_json TEXT NOT NULL,
                failure_json TEXT,
                checkpoint_sequence INTEGER NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER NOT NULL,
                elapsed_micros INTEGER NOT NULL,
                UNIQUE(run_id,logical_order,attempt)
            ) STRICT;
            CREATE TABLE IF NOT EXISTS run_trace_events(
                run_id TEXT NOT NULL REFERENCES runs(run_id),
                event_sequence INTEGER NOT NULL,
                logical_order INTEGER,
                checkpoint_sequence INTEGER NOT NULL,
                phase TEXT NOT NULL CHECK(phase IN ('logical','physical','control')),
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                previous_hash TEXT NOT NULL,
                event_hash TEXT NOT NULL,
                occurred_at INTEGER NOT NULL,
                PRIMARY KEY(run_id,event_sequence)
            ) STRICT;
            CREATE TABLE IF NOT EXISTS run_checkpoints(
                run_id TEXT NOT NULL REFERENCES runs(run_id),
                checkpoint_sequence INTEGER NOT NULL,
                state TEXT NOT NULL,
                logical_order INTEGER NOT NULL,
                snapshot_json TEXT NOT NULL,
                trace_head_hash TEXT NOT NULL,
                checkpoint_hash TEXT NOT NULL,
                committed_at INTEGER NOT NULL,
                PRIMARY KEY(run_id,checkpoint_sequence)
            ) STRICT;
            CREATE TABLE IF NOT EXISTS run_generation_progress(
                run_id TEXT PRIMARY KEY REFERENCES runs(run_id),
                state TEXT NOT NULL CHECK(state IN ('running','succeeded','failed','cancelled','suspended')),
                generated_count INTEGER NOT NULL,
                logical_bytes INTEGER NOT NULL,
                stream_digest TEXT NOT NULL,
                backpressure_events INTEGER NOT NULL,
                backpressure_micros INTEGER NOT NULL,
                artifact_json TEXT,
                transform_node_id TEXT,
                transformed_count INTEGER NOT NULL DEFAULT 0,
                transformed_logical_bytes INTEGER NOT NULL DEFAULT 0,
                transformed_stream_digest TEXT NOT NULL DEFAULT 'genesis',
                branch_node_id TEXT,
                branch_true_count INTEGER NOT NULL DEFAULT 0,
                branch_false_count INTEGER NOT NULL DEFAULT 0,
                branch_stream_digest TEXT NOT NULL DEFAULT 'genesis',
                merge_json TEXT,
                summary_json TEXT,
                updated_at INTEGER NOT NULL
            ) STRICT;
            CREATE TABLE IF NOT EXISTS run_suspensions(
                run_id TEXT PRIMARY KEY REFERENCES runs(run_id),
                code TEXT NOT NULL,
                safe_message TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            ) STRICT;
            CREATE TRIGGER IF NOT EXISTS immutable_run_activations_update
                BEFORE UPDATE ON run_activations BEGIN
                SELECT RAISE(ABORT,'Run Activations are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_run_activations_delete
                BEFORE DELETE ON run_activations BEGIN
                SELECT RAISE(ABORT,'Run Activations are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_run_trace_events_update
                BEFORE UPDATE ON run_trace_events BEGIN
                SELECT RAISE(ABORT,'Causal Trace events are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_run_trace_events_delete
                BEFORE DELETE ON run_trace_events BEGIN
                SELECT RAISE(ABORT,'Causal Trace events are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_run_checkpoints_update
                BEFORE UPDATE ON run_checkpoints BEGIN
                SELECT RAISE(ABORT,'Durable Checkpoints are immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_run_checkpoints_delete
                BEFORE DELETE ON run_checkpoints BEGIN
                SELECT RAISE(ABORT,'Durable Checkpoints are immutable'); END;
            "#,
        )
        .map_err(|error| error.to_string())?;
    ensure_generation_column(connection, "transform_node_id", "TEXT")?;
    ensure_generation_column(
        connection,
        "transformed_count",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_generation_column(
        connection,
        "transformed_logical_bytes",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_generation_column(
        connection,
        "transformed_stream_digest",
        "TEXT NOT NULL DEFAULT 'genesis'",
    )?;
    ensure_generation_column(connection, "branch_node_id", "TEXT")?;
    ensure_generation_column(
        connection,
        "branch_true_count",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_generation_column(
        connection,
        "branch_false_count",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_generation_column(
        connection,
        "branch_stream_digest",
        "TEXT NOT NULL DEFAULT 'genesis'",
    )?;
    ensure_generation_column(connection, "merge_json", "TEXT")?;
    ensure_generation_column(connection, "summary_json", "TEXT")?;
    Ok(())
}

fn ensure_generation_column(
    connection: &Connection,
    name: &str,
    definition: &str,
) -> Result<(), String> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('run_generation_progress') WHERE name=?1)",
            params![name],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?
        == 1;
    if !exists {
        connection
            .execute_batch(&format!(
                "ALTER TABLE run_generation_progress ADD COLUMN {name} {definition};"
            ))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn admit_transaction(
    connection: &mut Connection,
    workflow_id: &str,
    request: AdmitRunRequest,
) -> Result<AdmissionResult, RunError> {
    let request_digest = admission_request_digest(workflow_id, &request)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    if let Some((run_id, stored_digest)) = transaction
        .query_row(
            "SELECT run_id,request_digest FROM runs WHERE run_request_id=?1",
            params![request.run_request_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?
    {
        if stored_digest != request_digest {
            return Err(RunError::RequestIdentityConflict);
        }
        let run = load_run(&transaction, &run_id)?;
        transaction.commit().map_err(storage_error)?;
        return Ok(AdmissionResult {
            created: false,
            run,
        });
    }
    let nonterminal: i64 = transaction
        .query_row(
            "SELECT count(*) FROM runs WHERE state IN ('queued','cancel_requested')",
            [],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if nonterminal >= MAX_NONTERMINAL_RUNS as i64 {
        return Err(RunError::AdmissionFull);
    }
    let current: Option<(String, String, String, String, String, String, String)> = transaction
        .query_row(
            "SELECT h.event_id,h.revision_id,r.revision_digest,p.plan_id,p.plan_digest,p.payload_json,e.envelope_json
             FROM workflow_publication_heads h
             JOIN workflow_revisions r ON r.revision_id=h.revision_id
             JOIN execution_plans p ON p.revision_id=r.revision_id
             JOIN publication_events e ON e.event_id=h.event_id
             WHERE h.workflow_id=?1",
            params![workflow_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        event_id,
        revision_id,
        revision_digest,
        plan_id,
        plan_digest,
        plan_json,
        envelope_json,
    )) = current
    else {
        return Err(RunError::NoCurrentPublication);
    };
    if event_id != request.publication_event_id
        || revision_id != request.revision_id
        || plan_digest != request.plan_digest
    {
        return Err(RunError::StalePublication);
    }
    let plan: ExecutionPlan = serde_json::from_str(&plan_json)
        .map_err(|error| RunError::Integrity(format!("stored plan is invalid: {error}")))?;
    if digest(&plan).map_err(RunError::Integrity)? != plan_digest
        || plan.revision_digest != revision_digest
    {
        return Err(RunError::Integrity(
            "pinned plan digest or revision linkage failed".into(),
        ));
    }
    let envelope: Value = serde_json::from_str(&envelope_json).map_err(|error| {
        RunError::Integrity(format!("publication envelope is invalid: {error}"))
    })?;
    if envelope["event_id"] != event_id
        || envelope["target_revision_id"] != revision_id
        || envelope["revision_digest"] != revision_digest
        || envelope["plan_digest"] != plan_digest
    {
        return Err(RunError::Integrity(
            "current publication event linkage failed".into(),
        ));
    }

    let run_id = random_id("run");
    let admitted_at = now_millis();
    let invocation_json = canonical_text(&request.captured_invocation)?;
    let admission_payload = json!({
        "run_request_id": request.run_request_id,
        "workflow_id": workflow_id,
        "publication_event_id": event_id,
        "revision_id": revision_id,
        "revision_digest": revision_digest,
        "plan_id": plan_id,
        "plan_digest": plan_digest,
        "state": "queued",
        "resume": {"next_logical_order": 1},
        "queue_profile": QUEUE_PROFILE
    });
    let trace_event = make_trace_event(
        &run_id,
        1,
        None,
        1,
        "control",
        "run_admitted",
        admission_payload,
        "genesis",
        admitted_at,
    )?;
    transaction
        .execute(
            "INSERT INTO runs(run_id,run_request_id,request_digest,workflow_id,publication_event_id,revision_id,revision_digest,plan_id,plan_digest,captured_invocation_json,state,checkpoint_sequence,logical_order,attempted,succeeded,cancelled,failed,output_count,correctness_digest,digest_complete,cancellation_request_id,cancellation_request_digest,trace_head_hash,admitted_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'queued',1,0,0,0,0,0,0,NULL,0,NULL,NULL,?11,?12,?12)",
            params![
                run_id,
                request.run_request_id,
                request_digest,
                workflow_id,
                event_id,
                revision_id,
                revision_digest,
                plan_id,
                plan_digest,
                invocation_json,
                trace_event.event_hash,
                admitted_at
            ],
        )
        .map_err(storage_error)?;
    insert_trace_event(&transaction, &trace_event)?;
    insert_checkpoint(
        &transaction,
        &run_id,
        1,
        "queued",
        0,
        &json!({
            "resume": {"next_logical_order": 1},
            "counters": counters_json(0, 0, 0, 0, 0),
            "correctness_digest": Value::Null,
            "complete": false,
            "provenance": {"revision_id": revision_id, "plan_id": plan_id}
        }),
        &trace_event.event_hash,
        admitted_at,
    )?;
    let run = load_run(&transaction, &run_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(AdmissionResult { created: true, run })
}

fn cancel_transaction(
    connection: &mut Connection,
    run_id: &str,
    request: CancelRunRequest,
    active: bool,
) -> Result<CancellationResult, RunError> {
    let request_digest = digest(&request).map_err(RunError::Integrity)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current = load_run(&transaction, run_id)?;
    let stored_cancel: (Option<String>, Option<String>, String) = transaction
        .query_row(
            "SELECT cancellation_request_id,cancellation_request_digest,trace_head_hash FROM runs WHERE run_id=?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(storage_error)?;
    if stored_cancel.0.as_deref() == Some(&request.cancellation_request_id)
        && stored_cancel.1.as_deref() != Some(&request_digest)
    {
        return Err(RunError::RequestIdentityConflict);
    }
    if current.durable.terminal {
        transaction.commit().map_err(storage_error)?;
        return Ok(CancellationResult {
            accepted: stored_cancel.0.as_deref() == Some(&request.cancellation_request_id),
            already_terminal: true,
            run: current,
        });
    }
    if current.durable.state == "cancel_requested" {
        transaction.commit().map_err(storage_error)?;
        return Ok(CancellationResult {
            accepted: true,
            already_terminal: false,
            run: current,
        });
    }

    let checkpoint = current.durable.checkpoint_sequence + 1;
    let occurred_at = now_millis();
    let terminal = !active;
    let state = if terminal {
        "cancelled"
    } else {
        "cancel_requested"
    };
    let cancellation_payload = json!({
        "cancellation_request_id": request.cancellation_request_id,
        "state": state,
        "stopped_new_work": true,
        "active_activation": active,
        "terminal": terminal
    });
    let sequence = next_trace_sequence(&transaction, run_id)?;
    let trace_event = make_trace_event(
        run_id,
        sequence,
        None,
        checkpoint,
        "control",
        if terminal {
            "run_cancelled_before_activation"
        } else {
            "cancellation_requested"
        },
        cancellation_payload,
        &stored_cancel.2,
        occurred_at,
    )?;
    insert_trace_event(&transaction, &trace_event)?;
    let cancelled_digest =
        run_engine::cancelled_correctness_digest(&current.revision_digest, &current.plan_digest);
    let snapshot = json!({
        "resume": {"next_logical_order": 1},
        "counters": counters_json(0, 0, 0, 0, 0),
        "correctness_digest": if terminal { Value::String(cancelled_digest.clone()) } else { Value::Null },
        "complete": false,
        "cancellation_request_id": request.cancellation_request_id
    });
    insert_checkpoint(
        &transaction,
        run_id,
        checkpoint,
        state,
        0,
        &snapshot,
        &trace_event.event_hash,
        occurred_at,
    )?;
    transaction
        .execute(
            "UPDATE runs SET state=?2,checkpoint_sequence=?3,correctness_digest=?4,digest_complete=0,cancellation_request_id=?5,cancellation_request_digest=?6,trace_head_hash=?7,updated_at=?8,terminal_at=?9 WHERE run_id=?1",
            params![
                run_id,
                state,
                checkpoint as i64,
                if terminal { Some(cancelled_digest) } else { None },
                request.cancellation_request_id,
                request_digest,
                trace_event.event_hash,
                occurred_at,
                if terminal { Some(occurred_at) } else { None }
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM run_suspensions WHERE run_id=?1",
            params![run_id],
        )
        .map_err(storage_error)?;
    let run = load_run(&transaction, run_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(CancellationResult {
        accepted: true,
        already_terminal: false,
        run,
    })
}

fn preparation_failed_transaction(
    connection: &mut Connection,
    run_id: &str,
    reason: &str,
    suspended: bool,
) -> Result<RunView, RunError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current = load_run(&transaction, run_id)?;
    if current.durable.terminal || (suspended && current.durable.state == "suspended") {
        transaction.commit().map_err(storage_error)?;
        return Ok(current);
    }
    if current.durable.state == "cancel_requested" {
        transaction.commit().map_err(storage_error)?;
        finalize_requested_cancellation(
            connection,
            run_id,
            "cancellation_settled_after_preparation",
            "artifact_preparation_ended_after_durable_cancellation",
        )?;
        return load_run(connection, run_id);
    }
    let checkpoint = current.durable.checkpoint_sequence + 1;
    let committed_at = now_millis();
    let state = if suspended { "suspended" } else { "failed" };
    let code = if suspended {
        "canopy.generate-items.artifact_storage_pressure"
    } else {
        "canopy.generate-items.artifact_preparation_failed"
    };
    let safe_message = if suspended {
        "Artifact storage is temporarily unavailable; the Run is durably suspended."
    } else {
        "The configured Artifact could not be safely prepared."
    };
    let event = make_trace_event(
        run_id,
        next_trace_sequence(&transaction, run_id)?,
        Some(1),
        checkpoint,
        "physical",
        if suspended {
            "run_durably_suspended"
        } else {
            "artifact_preparation_failed"
        },
        json!({"state":state,"code":code,"safe_message":safe_message}),
        &current_trace_head(&transaction, run_id)?,
        committed_at,
    )?;
    insert_trace_event(&transaction, &event)?;
    let generated_count = current
        .generation
        .as_ref()
        .map_or(0, |progress| progress.generated_count);
    let logical_bytes = current
        .generation
        .as_ref()
        .map_or(0, |progress| progress.logical_bytes);
    let stream_digest = current.generation.as_ref().map_or_else(
        || "genesis".into(),
        |progress| progress.stream_digest.clone(),
    );
    let snapshot = json!({
        "state":state,
        "resume":{"generate_next_ordinal":generated_count},
        "generated_count":generated_count,
        "logical_bytes":logical_bytes,
        "stream_digest":stream_digest,
        "failure":{"code":code,"message":safe_message},
        "complete":false
    });
    insert_checkpoint(
        &transaction,
        run_id,
        checkpoint,
        state,
        current.durable.logical_order,
        &snapshot,
        &event.event_hash,
        committed_at,
    )?;
    if suspended {
        transaction
            .execute(
                "INSERT INTO run_suspensions(run_id,code,safe_message,updated_at) VALUES(?1,?2,?3,?4)
                 ON CONFLICT(run_id) DO UPDATE SET code=excluded.code,safe_message=excluded.safe_message,updated_at=excluded.updated_at",
                params![run_id, code, safe_message, committed_at],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE runs SET checkpoint_sequence=?2,trace_head_hash=?3,updated_at=?4 WHERE run_id=?1 AND state='queued'",
                params![run_id, checkpoint as i64, event.event_hash, committed_at],
            )
            .map_err(storage_error)?;
    } else {
        let correctness_digest = digest(&json!({
            "schema":"canopy.correctness-digest/v1alpha1",
            "revision_digest":current.revision_digest,
            "plan_digest":current.plan_digest,
            "complete":false,
            "failure_code":code
        }))
        .map_err(RunError::Integrity)?;
        transaction
            .execute(
                "DELETE FROM run_suspensions WHERE run_id=?1",
                params![run_id],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE runs SET state='failed',checkpoint_sequence=?2,attempted=1,failed=1,correctness_digest=?3,digest_complete=0,trace_head_hash=?4,updated_at=?5,terminal_at=?5 WHERE run_id=?1 AND state='queued'",
                params![run_id, checkpoint as i64, correctness_digest, event.event_hash, committed_at],
            )
            .map_err(storage_error)?;
    }
    let run = load_run(&transaction, run_id)?;
    transaction.commit().map_err(storage_error)?;
    if !suspended {
        warn!(event = "run_artifact_permanent_failure", run_id, reason);
    }
    Ok(run)
}

fn generation_progress_transaction(
    connection: &mut Connection,
    progress: GeneratedProgressCommit,
) -> Result<RunView, RunError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current = load_run(&transaction, &progress.run_id)?;
    if current.durable.terminal || current.durable.state == "cancel_requested" {
        transaction.commit().map_err(storage_error)?;
        return Ok(current);
    }
    if progress.generated_count != progress.last_ordinal.saturating_add(1)
        || progress.first_ordinal > progress.last_ordinal
    {
        return Err(RunError::Integrity(
            "Generate checkpoint ordinal range is inconsistent".into(),
        ));
    }
    if progress.transform_node_id.is_some()
        && progress.transformed_count != progress.generated_count
    {
        return Err(RunError::Integrity(
            "Edit Fields checkpoint count is not one-to-one with generated items".into(),
        ));
    }
    if progress.transform_node_id.is_none()
        && (progress.transformed_count != 0 || progress.transformed_logical_bytes != 0)
    {
        return Err(RunError::Integrity(
            "Transform checkpoint exists without an Edit Fields node".into(),
        ));
    }
    if let Some(branch_node_id) = progress.branch_node_id.as_ref() {
        if progress
            .branch_true_count
            .saturating_add(progress.branch_false_count)
            != progress.generated_count
        {
            return Err(RunError::Integrity(format!(
                "If checkpoint counts do not cover generated items for {branch_node_id}"
            )));
        }
    } else if progress.branch_true_count != 0
        || progress.branch_false_count != 0
        || progress.branch_stream_digest != "genesis"
    {
        return Err(RunError::Integrity(
            "Branch checkpoint exists without an If node".into(),
        ));
    }
    let durable_count = current
        .generation
        .as_ref()
        .map_or(0, |durable| durable.generated_count);
    if progress.generated_count <= durable_count {
        transaction.commit().map_err(storage_error)?;
        return Ok(current);
    }
    if progress.first_ordinal != durable_count {
        return Err(RunError::Integrity(
            "Generate checkpoint is not contiguous with its durable cursor".into(),
        ));
    }
    if let Some(durable) = &current.generation {
        if progress.logical_bytes < durable.logical_bytes {
            return Err(RunError::Integrity(
                "Generate checkpoint logical bytes regressed".into(),
            ));
        }
        if let Some(transform) = &durable.transform {
            if progress.transformed_logical_bytes < transform.logical_bytes
                || progress.transformed_count < transform.transformed_count
            {
                return Err(RunError::Integrity(
                    "Edit Fields checkpoint progress regressed".into(),
                ));
            }
        }
        if let Some(branch) = &durable.branch {
            if progress.branch_true_count < branch.true_count
                || progress.branch_false_count < branch.false_count
            {
                return Err(RunError::Integrity(
                    "If checkpoint progress regressed".into(),
                ));
            }
        }
    }
    let checkpoint = current.durable.checkpoint_sequence + 1;
    let committed_at = now_millis();
    let mut previous_hash = current_trace_head(&transaction, &progress.run_id)?;
    let mut sequence = next_trace_sequence(&transaction, &progress.run_id)?;
    if current.durable.logical_order == 0 {
        let plan = load_execution_plan(&transaction, &progress.run_id)?;
        let manual_node = plan
            .nodes
            .iter()
            .find(|node| node.contract_lock.name == "manual-trigger")
            .ok_or_else(|| RunError::Integrity("Pinned Manual Trigger is missing".into()))?
            .node_instance_id
            .clone();
        let input_text: String = transaction
            .query_row(
                "SELECT captured_invocation_json FROM runs WHERE run_id=?1",
                params![progress.run_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let input: Value = serde_json::from_str(&input_text).map_err(|error| {
            RunError::Integrity(format!("captured invocation is invalid: {error}"))
        })?;
        let activation_id = generated_activation_id(&progress.run_id, &manual_node, 1);
        insert_activation_row(
            &transaction,
            ActivationInsert {
                activation_id: &activation_id,
                run_id: &progress.run_id,
                node_id: &manual_node,
                logical_order: 1,
                outcome: "success",
                input: &input,
                output: Some(&input),
                failure: None,
                checkpoint,
                started_at: progress.started_at,
                completed_at: progress.started_at,
                elapsed_micros: 0,
                provenance: json!({"engine_abi":generate_engine::GENERATE_ENGINE_ABI,"lane":"native-cpu","effect_class":"pure","output_port":"invocation"}),
            },
        )?;
        let manual_event = make_trace_event(
            &progress.run_id,
            sequence,
            Some(1),
            checkpoint,
            "logical",
            "activation_outcome",
            json!({
                "activation_id":activation_id,"node_instance_id":manual_node,"attempt":1,
                "outcome":"success","input_digest":digest(&input).map_err(RunError::Integrity)?,
                "output_digest":digest(&input).map_err(RunError::Integrity)?
            }),
            &previous_hash,
            committed_at,
        )?;
        insert_trace_event(&transaction, &manual_event)?;
        previous_hash = manual_event.event_hash;
        sequence += 1;
    }
    let event = make_trace_event(
        &progress.run_id,
        sequence,
        Some(2),
        checkpoint,
        "physical",
        "generation_checkpoint",
        json!({
            "generated_count":progress.generated_count,
            "logical_bytes":progress.logical_bytes,
            "stream_digest":progress.stream_digest,
            "transform": progress.transform_node_id.as_ref().map(|node_id| json!({
                "node_instance_id": node_id,
                "transformed_count": progress.transformed_count,
                "logical_bytes": progress.transformed_logical_bytes,
                "stream_digest": progress.transformed_stream_digest
            })),
            "branch": progress.branch_node_id.as_ref().map(|node_id| json!({
                "node_instance_id": node_id,
                "true_count": progress.branch_true_count,
                "false_count": progress.branch_false_count,
                "stream_digest": progress.branch_stream_digest
            })),
            "contiguous_ordinal_range":[progress.first_ordinal,progress.last_ordinal],
            "backpressure_events":progress.backpressure_events,
            "backpressure_micros":progress.backpressure_micros,
            "artifact":progress.artifact
        }),
        &previous_hash,
        committed_at,
    )?;
    insert_trace_event(&transaction, &event)?;
    let snapshot = json!({
        "resume":{"generate_next_ordinal":progress.generated_count},
        "logical_order":1,
        "generated_count":progress.generated_count,
        "logical_bytes":progress.logical_bytes,
        "stream_digest":progress.stream_digest,
        "transform": progress.transform_node_id.as_ref().map(|node_id| json!({
            "node_instance_id": node_id,
            "transformed_count": progress.transformed_count,
            "logical_bytes": progress.transformed_logical_bytes,
            "stream_digest": progress.transformed_stream_digest
        })),
        "branch": progress.branch_node_id.as_ref().map(|node_id| json!({
            "node_instance_id": node_id,
            "true_count": progress.branch_true_count,
            "false_count": progress.branch_false_count,
            "stream_digest": progress.branch_stream_digest
        })),
        "artifact":progress.artifact,
        "complete":false
    });
    insert_checkpoint(
        &transaction,
        &progress.run_id,
        checkpoint,
        "running",
        1,
        &snapshot,
        &event.event_hash,
        committed_at,
    )?;
    let artifact_json = progress.artifact.as_ref().map(canonical_text).transpose()?;
    transaction.execute(
        "INSERT INTO run_generation_progress(run_id,state,generated_count,logical_bytes,stream_digest,backpressure_events,backpressure_micros,artifact_json,transform_node_id,transformed_count,transformed_logical_bytes,transformed_stream_digest,branch_node_id,branch_true_count,branch_false_count,branch_stream_digest,merge_json,updated_at)
         VALUES(?1,'running',?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,NULL,?16)
         ON CONFLICT(run_id) DO UPDATE SET state='running',generated_count=excluded.generated_count,logical_bytes=excluded.logical_bytes,stream_digest=excluded.stream_digest,backpressure_events=excluded.backpressure_events,backpressure_micros=excluded.backpressure_micros,artifact_json=excluded.artifact_json,transform_node_id=excluded.transform_node_id,transformed_count=excluded.transformed_count,transformed_logical_bytes=excluded.transformed_logical_bytes,transformed_stream_digest=excluded.transformed_stream_digest,branch_node_id=excluded.branch_node_id,branch_true_count=excluded.branch_true_count,branch_false_count=excluded.branch_false_count,branch_stream_digest=excluded.branch_stream_digest,merge_json=NULL,summary_json=NULL,updated_at=excluded.updated_at",
        params![progress.run_id,progress.generated_count as i64,progress.logical_bytes as i64,progress.stream_digest,progress.backpressure_events as i64,progress.backpressure_micros as i64,artifact_json,progress.transform_node_id,progress.transformed_count as i64,progress.transformed_logical_bytes as i64,progress.transformed_stream_digest,progress.branch_node_id,progress.branch_true_count as i64,progress.branch_false_count as i64,progress.branch_stream_digest,committed_at]
    ).map_err(storage_error)?;
    transaction.execute(
        "UPDATE runs SET checkpoint_sequence=?2,logical_order=1,attempted=1,succeeded=1,output_count=?3,trace_head_hash=?4,started_at=COALESCE(started_at,?5),updated_at=?6 WHERE run_id=?1 AND state='queued'",
        params![progress.run_id,checkpoint as i64,progress.generated_count as i64,event.event_hash,progress.started_at,committed_at]
    ).map_err(storage_error)?;
    let run = load_run(&transaction, &progress.run_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(run)
}

fn complete_generated_transaction(
    connection: &mut Connection,
    completed: CompletedGeneratedWork,
) -> Result<RunView, RunError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current = load_run(&transaction, &completed.run_id)?;
    if current.durable.terminal {
        transaction.commit().map_err(storage_error)?;
        return Ok(current);
    }
    let plan = load_execution_plan(&transaction, &completed.run_id)?;
    let manual_node = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "manual-trigger")
        .map(|node| node.node_instance_id.clone())
        .unwrap_or_else(|| "invalid-manual".into());
    let generate_node = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "generate-items")
        .map(|node| node.node_instance_id.clone())
        .unwrap_or_else(|| "invalid-generate".into());
    let transform_node = completed.transform_node_id.clone().or_else(|| {
        plan.nodes
            .iter()
            .find(|node| node.contract_lock.name == "edit-fields")
            .map(|node| node.node_instance_id.clone())
    });
    let branch_node = completed.branch_node_id.clone().or_else(|| {
        plan.nodes
            .iter()
            .find(|node| node.contract_lock.name == "if")
            .map(|node| node.node_instance_id.clone())
    });
    let merge_node = completed
        .merge
        .as_ref()
        .map(|merge| merge.node_instance_id.clone())
        .or_else(|| {
            plan.nodes
                .iter()
                .find(|node| node.contract_lock.name == "merge")
                .map(|node| node.node_instance_id.clone())
        });
    let summary_node = completed
        .summary_progress
        .as_ref()
        .map(|summary| summary.node_instance_id.clone())
        .or_else(|| {
            plan.nodes
                .iter()
                .find(|node| node.contract_lock.name == "summarize")
                .map(|node| node.node_instance_id.clone())
        });
    let has_transform = transform_node.is_some();
    let has_branch = branch_node.is_some();
    let has_merge = merge_node.is_some();
    let has_summary = summary_node.is_some();
    let cancel_won = current.durable.state == "cancel_requested" || completed.cancelled;
    let state = if cancel_won {
        "cancelled"
    } else if completed.summary.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let generate_outcome = if state == "succeeded" {
        "success"
    } else if state == "cancelled" {
        "cancelled"
    } else {
        "permanent_failure"
    };
    let failure = completed
        .summary
        .as_ref()
        .err()
        .map(|reason| json!({"code":reason.code,"message":reason.message}));
    let transform_failed = failure
        .as_ref()
        .and_then(|failure| failure["code"].as_str())
        .is_some_and(|code| code.starts_with("canopy.edit-fields"));
    let branch_failed = failure
        .as_ref()
        .and_then(|failure| failure["code"].as_str())
        .is_some_and(|code| code.starts_with("canopy.if"));
    let merge_failed = failure
        .as_ref()
        .and_then(|failure| failure["code"].as_str())
        .is_some_and(|code| code.starts_with("canopy.merge"));
    let summary_failed = failure
        .as_ref()
        .and_then(|failure| failure["code"].as_str())
        .is_some_and(|code| code.starts_with("canopy.summarize"));
    let transform_should_commit =
        has_transform && (state == "succeeded" || state == "cancelled" || transform_failed);
    let branch_should_commit =
        has_branch && (state == "succeeded" || state == "cancelled" || branch_failed);
    let merge_should_commit =
        has_merge && (state == "succeeded" || state == "cancelled" || merge_failed);
    let summary_should_commit =
        has_summary && (state == "succeeded" || state == "cancelled" || summary_failed);
    if state == "succeeded" && has_branch {
        if completed.branch_node_id.as_deref() != branch_node.as_deref()
            || completed
                .branch_true_count
                .saturating_add(completed.branch_false_count)
                != completed.generated_count
        {
            return Err(RunError::Integrity(
                "successful If execution does not cover every generated item".into(),
            ));
        }
    }
    if state == "succeeded" && has_merge {
        let Some(merge) = completed.merge.as_ref() else {
            return Err(RunError::Integrity(
                "successful Merge execution has no reducer evidence".into(),
            ));
        };
        if merge.node_instance_id != merge_node.as_deref().unwrap_or_default()
            || merge.output_count != completed.generated_count
            || merge.true_count.saturating_add(merge.false_count) != merge.output_count
        {
            return Err(RunError::Integrity(
                "successful Merge execution does not cover every routed item".into(),
            ));
        }
    }
    if state == "succeeded" && has_summary {
        let Some(summary) = completed.summary_progress.as_ref() else {
            return Err(RunError::Integrity(
                "successful Summarize execution has no reducer evidence".into(),
            ));
        };
        if summary.node_instance_id != summary_node.as_deref().unwrap_or_default()
            || summary.total_count != completed.generated_count
            || summary.true_count != completed.branch_true_count
            || summary.false_count != completed.branch_false_count
            || summary.true_count.saturating_add(summary.false_count) != summary.total_count
        {
            return Err(RunError::Integrity(
                "successful Summarize execution does not cover every merged item".into(),
            ));
        }
    }
    let branch_order = if transform_should_commit {
        4_u64
    } else {
        3_u64
    };
    let merge_order = branch_order + 1;
    let summary_order = merge_order + u64::from(has_merge);
    let final_order = if summary_should_commit {
        summary_order
    } else if merge_should_commit {
        merge_order
    } else if branch_should_commit {
        branch_order
    } else if transform_should_commit {
        3_u64
    } else {
        2_u64
    };
    let logical_activation_count = if state == "succeeded" && has_summary {
        2_u64
            .saturating_add(if has_transform {
                completed.transformed_count
            } else {
                0
            })
            .saturating_add(if has_branch {
                completed.generated_count
            } else {
                0
            })
            .saturating_add(u64::from(has_merge))
            .saturating_add(1)
    } else {
        final_order
    };
    let correctness_digest = if state == "succeeded" {
        if has_transform || has_branch || has_merge || has_summary {
            let mut logical_outcomes = vec![
                json!({
                    "logical_order":1,
                    "node_instance_id":manual_node,
                    "outcome":"success",
                    "port":"invocation",
                    "output":completed.input
                }),
                json!({
                    "logical_order":2,
                    "node_instance_id":generate_node,
                    "outcome":"success",
                    "port":"items",
                    "generated_count":completed.generated_count,
                    "logical_bytes":completed.logical_bytes,
                    "stream_digest":completed.stream_digest
                }),
            ];
            if let Some(transform_node) = transform_node.as_ref() {
                logical_outcomes.push(json!({
                    "logical_order":3,
                    "node_instance_id":transform_node,
                    "outcome":"success",
                    "port":"item",
                    "transformed_count":completed.transformed_count,
                    "logical_bytes":completed.transformed_logical_bytes,
                    "stream_digest":completed.transformed_stream_digest,
                    "item_linking":"one_to_one"
                }));
            }
            if let Some(branch_node) = branch_node.as_ref() {
                logical_outcomes.push(json!({
                    "logical_order":if has_transform { 4 } else { 3 },
                    "node_instance_id":branch_node,
                    "outcome":"success",
                    "output_ports":["true","false"],
                    "true_count":completed.branch_true_count,
                    "false_count":completed.branch_false_count,
                    "stream_digest":completed.branch_stream_digest,
                    "route_chain_schema":"canopy.if-route-chain/v1alpha1",
                    "exactly_one_output_per_item":true,
                    "item_linking":"one_to_one"
                }));
            }
            if let (Some(merge_node), Some(merge)) = (merge_node.as_ref(), completed.merge.as_ref())
            {
                logical_outcomes.push(json!({
                    "logical_order":merge_order,
                    "node_instance_id":merge_node,
                    "outcome":"success",
                    "input_ports":["true","false"],
                    "output_port":"items",
                    "mode":merge.mode,
                    "true_count":merge.true_count,
                    "false_count":merge.false_count,
                    "output_count":merge.output_count,
                    "logical_bytes":merge.logical_bytes,
                    "stream_digest":merge.stream_digest,
                    "spooling":"artifact-backed-segments",
                    "item_linking":"one_to_one"
                }));
            }
            if let (Some(summary_node), Some(reduced)) =
                (summary_node.as_ref(), completed.summary_progress.as_ref())
            {
                logical_outcomes.push(json!({
                    "logical_order":summary_order,
                    "node_instance_id":summary_node,
                    "outcome":"success",
                    "activation_shape":"barrier_reducer",
                    "operation":reduced.operation,
                    "total_count":reduced.total_count,
                    "true_count":reduced.true_count,
                    "false_count":reduced.false_count,
                    "logical_bytes":reduced.logical_bytes,
                    "output_digest_schema":summarize::OUTPUT_DIGEST_SCHEMA,
                    "output_digest_algorithm":summarize::OUTPUT_DIGEST_ALGORITHM,
                    "output_digest":reduced.output_digest,
                    "item_linking":"one_to_one"
                }));
            }
            digest(&json!({
                "schema":"canopy.correctness-digest/v1alpha2",
                "revision_digest":current.revision_digest,
                "plan_digest":current.plan_digest,
                "logical_outcomes":logical_outcomes
            }))
            .map_err(RunError::Integrity)?
        } else {
            completed
                .summary
                .as_ref()
                .map(|summary| summary.correctness_digest.clone())
                .map_err(|_| RunError::Integrity("successful generation has no summary".into()))?
        }
    } else {
        digest(&json!({
            "schema":"canopy.correctness-digest/v1alpha2",
            "revision_digest":current.revision_digest,
            "plan_digest":current.plan_digest,
            "complete":false,
            "generated_count":completed.generated_count,
            "stream_digest":completed.stream_digest,
            "transform_node_id":transform_node,
            "transformed_count":completed.transformed_count,
            "transformed_logical_bytes":completed.transformed_logical_bytes,
            "transformed_stream_digest":completed.transformed_stream_digest,
            "cancelled":cancel_won,
            "failure":failure
        }))
        .map_err(RunError::Integrity)?
    };
    let checkpoint = current.durable.checkpoint_sequence + 1;
    let committed_at = now_millis();
    let manual_activation = generated_activation_id(&completed.run_id, &manual_node, 1);
    let generate_activation = generated_activation_id(&completed.run_id, &generate_node, 2);
    let transform_activation = transform_node
        .as_ref()
        .map(|node| generated_activation_id(&completed.run_id, node, 3));
    let branch_activation = branch_node
        .as_ref()
        .map(|node| generated_activation_id(&completed.run_id, node, branch_order));
    let merge_activation = merge_node
        .as_ref()
        .map(|node| generated_activation_id(&completed.run_id, node, merge_order));
    let summary_activation = summary_node
        .as_ref()
        .map(|node| generated_activation_id(&completed.run_id, node, summary_order));
    let manual_committed = current.durable.logical_order >= 1;
    if !manual_committed {
        insert_activation_row(
            &transaction,
            ActivationInsert {
                activation_id: &manual_activation,
                run_id: &completed.run_id,
                node_id: &manual_node,
                logical_order: 1,
                outcome: "success",
                input: &completed.input,
                output: Some(&completed.input),
                failure: None,
                checkpoint,
                started_at: completed.started_at,
                completed_at: completed.started_at,
                elapsed_micros: 0,
                provenance: json!({"engine_abi":generate_engine::GENERATE_ENGINE_ABI,"lane":"native-cpu","effect_class":"pure","output_port":"invocation"}),
            },
        )?;
    }
    let generate_output = if state == "succeeded" {
        Some(json!({
            "generated_count":completed.generated_count,"logical_bytes":completed.logical_bytes,
            "stream_digest":completed.stream_digest,"artifact":completed.artifact
        }))
    } else {
        None
    };
    insert_activation_row(
        &transaction,
        ActivationInsert {
            activation_id: &generate_activation,
            run_id: &completed.run_id,
            node_id: &generate_node,
            logical_order: 2,
            outcome: generate_outcome,
            input: &completed.input,
            output: generate_output.as_ref(),
            failure: failure.as_ref(),
            checkpoint,
            started_at: completed.started_at,
            completed_at: completed.completed_at,
            elapsed_micros: completed.elapsed_micros,
            provenance: json!({"engine_abi":generate_engine::GENERATE_ENGINE_ABI,"lane":"native-cpu","effect_class":"pure","output_port":"items","backpressure_micros":completed.backpressure_micros}),
        },
    )?;
    let transform_outcome = if state == "succeeded" {
        "success"
    } else if state == "cancelled" {
        "cancelled"
    } else {
        "permanent_failure"
    };
    let transform_input = json!({
        "source_node_instance_id":generate_node,
        "source_output_port":"items",
        "generated_count":completed.generated_count,
        "stream_digest":completed.stream_digest
    });
    let transform_output = json!({
        "transformed_count":completed.transformed_count,
        "logical_bytes":completed.transformed_logical_bytes,
        "stream_digest":completed.transformed_stream_digest,
        "item_linking":"one_to_one"
    });
    if transform_should_commit {
        let transform_node = transform_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("transform activation has no node".into()))?;
        let transform_activation = transform_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("transform activation has no identity".into()))?;
        insert_activation_row(
            &transaction,
            ActivationInsert {
                activation_id: transform_activation,
                run_id: &completed.run_id,
                node_id: transform_node,
                logical_order: 3,
                outcome: transform_outcome,
                input: &transform_input,
                output: (state == "succeeded").then_some(&transform_output),
                failure: if transform_failed {
                    failure.as_ref()
                } else {
                    None
                },
                checkpoint,
                started_at: completed.started_at,
                completed_at: completed.completed_at,
                elapsed_micros: completed.elapsed_micros,
                provenance: json!({"engine_abi":edit_fields::EDIT_FIELDS_ABI,"lane":"native-cpu","effect_class":"pure","input_link":"generate.items","output_port":"item","item_linking":"one_to_one"}),
            },
        )?;
    }
    let branch_outcome = if state == "succeeded" {
        "success"
    } else if state == "cancelled" {
        "cancelled"
    } else {
        "permanent_failure"
    };
    let branch_input = json!({
        "source_node_instance_id":if transform_should_commit {
            transform_node.as_ref().unwrap_or(&generate_node)
        } else {
            &generate_node
        },
        "source_output_port":if transform_should_commit { "item" } else { "items" },
        "generated_count":completed.generated_count,
        "stream_digest":if transform_should_commit {
            &completed.transformed_stream_digest
        } else {
            &completed.stream_digest
        }
    });
    let branch_output = json!({
        "true_count":completed.branch_true_count,
        "false_count":completed.branch_false_count,
        "stream_digest":completed.branch_stream_digest,
        "route_chain_schema":"canopy.if-route-chain/v1alpha1",
        "exactly_one_output_per_item":true,
        "output_ports":["true","false"],
        "item_linking":"one_to_one"
    });
    let merge_outcome = if state == "succeeded" {
        "success"
    } else if state == "cancelled" {
        "cancelled"
    } else {
        "permanent_failure"
    };
    let merge_input = json!({
        "source_node_instance_id":branch_node,
        "source_output_ports":["true","false"],
        "true_count":completed.branch_true_count,
        "false_count":completed.branch_false_count,
        "closed_before_reduce":true
    });
    let merge_output = completed.merge.as_ref().map(|merge| {
        json!({
            "mode":merge.mode,
            "true_count":merge.true_count,
            "false_count":merge.false_count,
            "output_count":merge.output_count,
            "logical_bytes":merge.logical_bytes,
            "stream_digest":merge.stream_digest,
            "physical_spool_bytes":merge.physical_spool_bytes,
            "true_segments":merge.true_segments,
            "false_segments":merge.false_segments,
            "output_segments":merge.output_segments,
            "item_linking":"one_to_one"
        })
    });
    let summary_outcome = if state == "succeeded" {
        "success"
    } else if state == "cancelled" {
        "cancelled"
    } else {
        "permanent_failure"
    };
    let summary_input = json!({
        "source_node_instance_id": merge_node,
        "source_output_port":"items",
        "closed_before_reduce":true,
        "output_count":completed.merge.as_ref().map_or(0, |merge| merge.output_count),
        "stream_digest":completed.merge.as_ref().map_or_else(|| "genesis".into(), |merge| merge.stream_digest.clone())
    });
    let summary_output = completed.summary_progress.as_ref().map(|reduced| {
        json!({
            "operation":reduced.operation,
            "total_count":reduced.total_count,
            "true_count":reduced.true_count,
            "false_count":reduced.false_count,
            "logical_bytes":reduced.logical_bytes,
            "output_digest_schema":summarize::OUTPUT_DIGEST_SCHEMA,
            "output_digest_algorithm":summarize::OUTPUT_DIGEST_ALGORITHM,
            "output_digest":reduced.output_digest,
            "first_ordinal":reduced.first_ordinal,
            "last_ordinal":reduced.last_ordinal
        })
    });
    if branch_should_commit {
        let branch_node = branch_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("If activation has no node".into()))?;
        let branch_activation = branch_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("If activation has no identity".into()))?;
        insert_activation_row(
            &transaction,
            ActivationInsert {
                activation_id: branch_activation,
                run_id: &completed.run_id,
                node_id: branch_node,
                logical_order: branch_order,
                outcome: branch_outcome,
                input: &branch_input,
                output: (state == "succeeded").then_some(&branch_output),
                failure: if branch_failed {
                    failure.as_ref()
                } else {
                    None
                },
                checkpoint,
                started_at: completed.started_at,
                completed_at: completed.completed_at,
                elapsed_micros: completed.elapsed_micros,
                provenance: json!({"engine_abi":if_node::IF_ABI,"lane":"native-cpu","effect_class":"pure","input_link":if transform_should_commit { "edit-fields.item" } else { "generate.items" },"output_ports":["true","false"],"item_linking":"one_to_one"}),
            },
        )?;
    }
    if merge_should_commit {
        let merge_node = merge_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Merge activation has no node".into()))?;
        let merge_activation = merge_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Merge activation has no identity".into()))?;
        insert_activation_row(
            &transaction,
            ActivationInsert {
                activation_id: merge_activation,
                run_id: &completed.run_id,
                node_id: merge_node,
                logical_order: merge_order,
                outcome: merge_outcome,
                input: &merge_input,
                output: if state == "succeeded" {
                    merge_output.as_ref()
                } else {
                    None
                },
                failure: if merge_failed { failure.as_ref() } else { None },
                checkpoint,
                started_at: completed.started_at,
                completed_at: completed.completed_at,
                elapsed_micros: completed.elapsed_micros,
                provenance: json!({
                    "engine_abi":merge::MERGE_ABI,
                    "lane":"native-cpu",
                    "effect_class":"pure",
                    "activation_shape":"barrier_reducer",
                    "input_ports":["true","false"],
                    "output_port":"items",
                    "spooling":"artifact-backed-segments",
                    "item_linking":"one_to_one"
                }),
            },
        )?;
    }
    if summary_should_commit {
        let summary_node = summary_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Summarize activation has no node".into()))?;
        let summary_activation = summary_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Summarize activation has no identity".into()))?;
        insert_activation_row(
            &transaction,
            ActivationInsert {
                activation_id: summary_activation,
                run_id: &completed.run_id,
                node_id: summary_node,
                logical_order: summary_order,
                outcome: summary_outcome,
                input: &summary_input,
                output: if state == "succeeded" {
                    summary_output.as_ref()
                } else {
                    None
                },
                failure: if summary_failed {
                    failure.as_ref()
                } else {
                    None
                },
                checkpoint,
                started_at: completed.started_at,
                completed_at: completed.completed_at,
                elapsed_micros: completed.elapsed_micros,
                provenance: json!({
                    "engine_abi":summarize::SUMMARIZE_ABI,
                    "lane":"native-cpu",
                    "effect_class":"pure",
                    "activation_shape":"barrier_reducer",
                    "input_port":"items",
                    "output_port":"summary",
                    "bounded_state":"counters-and-digest-only",
                    "output_digest_algorithm":summarize::OUTPUT_DIGEST_ALGORITHM
                }),
            },
        )?;
    }
    let mut previous = current_trace_head(&transaction, &completed.run_id)?;
    let mut sequence = next_trace_sequence(&transaction, &completed.run_id)?;
    if !manual_committed {
        let manual_event = make_trace_event(
            &completed.run_id,
            sequence,
            Some(1),
            checkpoint,
            "logical",
            "activation_outcome",
            json!({
                "activation_id":manual_activation,"node_instance_id":manual_node,"attempt":1,"outcome":"success",
                "input_digest":digest(&completed.input).map_err(RunError::Integrity)?,"output_digest":digest(&completed.input).map_err(RunError::Integrity)?
            }),
            &previous,
            committed_at,
        )?;
        insert_trace_event(&transaction, &manual_event)?;
        previous = manual_event.event_hash;
        sequence += 1;
    }
    let generate_event = make_trace_event(
        &completed.run_id,
        sequence,
        Some(2),
        checkpoint,
        "logical",
        "activation_outcome",
        json!({
            "activation_id":generate_activation,"node_instance_id":generate_node,"attempt":1,"outcome":generate_outcome,
            "generated_count":completed.generated_count,"logical_bytes":completed.logical_bytes,"stream_digest":completed.stream_digest,
            "artifact":completed.artifact,"failure":failure,"input_digest":digest(&completed.input).map_err(RunError::Integrity)?,
            "output_digest":generate_output.as_ref().map(digest).transpose().map_err(RunError::Integrity)?
        }),
        &previous,
        committed_at,
    )?;
    insert_trace_event(&transaction, &generate_event)?;
    previous = generate_event.event_hash.clone();
    sequence += 1;
    if transform_should_commit {
        let transform_node = transform_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("transform trace has no node".into()))?;
        let transform_activation = transform_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("transform trace has no identity".into()))?;
        let transform_event = make_trace_event(
            &completed.run_id,
            sequence,
            Some(3),
            checkpoint,
            "logical",
            "activation_outcome",
            json!({
                "activation_id":transform_activation,"node_instance_id":transform_node,"attempt":1,"outcome":transform_outcome,
                "generated_count":completed.generated_count,"transformed_count":completed.transformed_count,
                "logical_bytes":completed.transformed_logical_bytes,"stream_digest":completed.transformed_stream_digest,
                "item_linking":"one_to_one","failure":if transform_failed { failure.clone() } else { None::<Value> },
                "input_digest":digest(&transform_input).map_err(RunError::Integrity)?,
                "output_digest":if state == "succeeded" { Some(digest(&transform_output).map_err(RunError::Integrity)?) } else { None }
            }),
            &previous,
            committed_at,
        )?;
        insert_trace_event(&transaction, &transform_event)?;
        previous = transform_event.event_hash;
        sequence += 1;
    }
    if branch_should_commit {
        let branch_node = branch_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("If trace has no node".into()))?;
        let branch_activation = branch_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("If trace has no identity".into()))?;
        let branch_event = make_trace_event(
            &completed.run_id,
            sequence,
            Some(branch_order),
            checkpoint,
            "logical",
            "activation_outcome",
            json!({
                "activation_id":branch_activation,
                "node_instance_id":branch_node,
                "attempt":1,
                "outcome":branch_outcome,
                "true_count":completed.branch_true_count,
                "false_count":completed.branch_false_count,
                "stream_digest":completed.branch_stream_digest,
                "route_chain_schema":"canopy.if-route-chain/v1alpha1",
                "exactly_one_output_per_item":true,
                "output_ports":["true","false"],
                "failure":if branch_failed { failure.clone() } else { None::<Value> },
                "input_digest":digest(&branch_input).map_err(RunError::Integrity)?,
                "output_digest":if state == "succeeded" { Some(digest(&branch_output).map_err(RunError::Integrity)?) } else { None }
            }),
            &previous,
            committed_at,
        )?;
        insert_trace_event(&transaction, &branch_event)?;
        previous = branch_event.event_hash;
        sequence += 1;
    }
    if merge_should_commit {
        let merge_node = merge_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Merge trace has no node".into()))?;
        let merge_activation = merge_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Merge trace has no identity".into()))?;
        let merge_event = make_trace_event(
            &completed.run_id,
            sequence,
            Some(merge_order),
            checkpoint,
            "logical",
            "activation_outcome",
            json!({
                "activation_id":merge_activation,
                "node_instance_id":merge_node,
                "attempt":1,
                "outcome":merge_outcome,
                "input_ports":["true","false"],
                "output_port":"items",
                "mode":completed.merge.as_ref().map(|merge| merge.mode.clone()).unwrap_or_else(|| "true_then_false".into()),
                "true_count":completed.merge.as_ref().map_or(0, |merge| merge.true_count),
                "false_count":completed.merge.as_ref().map_or(0, |merge| merge.false_count),
                "output_count":completed.merge.as_ref().map_or(0, |merge| merge.output_count),
                "logical_bytes":completed.merge.as_ref().map_or(0, |merge| merge.logical_bytes),
                "stream_digest":completed.merge.as_ref().map_or_else(|| "genesis".into(), |merge| merge.stream_digest.clone()),
                "physical_spool_bytes":completed.merge.as_ref().map_or(0, |merge| merge.physical_spool_bytes),
                "spooling":"artifact-backed-segments",
                "failure":if merge_failed { failure.clone() } else { None::<Value> },
                "input_digest":digest(&merge_input).map_err(RunError::Integrity)?,
                "output_digest":merge_output.as_ref().map(digest).transpose().map_err(RunError::Integrity)?
            }),
            &previous,
            committed_at,
        )?;
        insert_trace_event(&transaction, &merge_event)?;
        previous = merge_event.event_hash;
        sequence += 1;
    }
    if summary_should_commit {
        let summary_node = summary_node
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Summarize trace has no node".into()))?;
        let summary_activation = summary_activation
            .as_ref()
            .ok_or_else(|| RunError::Integrity("Summarize trace has no identity".into()))?;
        let summary_event = make_trace_event(
            &completed.run_id,
            sequence,
            Some(summary_order),
            checkpoint,
            "logical",
            "activation_outcome",
            json!({
                "activation_id":summary_activation,
                "node_instance_id":summary_node,
                "attempt":1,
                "outcome":summary_outcome,
                "activation_shape":"barrier_reducer",
                "input_port":"items",
                "output_port":"summary",
                "operation":completed.summary_progress.as_ref().map_or_else(|| "output_digest".into(), |summary| summary.operation.clone()),
                "total_count":completed.summary_progress.as_ref().map_or(0, |summary| summary.total_count),
                "true_count":completed.summary_progress.as_ref().map_or(0, |summary| summary.true_count),
                "false_count":completed.summary_progress.as_ref().map_or(0, |summary| summary.false_count),
                "logical_bytes":completed.summary_progress.as_ref().map_or(0, |summary| summary.logical_bytes),
                "output_digest_schema":summarize::OUTPUT_DIGEST_SCHEMA,
                "output_digest_algorithm":summarize::OUTPUT_DIGEST_ALGORITHM,
                "output_digest":completed.summary_progress.as_ref().map(|summary| summary.output_digest.clone()),
                "bounded_state":"counters-and-digest-only",
                "causal_trace_links":{
                    "source_merge_output_digest":completed.merge.as_ref().map(|merge| merge.stream_digest.clone()),
                    "retained_ordinal_range":[completed.summary_progress.as_ref().and_then(|summary| summary.first_ordinal),completed.summary_progress.as_ref().and_then(|summary| summary.last_ordinal)],
                    "retained_item_provenance":"artifact-backed-merge-spool"
                },
                "failure":if summary_failed { failure.clone() } else { None::<Value> },
                "input_digest":digest(&summary_input).map_err(RunError::Integrity)?,
                "output_digest_value":summary_output.as_ref().map(digest).transpose().map_err(RunError::Integrity)?
            }),
            &previous,
            committed_at,
        )?;
        insert_trace_event(&transaction, &summary_event)?;
        previous = summary_event.event_hash;
        sequence += 1;
    }
    let checkpoint_event = make_trace_event(
        &completed.run_id,
        sequence,
        Some(final_order),
        checkpoint,
        "physical",
        "checkpoint_committed",
        json!({
            "state":state,"logical_order":final_order,"logical_activation_count":logical_activation_count,"generated_count":completed.generated_count,"logical_bytes":completed.logical_bytes,
            "stream_digest":completed.stream_digest,"transform_node_id":transform_node,"transformed_count":completed.transformed_count,
            "transformed_logical_bytes":completed.transformed_logical_bytes,"transformed_stream_digest":completed.transformed_stream_digest,
            "branch_node_id":branch_node,"branch_true_count":completed.branch_true_count,"branch_false_count":completed.branch_false_count,
            "branch_stream_digest":completed.branch_stream_digest,
            "merge_node_id":merge_node,"merge":completed.merge,
            "summary_node_id":summary_node,"summary":completed.summary_progress,
            "correctness_digest":correctness_digest,"backpressure_micros":completed.backpressure_micros,
            "safe_resources":safe_resource_facts()
        }),
        &previous,
        committed_at,
    )?;
    insert_trace_event(&transaction, &checkpoint_event)?;
    let attempted = logical_activation_count as i64;
    let (succeeded, cancelled, failed) = match state {
        "succeeded" => (attempted, 0, 0),
        "cancelled" => (1, attempted.saturating_sub(1), 0),
        _ if transform_failed || branch_failed || merge_failed => {
            (attempted.saturating_sub(1), 0, 1)
        }
        _ => (1, 0, 1),
    };
    let output_count = if has_merge {
        completed
            .merge
            .as_ref()
            .map_or(completed.generated_count, |merge| merge.output_count)
    } else if has_transform {
        completed.transformed_count
    } else {
        completed.generated_count
    };
    let snapshot = json!({
        "resume":{"generate_next_ordinal":completed.generated_count},"logical_order":final_order,"logical_activation_count":logical_activation_count,"state":state,
        "generated_count":completed.generated_count,"logical_bytes":completed.logical_bytes,"stream_digest":completed.stream_digest,
        "transform_node_id":transform_node,"transformed_count":completed.transformed_count,
        "transformed_logical_bytes":completed.transformed_logical_bytes,"transformed_stream_digest":completed.transformed_stream_digest,
        "branch_node_id":branch_node,"branch_true_count":completed.branch_true_count,"branch_false_count":completed.branch_false_count,
        "branch_stream_digest":completed.branch_stream_digest,
        "correctness_digest":correctness_digest,"complete":state=="succeeded","artifact":completed.artifact,
        "counters":{"attempted":attempted,"succeeded":succeeded,"cancelled":cancelled,"failed":failed,"output_count":output_count}
    });
    insert_checkpoint(
        &transaction,
        &completed.run_id,
        checkpoint,
        state,
        final_order,
        &snapshot,
        &checkpoint_event.event_hash,
        committed_at,
    )?;
    let artifact_json = completed
        .artifact
        .as_ref()
        .map(canonical_text)
        .transpose()?;
    let merge_json = completed.merge.as_ref().map(canonical_text).transpose()?;
    let summary_json = completed
        .summary_progress
        .as_ref()
        .map(canonical_text)
        .transpose()?;
    transaction.execute(
        "INSERT INTO run_generation_progress(run_id,state,generated_count,logical_bytes,stream_digest,backpressure_events,backpressure_micros,artifact_json,transform_node_id,transformed_count,transformed_logical_bytes,transformed_stream_digest,branch_node_id,branch_true_count,branch_false_count,branch_stream_digest,merge_json,summary_json,updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)
         ON CONFLICT(run_id) DO UPDATE SET state=excluded.state,generated_count=excluded.generated_count,logical_bytes=excluded.logical_bytes,stream_digest=excluded.stream_digest,backpressure_events=excluded.backpressure_events,backpressure_micros=excluded.backpressure_micros,artifact_json=excluded.artifact_json,transform_node_id=excluded.transform_node_id,transformed_count=excluded.transformed_count,transformed_logical_bytes=excluded.transformed_logical_bytes,transformed_stream_digest=excluded.transformed_stream_digest,branch_node_id=excluded.branch_node_id,branch_true_count=excluded.branch_true_count,branch_false_count=excluded.branch_false_count,branch_stream_digest=excluded.branch_stream_digest,merge_json=excluded.merge_json,summary_json=excluded.summary_json,updated_at=excluded.updated_at",
        params![completed.run_id,state,completed.generated_count as i64,completed.logical_bytes as i64,completed.stream_digest,completed.backpressure_events as i64,completed.backpressure_micros as i64,artifact_json,transform_node,completed.transformed_count as i64,completed.transformed_logical_bytes as i64,completed.transformed_stream_digest,completed.branch_node_id,completed.branch_true_count as i64,completed.branch_false_count as i64,completed.branch_stream_digest,merge_json,summary_json,committed_at]
    ).map_err(storage_error)?;
    transaction.execute(
        "UPDATE runs SET state=?2,checkpoint_sequence=?3,logical_order=?4,attempted=?5,succeeded=?6,cancelled=?7,failed=?8,output_count=?9,correctness_digest=?10,digest_complete=?11,trace_head_hash=?12,started_at=COALESCE(started_at,?13),updated_at=?14,terminal_at=?14 WHERE run_id=?1 AND state IN ('queued','cancel_requested')",
        params![completed.run_id,state,checkpoint as i64,final_order as i64,attempted,succeeded,cancelled,failed,output_count as i64,correctness_digest,if state=="succeeded"{1}else{0},checkpoint_event.event_hash,completed.started_at,committed_at]
    ).map_err(storage_error)?;
    let run = load_run(&transaction, &completed.run_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(run)
}

struct ActivationInsert<'a> {
    activation_id: &'a str,
    run_id: &'a str,
    node_id: &'a str,
    logical_order: u64,
    outcome: &'a str,
    input: &'a Value,
    output: Option<&'a Value>,
    failure: Option<&'a Value>,
    checkpoint: u64,
    started_at: i64,
    completed_at: i64,
    elapsed_micros: u64,
    provenance: Value,
}

fn insert_activation_row(
    transaction: &Transaction<'_>,
    value: ActivationInsert<'_>,
) -> Result<(), RunError> {
    let input_json = canonical_text(value.input)?;
    let output_json = value.output.map(canonical_text).transpose()?;
    let failure_json = value.failure.map(canonical_text).transpose()?;
    let provenance_json = canonical_text(&value.provenance)?;
    let input_digest = digest(value.input).map_err(RunError::Integrity)?;
    let output_digest = value
        .output
        .map(digest)
        .transpose()
        .map_err(RunError::Integrity)?;
    transaction.execute(
        "INSERT INTO run_activations(activation_id,run_id,node_instance_id,logical_order,attempt,outcome,input_json,output_json,input_digest,output_digest,provenance_json,failure_json,checkpoint_sequence,started_at,completed_at,elapsed_micros)
         VALUES(?1,?2,?3,?4,1,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![value.activation_id,value.run_id,value.node_id,value.logical_order as i64,value.outcome,input_json,output_json,input_digest,output_digest,provenance_json,failure_json,value.checkpoint as i64,value.started_at,value.completed_at,value.elapsed_micros as i64]
    ).map_err(storage_error)?;
    Ok(())
}

fn generated_activation_id(run_id: &str, node_id: &str, logical_order: u64) -> String {
    let value=digest(&json!({"schema":"canopy.activation-identity/v1alpha1","run_id":run_id,"node_instance_id":node_id,"logical_order":logical_order,"attempt":1})).expect("Activation identity is serializable");
    format!("activation-{}", &value[7..39])
}

fn current_trace_head(transaction: &Transaction<'_>, run_id: &str) -> Result<String, RunError> {
    transaction
        .query_row(
            "SELECT trace_head_hash FROM runs WHERE run_id=?1",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

fn load_execution_plan(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<ExecutionPlan, RunError> {
    let text:String=transaction.query_row("SELECT p.payload_json FROM runs r JOIN execution_plans p ON p.plan_id=r.plan_id WHERE r.run_id=?1",params![run_id],|row|row.get(0)).map_err(storage_error)?;
    serde_json::from_str(&text)
        .map_err(|error| RunError::Integrity(format!("pinned plan is invalid: {error}")))
}

fn complete_transaction(
    connection: &mut Connection,
    completed: CompletedWork,
) -> Result<RunView, RunError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let current = load_run(&transaction, &completed.run_id)?;
    if current.durable.terminal {
        transaction.commit().map_err(storage_error)?;
        return Ok(current);
    }
    let cancel_won = current.durable.state == "cancel_requested";
    let mut result = completed.result;
    if cancel_won {
        let late_outcome = outcome_name(&result.outcome).to_owned();
        result.outcome = ActivationOutcome::Cancelled;
        result.output = None;
        result.correctness_digest = run_engine::cancelled_correctness_digest(
            &current.revision_digest,
            &current.plan_digest,
        );
        result.counters.succeeded = 0;
        result.counters.failed = 0;
        result.counters.cancelled = 1;
        result.counters.output_count = 0;
        result.failure = None;
        result.provenance["late_speculative_outcome"] = Value::String(late_outcome);
    }
    let state = match result.outcome {
        ActivationOutcome::Success => "succeeded",
        ActivationOutcome::Cancelled => "cancelled",
        ActivationOutcome::PermanentFailure => "failed",
    };
    let checkpoint = current.durable.checkpoint_sequence + 1;
    let committed_at = now_millis();
    let input_json = canonical_text(&result.input)?;
    let output_json = result.output.as_ref().map(canonical_text).transpose()?;
    let provenance_json = canonical_text(&result.provenance)?;
    let failure_json = result.failure.as_ref().map(canonical_text).transpose()?;
    let input_digest = digest(&result.input).map_err(RunError::Integrity)?;
    let output_digest = result
        .output
        .as_ref()
        .map(digest)
        .transpose()
        .map_err(RunError::Integrity)?;
    transaction
        .execute(
            "INSERT INTO run_activations(activation_id,run_id,node_instance_id,logical_order,attempt,outcome,input_json,output_json,input_digest,output_digest,provenance_json,failure_json,checkpoint_sequence,started_at,completed_at,elapsed_micros)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                result.activation_id,
                completed.run_id,
                result.node_instance_id,
                result.logical_order as i64,
                result.attempt as i64,
                outcome_name(&result.outcome),
                input_json,
                output_json,
                input_digest,
                output_digest,
                provenance_json,
                failure_json,
                checkpoint as i64,
                completed.started_at,
                completed.completed_at,
                completed.elapsed_micros as i64
            ],
        )
        .map_err(storage_error)?;

    let previous_hash: String = transaction
        .query_row(
            "SELECT trace_head_hash FROM runs WHERE run_id=?1",
            params![completed.run_id],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    let activation_sequence = next_trace_sequence(&transaction, &completed.run_id)?;
    let activation_event = make_trace_event(
        &completed.run_id,
        activation_sequence,
        Some(result.logical_order),
        checkpoint,
        "logical",
        "activation_outcome",
        json!({
            "activation_id": result.activation_id,
            "node_instance_id": result.node_instance_id,
            "attempt": result.attempt,
            "outcome": outcome_name(&result.outcome),
            "input_digest": input_digest,
            "output_digest": output_digest,
            "provenance": result.provenance,
            "failure": result.failure
        }),
        &previous_hash,
        committed_at,
    )?;
    insert_trace_event(&transaction, &activation_event)?;
    let checkpoint_event = make_trace_event(
        &completed.run_id,
        activation_sequence + 1,
        Some(result.logical_order),
        checkpoint,
        "physical",
        "checkpoint_committed",
        json!({
            "state": state,
            "logical_order": result.logical_order,
            "correctness_digest": result.correctness_digest,
            "counters": result.counters,
            "timing": {
                "started_at": completed.started_at,
                "completed_at": completed.completed_at,
                "elapsed_micros": completed.elapsed_micros
            },
            "safe_resources": safe_resource_facts()
        }),
        &activation_event.event_hash,
        committed_at,
    )?;
    insert_trace_event(&transaction, &checkpoint_event)?;
    let snapshot = json!({
        "resume": {"next_logical_order": result.logical_order + 1},
        "logical_order": result.logical_order,
        "outcome": outcome_name(&result.outcome),
        "correctness_digest": result.correctness_digest,
        "complete": state == "succeeded",
        "counters": result.counters,
        "provenance": result.provenance
    });
    insert_checkpoint(
        &transaction,
        &completed.run_id,
        checkpoint,
        state,
        result.logical_order,
        &snapshot,
        &checkpoint_event.event_hash,
        committed_at,
    )?;
    transaction
        .execute(
            "UPDATE runs SET state=?2,checkpoint_sequence=?3,logical_order=?4,attempted=?5,succeeded=?6,cancelled=?7,failed=?8,output_count=?9,correctness_digest=?10,digest_complete=?11,trace_head_hash=?12,started_at=COALESCE(started_at,?13),updated_at=?14,terminal_at=?14 WHERE run_id=?1 AND state IN ('queued','cancel_requested')",
            params![
                completed.run_id,
                state,
                checkpoint as i64,
                result.logical_order as i64,
                result.counters.attempted as i64,
                result.counters.succeeded as i64,
                result.counters.cancelled as i64,
                result.counters.failed as i64,
                result.counters.output_count as i64,
                result.correctness_digest,
                if state == "succeeded" { 1 } else { 0 },
                checkpoint_event.event_hash,
                completed.started_at,
                committed_at
            ],
        )
        .map_err(storage_error)?;
    let run = load_run(&transaction, &completed.run_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(run)
}

fn recover_cancellations(connection: &mut Connection) -> Result<(), String> {
    let run_ids = {
        let mut statement = connection
            .prepare("SELECT run_id FROM runs WHERE state='cancel_requested' ORDER BY run_id")
            .map_err(|error| error.to_string())?;
        let values = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        drop(statement);
        values
    };
    for run_id in run_ids {
        finalize_requested_cancellation(
            connection,
            &run_id,
            "cancellation_recovered",
            "daemon_restarted_after_durable_cancellation",
        )
        .map_err(|error| format!("{error:?}"))?;
    }
    Ok(())
}

fn finalize_requested_cancellation(
    connection: &mut Connection,
    run_id: &str,
    event_type: &str,
    reason: &str,
) -> Result<(), RunError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let run = load_run(&transaction, run_id)?;
    let previous_hash: String = transaction
        .query_row(
            "SELECT trace_head_hash FROM runs WHERE run_id=?1",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    let checkpoint = run.durable.checkpoint_sequence + 1;
    let occurred_at = now_millis();
    let event = make_trace_event(
        run_id,
        next_trace_sequence(&transaction, run_id)?,
        None,
        checkpoint,
        "control",
        event_type,
        json!({"state": "cancelled", "reason": reason}),
        &previous_hash,
        occurred_at,
    )?;
    insert_trace_event(&transaction, &event)?;
    let correctness =
        run_engine::cancelled_correctness_digest(&run.revision_digest, &run.plan_digest);
    insert_checkpoint(
        &transaction,
        run_id,
        checkpoint,
        "cancelled",
        run.durable.logical_order,
        &json!({
            "resume": {"next_logical_order": run.durable.logical_order + 1},
            "correctness_digest": correctness,
            "complete": false,
            "settled": true,
            "recovered_after_restart": event_type == "cancellation_recovered",
            "counters": counters_json(
                run.correctness.attempted,
                run.correctness.succeeded,
                run.correctness.cancelled,
                run.correctness.failed,
                run.correctness.output_count
            )
        }),
        &event.event_hash,
        occurred_at,
    )?;
    transaction
        .execute(
            "UPDATE runs SET state='cancelled',checkpoint_sequence=?2,correctness_digest=?3,digest_complete=0,trace_head_hash=?4,updated_at=?5,terminal_at=?5 WHERE run_id=?1 AND state='cancel_requested'",
            params![run_id, checkpoint as i64, correctness, event.event_hash, occurred_at],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)
}

struct SchedulerContext {
    database: PathBuf,
    writer: WriterClient,
    ready: SyncSender<QueuedWork>,
    ready_budget: Arc<ByteBudget>,
    result_budget: Arc<ByteBudget>,
    results: Receiver<ExecutorTerminal>,
    envelopes: Receiver<GeneratedBatchEvent>,
    wake: Receiver<SchedulerSignal>,
    artifacts: Arc<ArtifactService>,
    controls: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    live: Arc<LiveHub>,
}

enum SchedulerSignal {
    Wake,
    Stop,
}

struct Candidate {
    run_id: String,
    revision_id: String,
    revision_digest: String,
    plan_digest: String,
    plan: ExecutionPlan,
    captured_invocation: Value,
    checkpoint_sequence: u64,
    generate_resume: Option<GenerateResume>,
    edit_fields: Option<EditFieldsCandidate>,
    if_node: Option<IfCandidate>,
    merge: Option<MergeCandidate>,
    summarize: Option<SummarizeCandidate>,
    logical_data_override: Option<Value>,
    artifact: Option<ArtifactReference>,
}

struct EditFieldsCandidate {
    node_id: String,
    configuration: Value,
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_stream_digest: String,
}

struct IfCandidate {
    node_id: String,
    configuration: Value,
    true_count: u64,
    false_count: u64,
    stream_digest: String,
}

struct MergeCandidate {
    node_id: String,
    configuration: Value,
}

struct SummarizeCandidate {
    node_id: String,
    configuration: Value,
}

struct QueuedWork {
    work: WorkItem,
    _ready_bytes: BytePermit,
}

struct WorkItem {
    candidate: Candidate,
    cancellation: Arc<AtomicBool>,
    _result_bytes: BytePermit,
}

enum ExecutorTerminal {
    Manual(CompletedWork),
    Generated(CompletedGeneratedWork),
}

struct CompletedWork {
    run_id: String,
    result: ManualActivationResult,
    started_at: i64,
    completed_at: i64,
    elapsed_micros: u64,
    _result_bytes: BytePermit,
}

struct GeneratedBatchEvent {
    run_id: String,
    envelopes: Vec<GeneratedEnvelope>,
    generated_count: u64,
    logical_bytes: u64,
    stream_digest: String,
    transform_node_id: Option<String>,
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_batch_logical_bytes: u64,
    transformed_stream_digest: String,
    branch_node_id: Option<String>,
    branch_true_count: u64,
    branch_false_count: u64,
    branch_stream_digest: String,
    branch_batch_true_count: u64,
    branch_batch_false_count: u64,
    branch_previous_stream_digest: String,
    backpressure_micros: u64,
    artifact: Option<ArtifactReference>,
    started_at: i64,
}

#[derive(Debug)]
struct AppliedEditFieldsBatch {
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_batch_logical_bytes: u64,
    transformed_stream_digest: String,
}

struct GeneratedProgressCommit {
    run_id: String,
    generated_count: u64,
    logical_bytes: u64,
    stream_digest: String,
    transform_node_id: Option<String>,
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_stream_digest: String,
    branch_node_id: Option<String>,
    branch_true_count: u64,
    branch_false_count: u64,
    branch_stream_digest: String,
    backpressure_events: u64,
    backpressure_micros: u64,
    first_ordinal: u64,
    last_ordinal: u64,
    artifact: Option<ArtifactReference>,
    started_at: i64,
}

struct CompletedGeneratedWork {
    run_id: String,
    input: Value,
    summary: Result<GenerateSummary, GenerateFailure>,
    generated_count: u64,
    logical_bytes: u64,
    stream_digest: String,
    transform_node_id: Option<String>,
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_stream_digest: String,
    branch_node_id: Option<String>,
    branch_true_count: u64,
    branch_false_count: u64,
    branch_stream_digest: String,
    merge: Option<MergeProgress>,
    summary_progress: Option<SummaryProgress>,
    artifact: Option<ArtifactReference>,
    cancelled: bool,
    started_at: i64,
    completed_at: i64,
    elapsed_micros: u64,
    backpressure_micros: u64,
    backpressure_events: u64,
    _result_bytes: BytePermit,
}

#[derive(Debug)]
struct GenerationCheckpointState {
    generated_count: u64,
    logical_bytes: u64,
    transform_node_id: Option<String>,
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_stream_digest: String,
    branch_node_id: Option<String>,
    branch_true_count: u64,
    branch_false_count: u64,
    branch_stream_digest: String,
    backpressure_events: u64,
    backpressure_micros: u64,
    checkpointed_at: Instant,
}

fn start_scheduler(context: SchedulerContext) -> Result<JoinHandle<()>, String> {
    thread::Builder::new()
        .name("workflowd-run-scheduler".into())
        .spawn(move || scheduler_loop(context))
        .map_err(|error| format!("cannot start governed Run scheduler: {error}"))
}

fn generation_plan_and_transform(
    plan: &ExecutionPlan,
) -> Result<
    (
        ExecutionPlan,
        Option<(String, Value)>,
        Option<(String, Value)>,
        Option<(String, Value)>,
        Option<(String, Value)>,
    ),
    GenerateFailure,
> {
    let manual = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "manual-trigger")
        .cloned();
    let generate = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "generate-items")
        .cloned();
    let edit = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "edit-fields")
        .cloned();
    let if_node = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "if")
        .cloned();
    let merge_node = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "merge")
        .cloned();
    let summarize_node = plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "summarize")
        .cloned();
    let Some(manual) = manual else {
        return Err(generate_failure(
            "canopy.generate-items.invalid_pinned_plan",
            "The pinned plan is missing Manual Trigger.",
        ));
    };
    let Some(generate) = generate else {
        return Err(generate_failure(
            "canopy.generate-items.invalid_pinned_plan",
            "The pinned plan is missing Generate Items.",
        ));
    };
    let has_manual_to_generate = plan.scheduling_dependencies.iter().any(|dependency| {
        dependency.source_node_id == manual.node_instance_id
            && dependency.source_port_id == "invocation"
            && dependency.target_node_id == generate.node_instance_id
            && dependency.target_port_id == "input"
    });
    if !has_manual_to_generate {
        return Err(generate_failure(
            "canopy.generate-items.invalid_pinned_plan",
            "The pinned plan is missing Manual Trigger to Generate Items.",
        ));
    }
    let edit_is_valid = edit.as_ref().is_none_or(|edit| {
        edit.contract_lock.namespace == "canopy.native"
            && edit.contract_lock.api_version == "v1alpha2"
            && edit.capabilities.is_empty()
            && edit.effects["class"] == "pure"
            && edit.effects["deterministic"] == true
    });
    let if_is_valid = if_node.as_ref().is_none_or(|node| {
        node.contract_lock.namespace == "canopy.native"
            && node.contract_lock.api_version == "v1alpha1"
            && node.capabilities.is_empty()
            && node.effects["class"] == "pure"
            && node.effects["deterministic"] == true
    });
    let has_generate_to_edit = edit.as_ref().is_none_or(|edit| {
        plan.scheduling_dependencies.iter().any(|dependency| {
            dependency.source_node_id == generate.node_instance_id
                && dependency.source_port_id == "items"
                && dependency.target_node_id == edit.node_instance_id
                && dependency.target_port_id == "input"
        })
    });
    let has_generate_to_if = if_node.as_ref().is_none_or(|node| {
        let source_node = edit
            .as_ref()
            .map_or(&generate.node_instance_id, |edit| &edit.node_instance_id);
        let source_port = if edit.is_some() { "item" } else { "items" };
        plan.scheduling_dependencies.iter().any(|dependency| {
            dependency.source_node_id == *source_node
                && dependency.source_port_id == source_port
                && dependency.target_node_id == node.node_instance_id
                && dependency.target_port_id == "input"
        })
    });
    let merge_is_valid = merge_node.as_ref().is_none_or(|node| {
        node.contract_lock.namespace == "canopy.native"
            && node.contract_lock.api_version == "v1alpha1"
            && node.contract_lock.name == "merge"
            && node.capabilities.is_empty()
            && node.effects["class"] == "pure"
            && node.effects["deterministic"] == true
    });
    let has_if_to_merge = merge_node.as_ref().is_none_or(|merge| {
        let Some(if_node) = if_node.as_ref() else {
            return false;
        };
        ["true", "false"].into_iter().all(|port| {
            plan.scheduling_dependencies.iter().any(|dependency| {
                dependency.source_node_id == if_node.node_instance_id
                    && dependency.source_port_id == port
                    && dependency.target_node_id == merge.node_instance_id
                    && dependency.target_port_id == port
            })
        })
    });
    let summarize_is_valid = summarize_node.as_ref().is_none_or(|node| {
        node.contract_lock.namespace == "canopy.native"
            && node.contract_lock.api_version == "v1alpha1"
            && node.contract_lock.name == "summarize"
            && node.activation["shape"] == "barrier_reducer"
            && node.capabilities.is_empty()
            && node.effects["class"] == "pure"
            && node.effects["deterministic"] == true
    });
    let has_merge_to_summarize = summarize_node.as_ref().is_none_or(|summarize| {
        let Some(merge) = merge_node.as_ref() else {
            return false;
        };
        plan.scheduling_dependencies.iter().any(|dependency| {
            dependency.source_node_id == merge.node_instance_id
                && dependency.source_port_id == "items"
                && dependency.target_node_id == summarize.node_instance_id
                && dependency.target_port_id == "items"
        })
    });
    let expected_nodes = 2
        + usize::from(edit.is_some())
        + usize::from(if_node.is_some())
        + usize::from(merge_node.is_some())
        + usize::from(summarize_node.is_some());
    let expected_edges = expected_nodes - 1 + usize::from(merge_node.is_some());
    if plan.nodes.len() != expected_nodes
        || plan.scheduling_dependencies.len() != expected_edges
        || !edit_is_valid
        || !if_is_valid
        || !merge_is_valid
        || !summarize_is_valid
        || !has_generate_to_edit
        || !has_generate_to_if
        || !has_if_to_merge
        || !has_merge_to_summarize
    {
        return Err(generate_failure(
            "canopy.generate-items.invalid_pinned_plan",
            "The pinned plan has unsupported native topology.",
        ));
    }
    let mut generation_plan = plan.clone();
    generation_plan.nodes = vec![manual.clone(), generate.clone()];
    generation_plan.scheduling_dependencies = plan
        .scheduling_dependencies
        .iter()
        .filter(|dependency| {
            dependency.source_node_id == manual.node_instance_id
                && dependency.target_node_id == generate.node_instance_id
        })
        .cloned()
        .collect();
    generation_plan
        .contract_locks
        .retain(|lock| lock.name == "manual-trigger" || lock.name == "generate-items");
    generation_plan.segment_candidates =
        vec![vec![manual.node_instance_id, generate.node_instance_id]];
    Ok((
        generation_plan,
        edit.map(|node| (node.node_instance_id, node.configuration)),
        if_node.map(|node| (node.node_instance_id, node.configuration)),
        merge_node.map(|node| (node.node_instance_id, node.configuration)),
        summarize_node.map(|node| (node.node_instance_id, node.configuration)),
    ))
}

fn generate_failure(code: &str, message: &str) -> GenerateFailure {
    GenerateFailure {
        code: code.into(),
        message: message.into(),
    }
}

fn apply_edit_fields_batch(
    envelopes: &mut [GeneratedEnvelope],
    configuration: Option<&CompiledConfiguration>,
    node_id: Option<&str>,
    artifact: Option<&ArtifactReference>,
    transformed_count: u64,
    transformed_logical_bytes: u64,
    transformed_stream_digest: &str,
) -> Result<AppliedEditFieldsBatch, GenerateFailure> {
    let Some(configuration) = configuration else {
        return Ok(AppliedEditFieldsBatch {
            transformed_count,
            transformed_logical_bytes,
            transformed_batch_logical_bytes: 0,
            transformed_stream_digest: transformed_stream_digest.into(),
        });
    };
    let node_id = node_id.ok_or_else(|| {
        generate_failure(
            "canopy.edit-fields.invalid_runtime_topology",
            "Edit Fields configuration has no node identity.",
        )
    })?;
    let initial_logical_bytes = transformed_logical_bytes;
    let mut next_count = transformed_count;
    let mut next_logical_bytes = transformed_logical_bytes;
    let mut next_stream_digest = transformed_stream_digest.to_owned();
    let mut staged = envelopes.to_vec();
    for envelope in &mut staged {
        let input_digest = digest(&envelope.logical_item).map_err(|message| GenerateFailure {
            code: "canopy.edit-fields.digest_failed".into(),
            message,
        })?;
        let transformed = configuration
            .apply(&envelope.logical_item, envelope.ordinal)
            .map_err(|error| GenerateFailure {
                code: error.code,
                message: error.message,
            })?;
        let logical_output = transformed.item;
        let logical_bytes =
            serde_jcs::to_vec(&logical_output).map_err(|error| GenerateFailure {
                code: "canopy.edit-fields.canonicalization_failed".into(),
                message: error.to_string(),
            })?;
        let next_bytes = next_logical_bytes
            .checked_add(logical_bytes.len() as u64)
            .ok_or_else(|| {
                generate_failure(
                    "canopy.edit-fields.output_bytes_exceeded",
                    "Edit Fields logical output exceeded its bounded byte budget.",
                )
            })?;
        if next_bytes > generate_engine::MAX_LOGICAL_OUTPUT_BYTES {
            return Err(generate_failure(
                "canopy.edit-fields.output_bytes_exceeded",
                "Edit Fields logical output exceeded its bounded byte budget.",
            ));
        }
        next_stream_digest = digest(&json!({
            "schema":"canopy.edit-fields-chain/v1alpha2",
            "previous":next_stream_digest,
            "ordinal":envelope.ordinal,
            "item":logical_output.clone()
        }))
        .map_err(|message| GenerateFailure {
            code: "canopy.edit-fields.digest_failed".into(),
            message,
        })?;
        envelope.logical_item = logical_output.clone();
        let mut physical_output = logical_output;
        if let Some(artifact) = artifact {
            let output = physical_output.as_object_mut().ok_or_else(|| {
                generate_failure(
                    "canopy.edit-fields.output_shape",
                    "Edit Fields output must be a JSON object.",
                )
            })?;
            output.insert("data".into(), json!({"$artifact": artifact}));
        }
        envelope.item = physical_output;
        if let Some(provenance) = envelope.provenance.as_object_mut() {
            provenance.insert("edit_fields_node_instance_id".into(), json!(node_id));
            provenance.insert("input_ordinal".into(), json!(envelope.ordinal));
            provenance.insert("input_digest".into(), json!(input_digest));
        }
        let physical_bytes = serde_jcs::to_vec(envelope).map_err(|error| GenerateFailure {
            code: "canopy.edit-fields.canonicalization_failed".into(),
            message: error.to_string(),
        })?;
        if physical_bytes.len() > generate_engine::MICRO_BATCH_BYTES {
            return Err(generate_failure(
                "canopy.edit-fields.item_bytes_exceeded",
                "One transformed physical Envelope exceeds the micro-batch byte limit.",
            ));
        }
        next_count = next_count.checked_add(1).ok_or_else(|| {
            generate_failure(
                "canopy.edit-fields.output_count_exceeded",
                "Edit Fields output exceeded its bounded item budget.",
            )
        })?;
        next_logical_bytes = next_bytes;
    }
    envelopes.clone_from_slice(&staged);
    Ok(AppliedEditFieldsBatch {
        transformed_count: next_count,
        transformed_logical_bytes: next_logical_bytes,
        transformed_batch_logical_bytes: next_logical_bytes.saturating_sub(initial_logical_bytes),
        transformed_stream_digest: next_stream_digest,
    })
}

struct MergeSpool {
    artifacts: Arc<ArtifactService>,
    run_id: String,
    node_id: String,
    port_id: &'static str,
    reference_kind: &'static str,
    next_segment: u32,
    active: Option<crate::artifact::UploadLease>,
    active_count: u64,
    active_bytes: u64,
    pending: Vec<u8>,
    pending_records: u64,
    references: Vec<ArtifactReference>,
    physical_bytes: u64,
}

impl MergeSpool {
    fn new(
        artifacts: Arc<ArtifactService>,
        run_id: &str,
        node_id: &str,
        port_id: &'static str,
        reference_kind: &'static str,
    ) -> Self {
        Self {
            artifacts,
            run_id: run_id.into(),
            node_id: node_id.into(),
            port_id,
            reference_kind,
            next_segment: 0,
            active: None,
            active_count: 0,
            active_bytes: 0,
            pending: Vec::new(),
            pending_records: 0,
            references: Vec::new(),
            physical_bytes: 0,
        }
    }

    fn append(&mut self, record: &merge::MergeRecord) -> Result<(), merge::MergeError> {
        let mut line = serde_jcs::to_vec(record).map_err(|error| merge::MergeError {
            code: "canopy.merge.canonicalization".into(),
            message: error.to_string(),
        })?;
        line.push(b'\n');
        if line.len() > generate_engine::MICRO_BATCH_BYTES {
            return Err(merge::MergeError {
                code: "canopy.merge.record_too_large".into(),
                message: "One Merge record exceeds the bounded spool record size".into(),
            });
        }
        if self.active.is_none() {
            self.begin_segment()?;
        }
        if !self.pending.is_empty()
            && self.pending.len().saturating_add(line.len()) > generate_engine::MICRO_BATCH_BYTES
        {
            self.flush_pending()?;
        }
        self.pending.extend_from_slice(&line);
        self.pending_records = self.pending_records.saturating_add(1);
        self.physical_bytes = self.physical_bytes.saturating_add(line.len() as u64);
        if self.pending.len() >= generate_engine::MICRO_BATCH_BYTES {
            self.flush_pending()?;
        }
        Ok(())
    }

    fn flush_pending(&mut self) -> Result<(), merge::MergeError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if self.active.is_none() {
            self.begin_segment()?;
        }
        let chunk = std::mem::take(&mut self.pending);
        let records = self.pending_records;
        let bytes = chunk.len() as u64;
        self.pending_records = 0;
        let result = self
            .active
            .as_mut()
            .ok_or_else(|| merge::MergeError {
                code: "canopy.merge.spool_storage".into(),
                message: "Merge spool did not open an Artifact lease".into(),
            })
            .and_then(|lease| {
                self.artifacts
                    .append_upload(lease, &chunk)
                    .map_err(merge_artifact_error)
            });
        if let Err(error) = result {
            if !matches!(error.code.as_str(), "canopy.merge.spool_limit") || self.active_count == 0
            {
                return Err(error);
            }
            self.finalize_active_segment()?;
            self.begin_segment()?;
            let lease = self.active.as_mut().ok_or_else(|| merge::MergeError {
                code: "canopy.merge.spool_storage".into(),
                message: "Merge spool did not open an Artifact lease".into(),
            })?;
            self.artifacts
                .append_upload(lease, &chunk)
                .map_err(merge_artifact_error)?;
        }
        self.active_count = self.active_count.saturating_add(records);
        self.active_bytes = self.active_bytes.saturating_add(bytes);
        Ok(())
    }

    fn begin_segment(&mut self) -> Result<(), merge::MergeError> {
        if self.active.is_some() {
            return Ok(());
        }
        let lease = self
            .artifacts
            .begin_upload()
            .map_err(merge_artifact_error)?;
        self.active = Some(lease);
        self.active_count = 0;
        self.active_bytes = 0;
        Ok(())
    }

    fn finalize_active_segment(&mut self) -> Result<(), merge::MergeError> {
        let Some(lease) = self.active.take() else {
            return Ok(());
        };
        if self.active_count == 0 {
            self.artifacts.abandon_upload(lease);
            return Ok(());
        }
        let reference_id = format!(
            "run:{}:merge:{}:{}:segment:{}",
            self.run_id, self.node_id, self.port_id, self.next_segment
        );
        let view = self
            .artifacts
            .finalize_upload(
                lease,
                "application/x-ndjson",
                &reference_id,
                self.reference_kind,
                self.active_count,
            )
            .map_err(merge_artifact_error)?;
        self.references.push(view.reference);
        self.next_segment = self.next_segment.saturating_add(1);
        self.active_count = 0;
        self.active_bytes = 0;
        Ok(())
    }

    fn finish_segment(&mut self) -> Result<(), merge::MergeError> {
        self.flush_pending()?;
        self.finalize_active_segment()
    }

    fn finish(&mut self) -> Result<(), merge::MergeError> {
        self.finish_segment()
    }

    fn abandon(&mut self) {
        if let Some(lease) = self.active.take() {
            self.artifacts.abandon_upload(lease);
        }
        for segment in 0..self.references.len() {
            let reference_id = format!(
                "run:{}:merge:{}:{}:segment:{}",
                self.run_id, self.node_id, self.port_id, segment
            );
            let _ = self.artifacts.release_reference(&reference_id);
        }
        self.references.clear();
        self.next_segment = 0;
        self.active_count = 0;
        self.active_bytes = 0;
        self.pending.clear();
        self.pending_records = 0;
    }

    fn references(&self) -> Vec<ArtifactReference> {
        self.references.clone()
    }
}

fn merge_artifact_error(error: ArtifactError) -> merge::MergeError {
    let code = match error {
        ArtifactError::Storage(_) => "canopy.merge.spool_storage",
        ArtifactError::TooLarge => "canopy.merge.spool_limit",
        ArtifactError::Integrity(_) => "canopy.merge.spool_integrity",
        ArtifactError::NotAuthorized => "canopy.merge.spool_unauthorized",
        ArtifactError::Invalid(_) => "canopy.merge.spool_invalid",
    };
    merge::MergeError {
        code: code.into(),
        message: error.to_string(),
    }
}

struct MergeArtifactIterator {
    artifacts: Arc<ArtifactService>,
    references: Vec<ArtifactReference>,
    next_reference: usize,
    stream: Option<crate::artifact::ArtifactContentStream>,
    buffer: Vec<u8>,
    finished: bool,
}

impl MergeArtifactIterator {
    fn new(artifacts: Arc<ArtifactService>, references: Vec<ArtifactReference>) -> Self {
        Self {
            artifacts,
            references,
            next_reference: 0,
            stream: None,
            buffer: Vec::new(),
            finished: false,
        }
    }

    fn error(error: impl Into<String>) -> merge::MergeError {
        merge::MergeError {
            code: "canopy.merge.spool_read".into(),
            message: error.into(),
        }
    }
}

impl Iterator for MergeArtifactIterator {
    type Item = Result<merge::MergeRecord, merge::MergeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        loop {
            if let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = self.buffer.drain(..=newline).collect();
                let line = &line[..line.len().saturating_sub(1)];
                return Some(
                    serde_json::from_slice::<Value>(line)
                        .map_err(|error| {
                            Self::error(format!("Merge record JSON is invalid: {error}"))
                        })
                        .and_then(merge::record_from_value),
                );
            }
            if self.buffer.len() > generate_engine::MICRO_BATCH_BYTES {
                self.finished = true;
                return Some(Err(Self::error(
                    "Merge spool record exceeded the bounded read buffer",
                )));
            }
            if self.stream.is_none() {
                if self.next_reference >= self.references.len() {
                    self.finished = true;
                    if self.buffer.is_empty() {
                        return None;
                    }
                    let line = std::mem::take(&mut self.buffer);
                    return Some(
                        serde_json::from_slice::<Value>(&line)
                            .map_err(|error| {
                                Self::error(format!("Merge final record JSON is invalid: {error}"))
                            })
                            .and_then(merge::record_from_value),
                    );
                }
                let reference = &self.references[self.next_reference];
                self.next_reference += 1;
                match self.artifacts.stream_content(&reference.artifact_id) {
                    Ok((_, stream)) => self.stream = Some(stream),
                    Err(error) => {
                        self.finished = true;
                        return Some(Err(merge_artifact_error(error)));
                    }
                }
            }
            let stream = self.stream.as_mut().expect("Merge stream exists");
            match stream.next_chunk() {
                Ok(Some(chunk)) => self.buffer.extend_from_slice(&chunk),
                Ok(None) => self.stream = None,
                Err(error) => {
                    self.finished = true;
                    return Some(Err(merge_artifact_error(error)));
                }
            }
        }
    }
}

fn build_merge_progress(
    node_id: &str,
    summary: &merge::MergeSummary,
    physical_spool_bytes: u64,
    true_segments: Vec<ArtifactReference>,
    false_segments: Vec<ArtifactReference>,
    output_segments: Vec<ArtifactReference>,
) -> MergeProgress {
    MergeProgress {
        node_instance_id: node_id.into(),
        mode: summary.mode.clone(),
        true_count: summary.true_count,
        false_count: summary.false_count,
        output_count: summary.output_count,
        logical_bytes: summary.logical_bytes,
        stream_digest: summary.stream_digest.clone(),
        physical_spool_bytes,
        true_segments,
        false_segments,
        output_segments,
    }
}

fn build_summary_progress(node_id: &str, summary: &summarize::Summary) -> SummaryProgress {
    SummaryProgress {
        node_instance_id: node_id.into(),
        operation: summary.operation.clone(),
        total_count: summary.total_count,
        true_count: summary.true_count,
        false_count: summary.false_count,
        logical_bytes: summary.logical_bytes,
        output_digest: summary.output_digest.clone(),
        first_ordinal: summary.first_ordinal,
        last_ordinal: summary.last_ordinal,
    }
}

fn start_executor(
    ready: Receiver<QueuedWork>,
    results: SyncSender<ExecutorTerminal>,
    envelopes: SyncSender<GeneratedBatchEvent>,
    artifacts: Arc<ArtifactService>,
) -> Result<JoinHandle<()>, String> {
    thread::Builder::new()
        .name("workflowd-native-executor".into())
        .spawn(move || {
            while let Ok(queued) = ready.recv() {
                let WorkItem {
                    candidate,
                    cancellation,
                    _result_bytes,
                } = queued.work;
                let started_at = now_millis();
                let started = Instant::now();
                if candidate.plan.nodes.len() == 1 {
                    let result = run_engine::execute_manual(ManualActivationInput {
                        run_id: candidate.run_id.clone(),
                        revision_id: candidate.revision_id,
                        revision_digest: candidate.revision_digest,
                        plan_digest: candidate.plan_digest,
                        plan: candidate.plan,
                        captured_invocation: candidate.captured_invocation,
                        cancellation_observed: cancellation.load(Ordering::Acquire),
                    });
                    let completed_at = now_millis();
                    let elapsed_micros = started.elapsed().as_micros().min(i64::MAX as u128) as u64;
                    if results
                        .send(ExecutorTerminal::Manual(CompletedWork {
                            run_id: candidate.run_id,
                            result,
                            started_at,
                            completed_at,
                            elapsed_micros,
                            _result_bytes,
                        }))
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }

                let run_id = candidate.run_id.clone();
                let input = candidate.captured_invocation.clone();
                let artifact = candidate.artifact.clone();
                let merge_artifacts = artifacts.clone();
                let edit_fields_candidate = candidate.edit_fields;
                let if_candidate = candidate.if_node;
                let merge_candidate = candidate.merge;
                let summarize_candidate = candidate.summarize;
                let (
                    generation_plan,
                    edit_fields_definition,
                    if_definition,
                    plan_merge_definition,
                    plan_summarize_definition,
                ) = match generation_plan_and_transform(&candidate.plan) {
                        Ok(value) => value,
                        Err(error) => {
                            let completed_at = now_millis();
                            let elapsed_micros =
                                started.elapsed().as_micros().min(i64::MAX as u128) as u64;
                            if results
                                .send(ExecutorTerminal::Generated(CompletedGeneratedWork {
                                    run_id,
                                    input,
                                    summary: Err(error),
                                    generated_count: 0,
                                    logical_bytes: 0,
                                    stream_digest: "genesis".into(),
                                    transform_node_id: None,
                                    transformed_count: 0,
                                    transformed_logical_bytes: 0,
                                    transformed_stream_digest: "genesis".into(),
                                    branch_node_id: None,
                                    branch_true_count: 0,
                                    branch_false_count: 0,
                                    branch_stream_digest: "genesis".into(),
                                    merge: None,
                                    summary_progress: None,
                                    artifact,
                                    cancelled: false,
                                    started_at,
                                    completed_at,
                                    elapsed_micros,
                                    backpressure_micros: 0,
                                    backpressure_events: 0,
                                    _result_bytes,
                                }))
                                .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                    };
                let merge_definition = merge_candidate
                    .map(|candidate| (candidate.node_id, candidate.configuration))
                    .or(plan_merge_definition);
                let summarize_definition = summarize_candidate
                    .map(|candidate| (candidate.node_id, candidate.configuration))
                    .or(plan_summarize_definition);
                let (transform_node_id, transform_configuration) = match edit_fields_definition {
                    Some((node_id, configuration)) => {
                        let compiled = match edit_fields::compile_configuration(&configuration) {
                            Ok(compiled) => compiled,
                            Err(error) => {
                                let completed_at = now_millis();
                                let elapsed_micros =
                                    started.elapsed().as_micros().min(i64::MAX as u128) as u64;
                                if results
                                    .send(ExecutorTerminal::Generated(CompletedGeneratedWork {
                                        run_id,
                                        input,
                                        summary: Err(GenerateFailure {
                                            code: error.code,
                                            message: error.message,
                                        }),
                                        generated_count: 0,
                                        logical_bytes: 0,
                                        stream_digest: "genesis".into(),
                                        transform_node_id: Some(node_id),
                                        transformed_count: 0,
                                        transformed_logical_bytes: 0,
                                        transformed_stream_digest: "genesis".into(),
                                        branch_node_id: None,
                                        branch_true_count: 0,
                                        branch_false_count: 0,
                                        branch_stream_digest: "genesis".into(),
                                        merge: None,
                                        summary_progress: None,
                                        artifact,
                                        cancelled: false,
                                        started_at,
                                        completed_at,
                                        elapsed_micros,
                                        backpressure_micros: 0,
                                        backpressure_events: 0,
                                        _result_bytes,
                                    }))
                                    .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        };
                        (Some(node_id), Some(compiled))
                    }
                    None => (None, None),
                };
                let (branch_node_id, branch_configuration) = match if_definition {
                    Some((node_id, configuration)) => (
                        Some(node_id),
                        Some(
                            if_node::compile_configuration(&configuration)
                                .map_err(|error| generate_failure(&error.code, &error.message)),
                        ),
                    ),
                    None => (None, None),
                };
                let (merge_node_id, merge_configuration) = match merge_definition {
                    Some((node_id, configuration)) => (
                        Some(node_id),
                        Some(
                            merge::compile_configuration(&configuration)
                                .map_err(|error| generate_failure(&error.code, &error.message)),
                        ),
                    ),
                    None => (None, None),
                };
                let (summarize_node_id, summarize_configuration) = match summarize_definition {
                    Some((node_id, configuration)) => (
                        Some(node_id),
                        Some(
                            summarize::compile_configuration(&configuration)
                                .map_err(|error| generate_failure(&error.code, &error.message)),
                        ),
                    ),
                    None => (None, None),
                };
                let mut generated_count = 0_u64;
                let mut logical_bytes = 0_u64;
                let mut stream_digest = "genesis".to_owned();
                let mut transformed_count = edit_fields_candidate
                    .as_ref()
                    .map_or(0, |candidate| candidate.transformed_count);
                let mut transformed_logical_bytes = edit_fields_candidate
                    .as_ref()
                    .map_or(0, |candidate| candidate.transformed_logical_bytes);
                let mut transformed_stream_digest = edit_fields_candidate.as_ref().map_or_else(
                    || "genesis".into(),
                    |candidate| candidate.transformed_stream_digest.clone(),
                );
                let mut branch_true_count = if_candidate
                    .as_ref()
                    .map_or(0, |candidate| candidate.true_count);
                let mut branch_false_count = if_candidate
                    .as_ref()
                    .map_or(0, |candidate| candidate.false_count);
                let mut branch_stream_digest = if_candidate.as_ref().map_or_else(
                    || "genesis".into(),
                    |candidate| candidate.stream_digest.clone(),
                );
                let mut backpressure_micros = 0_u64;
                let mut backpressure_events = 0_u64;
                let mut cancelled = false;
                let physical_data = candidate
                    .artifact
                    .as_ref()
                    .map(|reference| json!({"$artifact": reference}));
                let mut merge_true_spool = merge_node_id.as_ref().map(|node_id| {
                    MergeSpool::new(
                        merge_artifacts.clone(),
                        &run_id,
                        node_id,
                        "true",
                        "run_merge_true_segment",
                    )
                });
                let mut merge_false_spool = merge_node_id.as_ref().map(|node_id| {
                    MergeSpool::new(
                        merge_artifacts.clone(),
                        &run_id,
                        node_id,
                        "false",
                        "run_merge_false_segment",
                    )
                });
                let mut merge_output_spool = merge_node_id.as_ref().map(|node_id| {
                    MergeSpool::new(
                        merge_artifacts.clone(),
                        &run_id,
                        node_id,
                        "output",
                        "run_merge_output_segment",
                    )
                });
                let branch_configuration_error = branch_configuration
                    .as_ref()
                    .and_then(|configuration| configuration.as_ref().err().cloned());
                let merge_configuration_error = merge_configuration
                    .as_ref()
                    .and_then(|configuration| configuration.as_ref().err().cloned());
                let summarize_configuration_error = summarize_configuration
                    .as_ref()
                    .and_then(|configuration| configuration.as_ref().err().cloned());
                let merge_cleanup_error = merge_node_id.as_ref().and_then(|node_id| {
                    let prefix = format!("run:{}:merge:{}:", run_id, node_id);
                    merge_artifacts
                        .release_reference_prefix(&prefix)
                        .err()
                        .map(|error| GenerateFailure {
                            code: "canopy.merge.spool_cleanup".into(),
                            message: error.to_string(),
                        })
                });
                let configuration_error = branch_configuration_error
                    .or(merge_configuration_error)
                    .or(summarize_configuration_error)
                    .or(merge_cleanup_error);
                let mut merge_progress = None;
                let mut summary_progress = None;
                let mut summary = match configuration_error {
                    Some(error) => Err(error),
                    None => match GenerateSession::start(GenerateStart {
                        run_id: candidate.run_id,
                        revision_digest: candidate.revision_digest,
                        plan_digest: candidate.plan_digest,
                        plan: &generation_plan,
                        input: candidate.captured_invocation,
                        logical_data_override: candidate.logical_data_override,
                        physical_data,
                        resume: candidate.generate_resume,
                    }) {
                        Ok(mut session) => loop {
                            if cancellation.load(Ordering::Acquire) {
                                cancelled = true;
                                break Err(GenerateFailure {
                                    code: "canopy.generate-items.cancelled".into(),
                                    message: "Generation observed cooperative cancellation.".into(),
                                });
                            }
                            match session.next_batch() {
                                Ok(Some(mut batch)) => {
                                    let progress = session.progress();
                                    generated_count = progress.0;
                                    logical_bytes = progress.1;
                                    stream_digest = progress.2.to_owned();
                                    let transformed = match apply_edit_fields_batch(
                                        &mut batch,
                                        transform_configuration.as_ref(),
                                        transform_node_id.as_deref(),
                                        artifact.as_ref(),
                                        transformed_count,
                                        transformed_logical_bytes,
                                        &transformed_stream_digest,
                                    ) {
                                        Ok(progress) => progress,
                                        Err(error) => break Err(error),
                                    };
                                    let branch_previous_stream_digest =
                                        branch_stream_digest.clone();
                                    let branch = match (
                                        branch_node_id.as_deref(),
                                        branch_configuration.as_ref(),
                                    ) {
                                        (Some(node_id), Some(Ok(configuration))) => configuration
                                            .route_batch(&mut batch, node_id, &branch_stream_digest)
                                            .map_err(|error| {
                                                generate_failure(&error.code, &error.message)
                                            }),
                                        (Some(_), Some(Err(error))) => Err(error.clone()),
                                        (None, None) => Ok(if_node::BranchBatch {
                                            true_count: 0,
                                            false_count: 0,
                                            stream_digest: branch_stream_digest.clone(),
                                        }),
                                        _ => Err(generate_failure(
                                            "canopy.if.invalid_runtime_topology",
                                            "If runtime configuration and node identity disagree.",
                                        )),
                                    };
                                    let branch = match branch {
                                        Ok(branch) => branch,
                                        Err(error) => break Err(error),
                                    };
                                    branch_true_count =
                                        branch_true_count.saturating_add(branch.true_count);
                                    branch_false_count =
                                        branch_false_count.saturating_add(branch.false_count);
                                    branch_stream_digest = branch.stream_digest;
                                    if merge_node_id.is_some() {
                                        let spool_result: Result<(), GenerateFailure> = (|| {
                                            for envelope in &batch {
                                                let port = envelope
                                                    .provenance
                                                    .get("if_output_port")
                                                    .and_then(Value::as_str)
                                                    .ok_or_else(|| generate_failure(
                                                        "canopy.merge.missing_route_provenance",
                                                        "Merge input item is missing the If output port.",
                                                    ))?;
                                                let logical_bytes = serde_jcs::to_vec(&envelope.logical_item)
                                                    .map_err(|error| {
                                                        generate_failure(
                                                            "canopy.merge.canonicalization",
                                                            &error.to_string(),
                                                        )
                                                    })?
                                                    .len() as u64;
                                                let record = merge::MergeRecord {
                                                    ordinal: envelope.ordinal,
                                                    item: envelope.item.clone(),
                                                    logical_item: envelope.logical_item.clone(),
                                                    logical_bytes,
                                                    provenance: envelope.provenance.clone(),
                                                };
                                                let result = match port {
                                                    "true" => merge_true_spool
                                                        .as_mut()
                                                        .ok_or_else(|| generate_failure(
                                                            "canopy.merge.invalid_runtime_topology",
                                                            "Merge true input spool is unavailable.",
                                                        ))?
                                                        .append(&record),
                                                    "false" => merge_false_spool
                                                        .as_mut()
                                                        .ok_or_else(|| generate_failure(
                                                            "canopy.merge.invalid_runtime_topology",
                                                            "Merge false input spool is unavailable.",
                                                        ))?
                                                        .append(&record),
                                                    _ => Err(merge::MergeError {
                                                        code: "canopy.merge.invalid_route_provenance".into(),
                                                        message: "If emitted an unknown output port.".into(),
                                                    }),
                                                };
                                                result.map_err(|error| {
                                                    generate_failure(&error.code, &error.message)
                                                })?;
                                            }
                                            Ok(())
                                        })();
                                        if let Err(error) = spool_result {
                                            break Err(error);
                                        }
                                    }
                                    let sent = Instant::now();
                                    if envelopes
                                        .send(GeneratedBatchEvent {
                                            run_id: run_id.clone(),
                                            envelopes: batch,
                                            generated_count,
                                            logical_bytes,
                                            stream_digest: stream_digest.clone(),
                                            transform_node_id: transform_node_id.clone(),
                                            transformed_count: transformed.transformed_count,
                                            transformed_logical_bytes: transformed
                                                .transformed_logical_bytes,
                                            transformed_batch_logical_bytes: transformed
                                                .transformed_batch_logical_bytes,
                                            transformed_stream_digest: transformed
                                                .transformed_stream_digest
                                                .clone(),
                                            branch_node_id: branch_node_id.clone(),
                                            branch_true_count,
                                            branch_false_count,
                                            branch_stream_digest: branch_stream_digest.clone(),
                                            branch_batch_true_count: branch.true_count,
                                            branch_batch_false_count: branch.false_count,
                                            branch_previous_stream_digest,
                                            backpressure_micros,
                                            artifact: artifact.clone(),
                                            started_at,
                                        })
                                        .is_err()
                                    {
                                        return;
                                    }
                                    transformed_count = transformed.transformed_count;
                                    transformed_logical_bytes =
                                        transformed.transformed_logical_bytes;
                                    transformed_stream_digest =
                                        transformed.transformed_stream_digest;
                                    let blocked_micros =
                                        sent.elapsed().as_micros().min(u64::MAX as u128) as u64;
                                    backpressure_micros =
                                        backpressure_micros.saturating_add(blocked_micros);
                                    if blocked_micros >= 1_000 {
                                        backpressure_events = backpressure_events.saturating_add(1);
                                    }
                                }
                                Ok(None) => break session.finish(),
                                Err(error) => break Err(error),
                            }
                        },
                        Err(error) => Err(error),
                    },
                };
                if summary.is_ok() {
                    let merge_result: Result<Option<MergeProgress>, GenerateFailure> = (|| {
                        let (Some(node_id), Some(Ok(configuration)), Some(true_spool), Some(false_spool), Some(output_spool)) = (
                            merge_node_id.as_deref(),
                            merge_configuration.as_ref(),
                            merge_true_spool.as_mut(),
                            merge_false_spool.as_mut(),
                            merge_output_spool.as_mut(),
                        ) else {
                            return Ok(None);
                        };
                        true_spool.finish().map_err(|error| {
                            generate_failure(&error.code, &error.message)
                        })?;
                        false_spool.finish().map_err(|error| {
                            generate_failure(&error.code, &error.message)
                        })?;
                        let true_segments = true_spool.references();
                        let false_segments = false_spool.references();
                        let true_reader = MergeArtifactIterator::new(
                            merge_artifacts.clone(),
                            true_segments.clone(),
                        );
                        let false_reader = MergeArtifactIterator::new(
                            merge_artifacts.clone(),
                            false_segments.clone(),
                        );
                        let summary = configuration
                            .merge_ordered(true_reader, false_reader, |record, _port| {
                                output_spool
                                    .append(record)
                                    .map_err(|error| merge::MergeError {
                                        code: error.code,
                                        message: error.message,
                                    })
                            })
                            .map_err(|error| generate_failure(&error.code, &error.message))?;
                        output_spool.finish().map_err(|error| {
                            generate_failure(&error.code, &error.message)
                        })?;
                        let output_segments = output_spool.references();
                        Ok(Some(build_merge_progress(
                            node_id,
                            &summary,
                            true_spool
                                .physical_bytes
                                .saturating_add(false_spool.physical_bytes)
                                .saturating_add(output_spool.physical_bytes),
                            true_segments,
                            false_segments,
                            output_segments,
                        )))
                    })();
                    match merge_result {
                        Ok(progress) => merge_progress = progress,
                        Err(error) => summary = Err(error),
                    }
                    if summary.is_ok() {
                        if let (
                            Some(node_id),
                            Some(Ok(configuration)),
                            Some(merge_progress),
                        ) = (
                            summarize_node_id.as_deref(),
                            summarize_configuration.as_ref(),
                            merge_progress.as_ref(),
                        ) {
                            let records = MergeArtifactIterator::new(
                                merge_artifacts.clone(),
                                merge_progress.output_segments.clone(),
                            )
                            .map(|record| {
                                record
                                    .map(|record| summarize::SummaryRecord {
                                        ordinal: record.ordinal,
                                        input_port: record
                                            .provenance
                                            .get("if_output_port")
                                            .and_then(Value::as_str)
                                            .unwrap_or("unknown")
                                            .into(),
                                        logical_item: record.logical_item,
                                        logical_bytes: record.logical_bytes,
                                        provenance: record.provenance,
                                    })
                                    .map_err(|error| summarize::SummarizeError {
                                        code: error.code,
                                        message: error.message,
                                    })
                            });
                            match configuration.summarize(records) {
                                Ok(reduced) => {
                                    summary_progress = Some(build_summary_progress(node_id, &reduced));
                                }
                                Err(error) => {
                                    summary = Err(generate_failure(&error.code, &error.message));
                                }
                            }
                        }
                    }
                }
                if summary.is_err() {
                    if let Some(spool) = merge_true_spool.as_mut() {
                        spool.abandon();
                    }
                    if let Some(spool) = merge_false_spool.as_mut() {
                        spool.abandon();
                    }
                    if let Some(spool) = merge_output_spool.as_mut() {
                        spool.abandon();
                    }
                }
                let completed_at = now_millis();
                let elapsed_micros = started.elapsed().as_micros().min(i64::MAX as u128) as u64;
                if results
                    .send(ExecutorTerminal::Generated(CompletedGeneratedWork {
                        run_id,
                        input,
                        summary,
                        generated_count,
                        logical_bytes,
                        stream_digest,
                        transform_node_id,
                        transformed_count,
                        transformed_logical_bytes,
                        transformed_stream_digest,
                        branch_node_id,
                        branch_true_count,
                        branch_false_count,
                        branch_stream_digest,
                        merge: merge_progress,
                        summary_progress,
                        artifact,
                        cancelled,
                        started_at,
                        completed_at,
                        elapsed_micros,
                        backpressure_micros,
                        backpressure_events,
                        _result_bytes,
                    }))
                    .is_err()
                {
                    break;
                }
            }
        })
        .map_err(|error| format!("cannot start native Run executor: {error}"))
}

fn scheduler_loop(context: SchedulerContext) {
    let connection = match connect(&context.database) {
        Ok(connection) => connection,
        Err(reason) => {
            error!(event = "run_scheduler_database_failed", reason);
            return;
        }
    };
    let mut active = HashSet::new();
    let mut stopping = false;
    let mut scan_requested = false;
    let mut scan_after = Instant::now();
    let mut generation_checkpoints: HashMap<String, GenerationCheckpointState> = HashMap::new();
    while !stopping {
        loop {
            match context.wake.try_recv() {
                Ok(SchedulerSignal::Stop) => {
                    stopping = true;
                    break;
                }
                Ok(SchedulerSignal::Wake) => {
                    if !scan_requested {
                        scan_requested = true;
                        scan_after = Instant::now() + MINIMUM_QUEUE_VISIBILITY;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    stopping = true;
                    break;
                }
            }
        }
        if stopping {
            break;
        }

        while let Ok(batch) = context.envelopes.try_recv() {
            handle_generated_batch(&context, &mut generation_checkpoints, batch);
        }
        while let Ok(completed) = context.results.try_recv() {
            finish_executor_terminal(
                &context,
                &mut active,
                &mut generation_checkpoints,
                completed,
            );
            scan_requested = true;
            scan_after = Instant::now();
        }

        let mut dispatched = false;
        if scan_requested && Instant::now() >= scan_after {
            while active.len() < MAX_HOT_RUNS {
                let mut candidate = match next_candidate(&connection, &active) {
                    Ok(Some(candidate)) => candidate,
                    Ok(None) => {
                        scan_requested = false;
                        break;
                    }
                    Err(reason) => {
                        error!(event = "run_scheduler_read_failed", reason = ?reason);
                        scan_requested = false;
                        break;
                    }
                };
                let cancellation = Arc::new(AtomicBool::new(false));
                if let Ok(mut controls) = context.controls.lock() {
                    controls.insert(candidate.run_id.clone(), cancellation.clone());
                } else {
                    break;
                }
                match run_state(&connection, &candidate.run_id) {
                    Ok(state) if state == "queued" => {}
                    _ => {
                        if let Ok(mut controls) = context.controls.lock() {
                            controls.remove(&candidate.run_id);
                        }
                        continue;
                    }
                }
                if let Err(reason) = prepare_candidate_artifact(&context.artifacts, &mut candidate)
                {
                    error!(event = "run_artifact_preparation_failed", run_id = candidate.run_id, reason = ?reason);
                    if let Ok(mut controls) = context.controls.lock() {
                        controls.remove(&candidate.run_id);
                    }
                    let suspended = matches!(&reason, RunError::Storage(_));
                    match context.writer.call(
                        WriterOperation::PreparationFailed {
                            run_id: candidate.run_id.clone(),
                            reason: format!("{reason:?}"),
                            suspended,
                        },
                        8 * 1024,
                    ) {
                        Ok(WriterReply::Run(run)) => context.live.emit(
                            &run.run_id,
                            run.durable.checkpoint_sequence,
                            if run.durable.terminal {
                                "terminal"
                            } else {
                                "checkpoint"
                            },
                            stream_payload(
                                &run,
                                "durable",
                                &run.durable.state,
                                run.durable.terminal,
                            ),
                            true,
                        ),
                        Ok(_) => error!(event = "run_preparation_wrong_writer_reply"),
                        Err(error) => {
                            error!(event = "run_preparation_checkpoint_failed", error = ?error)
                        }
                    }
                    continue;
                }
                let ready_weight = candidate_weight(&candidate);
                let Some(ready_permit) = context.ready_budget.reserve(ready_weight) else {
                    if let Ok(mut controls) = context.controls.lock() {
                        controls.remove(&candidate.run_id);
                    }
                    warn!(event = "ready_queue_byte_pressure");
                    break;
                };
                let result_reservation = 32 * 1024;
                let Some(result_permit) = context.result_budget.reserve(result_reservation) else {
                    if let Ok(mut controls) = context.controls.lock() {
                        controls.remove(&candidate.run_id);
                    }
                    warn!(event = "result_queue_byte_pressure");
                    break;
                };
                let run_id = candidate.run_id.clone();
                let checkpoint = candidate.checkpoint_sequence;
                match context.ready.try_send(QueuedWork {
                    work: WorkItem {
                        candidate,
                        cancellation,
                        _result_bytes: result_permit,
                    },
                    _ready_bytes: ready_permit,
                }) {
                    Ok(()) => {
                        active.insert(run_id.clone());
                        context.live.emit(
                            &run_id,
                            checkpoint,
                            "live",
                            json!({
                                "run_id": run_id,
                                "durability": "speculative",
                                "state": "running",
                                "durable_checkpoint_sequence": checkpoint,
                                "terminal": false
                            }),
                            false,
                        );
                        dispatched = true;
                    }
                    Err(TrySendError::Full(_)) => {
                        if let Ok(mut controls) = context.controls.lock() {
                            controls.remove(&run_id);
                        }
                        break;
                    }
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
        }

        if !active.is_empty() {
            match context.results.recv_timeout(SCHEDULER_TICK) {
                Ok(completed) => {
                    finish_executor_terminal(
                        &context,
                        &mut active,
                        &mut generation_checkpoints,
                        completed,
                    );
                    scan_requested = true;
                    scan_after = Instant::now();
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else if !dispatched {
            let signal = if scan_requested {
                let remaining = scan_after
                    .saturating_duration_since(Instant::now())
                    .max(Duration::from_millis(1));
                match context.wake.recv_timeout(remaining) {
                    Ok(signal) => Some(signal),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match context.wake.recv() {
                    Ok(signal) => Some(signal),
                    Err(_) => break,
                }
            };
            match signal {
                Some(SchedulerSignal::Stop) => break,
                Some(SchedulerSignal::Wake) => {
                    if !scan_requested {
                        scan_requested = true;
                        scan_after = Instant::now() + MINIMUM_QUEUE_VISIBILITY;
                    }
                }
                None => {}
            }
        }
    }
}

fn handle_generated_batch(
    context: &SchedulerContext,
    checkpoints: &mut HashMap<String, GenerationCheckpointState>,
    batch: GeneratedBatchEvent,
) {
    let Some(first_ordinal) = batch.envelopes.first().map(|envelope| envelope.ordinal) else {
        return;
    };
    let last_ordinal = batch
        .envelopes
        .last()
        .map(|envelope| envelope.ordinal)
        .unwrap_or(first_ordinal);
    debug_assert!(batch
        .envelopes
        .windows(2)
        .all(|window| window[1].ordinal == window[0].ordinal + 1));

    let state = checkpoints.entry(batch.run_id.clone()).or_insert_with(|| {
        let batch_logical_bytes = batch.envelopes.iter().fold(0_u64, |total, envelope| {
            total.saturating_add(envelope.logical_bytes)
        });
        GenerationCheckpointState {
            generated_count: first_ordinal,
            logical_bytes: batch.logical_bytes.saturating_sub(batch_logical_bytes),
            transform_node_id: batch.transform_node_id.clone(),
            transformed_count: batch
                .transformed_count
                .saturating_sub(batch.envelopes.len() as u64),
            transformed_logical_bytes: batch
                .transformed_logical_bytes
                .saturating_sub(batch.transformed_batch_logical_bytes),
            transformed_stream_digest: if batch.transformed_count == 0 {
                "genesis".into()
            } else {
                // The durable transformed cursor is restored from SQLite before the
                // next checkpoint. This value is only a conservative live baseline.
                "genesis".into()
            },
            branch_node_id: batch.branch_node_id.clone(),
            branch_true_count: batch
                .branch_true_count
                .saturating_sub(batch.branch_batch_true_count),
            branch_false_count: batch
                .branch_false_count
                .saturating_sub(batch.branch_batch_false_count),
            branch_stream_digest: batch.branch_previous_stream_digest.clone(),
            backpressure_events: 0,
            backpressure_micros: 0,
            checkpointed_at: Instant::now(),
        }
    });
    let observed_backpressure = batch.backpressure_micros > state.backpressure_micros;
    if observed_backpressure {
        state.backpressure_events = state.backpressure_events.saturating_add(1);
    }
    state.backpressure_micros = batch.backpressure_micros;

    let count_due = batch.generated_count.saturating_sub(state.generated_count)
        >= CHECKPOINT_MAX_OUTCOMES as u64;
    let bytes_due =
        batch.logical_bytes.saturating_sub(state.logical_bytes) >= CHECKPOINT_MAX_BYTES as u64;
    let time_due =
        state.checkpointed_at.elapsed() >= Duration::from_millis(CHECKPOINT_MAX_LATENCY_MILLIS);
    context.live.emit(
        &batch.run_id,
        0,
        "generation-progress",
        json!({
            "run_id": batch.run_id,
            "durability": "speculative",
            "generated_count": batch.generated_count,
            "logical_bytes": batch.logical_bytes,
            "stream_digest": batch.stream_digest,
            "transform": batch.transform_node_id.as_ref().map(|node_id| json!({
                "node_instance_id": node_id,
                "transformed_count": batch.transformed_count,
                "logical_bytes": batch.transformed_logical_bytes,
                "stream_digest": batch.transformed_stream_digest
            })),
            "branch": batch.branch_node_id.as_ref().map(|node_id| json!({
                "node_instance_id": node_id,
                "true_count": batch.branch_true_count,
                "false_count": batch.branch_false_count,
                "stream_digest": batch.branch_stream_digest
            })),
            "first_ordinal": first_ordinal,
            "last_ordinal": last_ordinal,
            "batch_count": batch.envelopes.len(),
            "queue_capacity_items": ENVELOPE_QUEUE_COUNT,
            "queue_capacity_bytes": ENVELOPE_QUEUE_BYTES,
            "artifact": batch.artifact,
            "backpressure_observed": observed_backpressure,
            "terminal": false
        }),
        false,
    );
    if !(count_due || bytes_due || time_due) {
        return;
    }

    let progress = GeneratedProgressCommit {
        run_id: batch.run_id.clone(),
        generated_count: batch.generated_count,
        logical_bytes: batch.logical_bytes,
        stream_digest: batch.stream_digest,
        transform_node_id: batch.transform_node_id.clone(),
        transformed_count: batch.transformed_count,
        transformed_logical_bytes: batch.transformed_logical_bytes,
        transformed_stream_digest: batch.transformed_stream_digest.clone(),
        branch_node_id: batch.branch_node_id.clone(),
        branch_true_count: batch.branch_true_count,
        branch_false_count: batch.branch_false_count,
        branch_stream_digest: batch.branch_stream_digest.clone(),
        backpressure_events: state.backpressure_events,
        backpressure_micros: batch.backpressure_micros,
        first_ordinal: state.generated_count,
        last_ordinal: batch.generated_count.saturating_sub(1),
        artifact: batch.artifact,
        started_at: batch.started_at,
    };
    let weight = 8 * 1024;
    match context
        .writer
        .call(WriterOperation::GenerationProgress(progress), weight)
    {
        Ok(WriterReply::Run(run)) => {
            state.generated_count = batch.generated_count;
            state.logical_bytes = batch.logical_bytes;
            state.transform_node_id = batch.transform_node_id.clone();
            state.transformed_count = batch.transformed_count;
            state.transformed_logical_bytes = batch.transformed_logical_bytes;
            state.transformed_stream_digest = batch.transformed_stream_digest.clone();
            state.branch_node_id = batch.branch_node_id.clone();
            state.branch_true_count = batch.branch_true_count;
            state.branch_false_count = batch.branch_false_count;
            state.branch_stream_digest = batch.branch_stream_digest.clone();
            state.checkpointed_at = Instant::now();
            context.live.emit(
                &run.run_id,
                run.durable.checkpoint_sequence,
                "checkpoint",
                stream_payload(&run, "durable", &run.durable.state, false),
                true,
            );
        }
        Ok(_) => error!(event = "run_generation_checkpoint_wrong_writer_reply"),
        Err(reason) => error!(
            event = "run_generation_checkpoint_failed",
            run_id = batch.run_id,
            reason = ?reason
        ),
    }
}

fn finish_executor_terminal(
    context: &SchedulerContext,
    active: &mut HashSet<String>,
    checkpoints: &mut HashMap<String, GenerationCheckpointState>,
    completed: ExecutorTerminal,
) {
    // The executor sends every batch before its terminal message. Drain that separate
    // bounded channel now so the terminal transaction cannot overtake the final batch.
    while let Ok(batch) = context.envelopes.try_recv() {
        handle_generated_batch(context, checkpoints, batch);
    }
    let (run_id, weight, operation) = match completed {
        ExecutorTerminal::Manual(completed) => {
            let run_id = completed.run_id.clone();
            let weight = result_weight(&completed.result);
            (run_id, weight, WriterOperation::Complete(completed))
        }
        ExecutorTerminal::Generated(mut completed) => {
            let run_id = completed.run_id.clone();
            if let Some(progress) = checkpoints.remove(&run_id) {
                completed.backpressure_events = completed
                    .backpressure_events
                    .max(progress.backpressure_events);
            }
            (
                run_id,
                32 * 1024,
                WriterOperation::CompleteGenerated(completed),
            )
        }
    };
    active.remove(&run_id);
    if let Ok(mut controls) = context.controls.lock() {
        controls.remove(&run_id);
    }
    match context.writer.call(operation, weight) {
        Ok(WriterReply::Run(run)) => context.live.emit(
            &run.run_id,
            run.durable.checkpoint_sequence,
            "terminal",
            stream_payload(&run, "durable", &run.durable.state, true),
            true,
        ),
        Ok(_) => error!(event = "run_checkpoint_wrong_writer_reply"),
        Err(reason) => error!(event = "run_checkpoint_failed", run_id, reason = ?reason),
    }
}

fn next_candidate(
    connection: &Connection,
    active: &HashSet<String>,
) -> Result<Option<Candidate>, RunError> {
    let mut statement = connection
        .prepare(
            "SELECT r.run_id,r.revision_id,r.revision_digest,r.plan_digest,p.payload_json,r.captured_invocation_json,r.checkpoint_sequence,g.generated_count,g.logical_bytes,g.stream_digest,g.transform_node_id,g.transformed_count,g.transformed_logical_bytes,g.transformed_stream_digest,g.artifact_json,g.branch_node_id,g.branch_true_count,g.branch_false_count,g.branch_stream_digest
             FROM runs r JOIN execution_plans p ON p.plan_id=r.plan_id
             LEFT JOIN run_generation_progress g ON g.run_id=r.run_id
             WHERE r.state='queued' AND NOT EXISTS(SELECT 1 FROM run_suspensions s WHERE s.run_id=r.run_id)
             ORDER BY r.admitted_at,r.run_id LIMIT 64",
        )
        .map_err(storage_error)?;
    let mut rows = statement.query([]).map_err(storage_error)?;
    while let Some(row) = rows.next().map_err(storage_error)? {
        let run_id: String = row.get(0).map_err(storage_error)?;
        if active.contains(&run_id) {
            continue;
        }
        let plan_json: String = row.get(4).map_err(storage_error)?;
        let invocation_json: String = row.get(5).map_err(storage_error)?;
        let plan: ExecutionPlan = serde_json::from_str(&plan_json).map_err(|error| {
            RunError::Integrity(format!("queued pinned plan is invalid: {error}"))
        })?;
        let (_, edit_fields_definition, if_definition, merge_definition, summarize_definition) =
            generation_plan_and_transform(&plan)
                .map_err(|error| RunError::Integrity(error.message))?;
        let stored_transform_node_id = row.get::<_, Option<String>>(10).map_err(storage_error)?;
        let durable_transformed_count = row
            .get::<_, Option<i64>>(11)
            .map_err(storage_error)?
            .unwrap_or(0) as u64;
        let durable_transformed_logical_bytes = row
            .get::<_, Option<i64>>(12)
            .map_err(storage_error)?
            .unwrap_or(0) as u64;
        let durable_transformed_stream_digest = row
            .get::<_, Option<String>>(13)
            .map_err(storage_error)?
            .unwrap_or_else(|| "genesis".into());
        let artifact = row
            .get::<_, Option<String>>(14)
            .map_err(storage_error)?
            .as_deref()
            .map(parse_sql_json)
            .transpose()
            .map_err(storage_error)?
            .map(|value| {
                serde_json::from_value(value).map_err(|error| {
                    RunError::Integrity(format!("durable Artifact reference is invalid: {error}"))
                })
            })
            .transpose()?;
        let durable_generated_count = row
            .get::<_, Option<i64>>(7)
            .map_err(storage_error)?
            .unwrap_or(0) as u64;
        // Merge spools are finalized only after both streams close. If a daemon
        // stops before that barrier, replay the bounded source from zero rather
        // than pretending an unpersisted suffix is a complete Merge input.
        let replay_merge_from_zero = merge_definition.is_some() && durable_generated_count > 0;
        let stored_generated_count = if replay_merge_from_zero {
            0
        } else {
            durable_generated_count
        };
        let stored_branch_node_id = row.get::<_, Option<String>>(15).map_err(storage_error)?;
        let durable_branch_true_count = row
            .get::<_, Option<i64>>(16)
            .map_err(storage_error)?
            .unwrap_or(0) as u64;
        let durable_branch_false_count = row
            .get::<_, Option<i64>>(17)
            .map_err(storage_error)?
            .unwrap_or(0) as u64;
        let durable_branch_stream_digest = row
            .get::<_, Option<String>>(18)
            .map_err(storage_error)?
            .unwrap_or_else(|| "genesis".into());
        let transformed_count = if replay_merge_from_zero {
            0
        } else {
            durable_transformed_count
        };
        let transformed_logical_bytes = if replay_merge_from_zero {
            0
        } else {
            durable_transformed_logical_bytes
        };
        let transformed_stream_digest = if replay_merge_from_zero {
            "genesis".into()
        } else {
            durable_transformed_stream_digest
        };
        let branch_true_count = if replay_merge_from_zero {
            0
        } else {
            durable_branch_true_count
        };
        let branch_false_count = if replay_merge_from_zero {
            0
        } else {
            durable_branch_false_count
        };
        let branch_stream_digest = if replay_merge_from_zero {
            "genesis".into()
        } else {
            durable_branch_stream_digest
        };
        let edit_fields = match edit_fields_definition {
            Some((node_id, configuration)) => {
                if stored_transform_node_id
                    .as_deref()
                    .is_some_and(|stored| stored != node_id.as_str())
                {
                    return Err(RunError::Integrity(
                        "durable Edit Fields node identity changed".into(),
                    ));
                }
                Some(EditFieldsCandidate {
                    node_id,
                    configuration,
                    transformed_count,
                    transformed_logical_bytes,
                    transformed_stream_digest,
                })
            }
            None => {
                if stored_transform_node_id.is_some() || transformed_count != 0 {
                    return Err(RunError::Integrity(
                        "durable transform progress exists without Edit Fields".into(),
                    ));
                }
                None
            }
        };
        let if_node = match if_definition {
            Some((node_id, configuration)) => {
                if stored_branch_node_id
                    .as_deref()
                    .is_some_and(|stored| stored != node_id.as_str())
                {
                    return Err(RunError::Integrity(
                        "durable If node identity changed".into(),
                    ));
                }
                if branch_true_count.saturating_add(branch_false_count) != stored_generated_count {
                    return Err(RunError::Integrity(
                        "durable If counts do not cover the generated cursor".into(),
                    ));
                }
                if branch_stream_digest != "genesis"
                    && validate_tagged_digest(&branch_stream_digest, "branch_stream_digest")
                        .is_err()
                {
                    return Err(RunError::Integrity(
                        "durable If stream digest is invalid".into(),
                    ));
                }
                Some(IfCandidate {
                    node_id,
                    configuration,
                    true_count: branch_true_count,
                    false_count: branch_false_count,
                    stream_digest: branch_stream_digest,
                })
            }
            None => {
                if stored_branch_node_id.is_some()
                    || branch_true_count != 0
                    || branch_false_count != 0
                    || branch_stream_digest != "genesis"
                {
                    return Err(RunError::Integrity(
                        "durable branch progress exists without If".into(),
                    ));
                }
                None
            }
        };
        let merge = merge_definition.map(|(node_id, configuration)| MergeCandidate {
            node_id,
            configuration,
        });
        let summarize = summarize_definition.map(|(node_id, configuration)| SummarizeCandidate {
            node_id,
            configuration,
        });
        return Ok(Some(Candidate {
            run_id,
            revision_id: row.get(1).map_err(storage_error)?,
            revision_digest: row.get(2).map_err(storage_error)?,
            plan_digest: row.get(3).map_err(storage_error)?,
            plan,
            captured_invocation: serde_json::from_str(&invocation_json).map_err(|error| {
                RunError::Integrity(format!("queued invocation is invalid: {error}"))
            })?,
            checkpoint_sequence: row.get::<_, i64>(6).map_err(storage_error)? as u64,
            generate_resume: if replay_merge_from_zero {
                None
            } else {
                row.get::<_, Option<i64>>(7)
                    .map_err(storage_error)?
                    .map(|generated_count| {
                        Ok(GenerateResume {
                            next_ordinal: generated_count as u64,
                            logical_bytes: row.get::<_, i64>(8).map_err(storage_error)? as u64,
                            stream_digest: row.get(9).map_err(storage_error)?,
                        })
                    })
                    .transpose()?
            },
            edit_fields,
            if_node,
            merge,
            summarize,
            logical_data_override: None,
            artifact,
        }));
    }
    Ok(None)
}

fn run_state(connection: &Connection, run_id: &str) -> Result<String, RunError> {
    connection
        .query_row(
            "SELECT state FROM runs WHERE run_id=?1",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

fn load_run(connection: &Connection, run_id: &str) -> Result<RunView, RunError> {
    let mut run = connection
        .query_row(
            "SELECT run_id,run_request_id,workflow_id,publication_event_id,revision_id,revision_digest,plan_id,plan_digest,state,checkpoint_sequence,logical_order,attempted,succeeded,cancelled,failed,output_count,correctness_digest,digest_complete,admitted_at,started_at,updated_at,terminal_at
             FROM runs WHERE run_id=?1",
            params![run_id],
            |row| {
                let state: String = row.get(8)?;
                Ok(RunView {
                    schema: RUN_SCHEMA.into(),
                    run_id: row.get(0)?,
                    run_request_id: row.get(1)?,
                    workflow_id: row.get(2)?,
                    publication_event_id: row.get(3)?,
                    revision_id: row.get(4)?,
                    revision_digest: row.get(5)?,
                    plan_id: row.get(6)?,
                    plan_digest: row.get(7)?,
                    durable: DurableProgress {
                        terminal: is_terminal(&state),
                        state,
                        checkpoint_sequence: row.get::<_, i64>(9)? as u64,
                        logical_order: row.get::<_, i64>(10)? as u64,
                        updated_at: row.get(20)?,
                    },
                    live: None,
                    correctness: CorrectnessView {
                        canonicalization: CANONICALIZATION.into(),
                        algorithm: DIGEST_ALGORITHM.into(),
                        digest: row.get(16)?,
                        complete: row.get::<_, i64>(17)? == 1,
                        attempted: row.get::<_, i64>(11)? as u64,
                        succeeded: row.get::<_, i64>(12)? as u64,
                        cancelled: row.get::<_, i64>(13)? as u64,
                        failed: row.get::<_, i64>(14)? as u64,
                        output_count: row.get::<_, i64>(15)? as u64,
                    },
                    generation: None,
                    admitted_at: row.get(18)?,
                    started_at: row.get(19)?,
                    terminal_at: row.get(21)?,
                    queue_profile: queue_profile(),
                })
            },
        )
        .map_err(|error| {
            if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                RunError::NotFound
            } else {
                storage_error(error)
            }
        })?;
    run.generation = load_generation_progress(connection, run_id)?;
    let suspended: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM run_suspensions WHERE run_id=?1)",
            params![run_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?
        == 1;
    if suspended && !run.durable.terminal {
        run.durable.state = "suspended".into();
    }
    Ok(run)
}

fn load_generation_progress(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<GenerationProgress>, RunError> {
    connection
        .query_row(
            "SELECT state,generated_count,logical_bytes,stream_digest,backpressure_events,artifact_json,transform_node_id,transformed_count,transformed_logical_bytes,transformed_stream_digest,branch_node_id,branch_true_count,branch_false_count,branch_stream_digest,merge_json,summary_json FROM run_generation_progress WHERE run_id=?1",
            params![run_id],
            |row| {
                let artifact: Option<String> = row.get(5)?;
                let transform_node_id: Option<String> = row.get(6)?;
                let transformed_count: u64 = row.get::<_, i64>(7)? as u64;
                let transformed_logical_bytes: u64 = row.get::<_, i64>(8)? as u64;
                let transformed_stream_digest: String = row.get(9)?;
                let branch_node_id: Option<String> = row.get(10)?;
                let branch_true_count: u64 = row.get::<_, i64>(11)? as u64;
                let branch_false_count: u64 = row.get::<_, i64>(12)? as u64;
                let branch_stream_digest: String = row.get(13)?;
                let merge: Option<String> = row.get(14)?;
                let summary: Option<String> = row.get(15)?;
                Ok(GenerationProgress {
                    state: row.get(0)?,
                    generated_count: row.get::<_, i64>(1)? as u64,
                    logical_bytes: row.get::<_, i64>(2)? as u64,
                    stream_digest: row.get(3)?,
                    backpressure_events: row.get::<_, i64>(4)? as u64,
                    artifact: artifact.as_deref().map(parse_sql_json).transpose()?.map(|value| {
                        serde_json::from_value(value).map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error)))
                    }).transpose()?,
                    transform: transform_node_id.map(|node_instance_id| TransformProgress {
                        node_instance_id,
                        transformed_count,
                        logical_bytes: transformed_logical_bytes,
                        stream_digest: transformed_stream_digest,
                    }),
                    branch: branch_node_id.map(|node_instance_id| BranchProgress {
                        node_instance_id,
                        true_count: branch_true_count,
                        false_count: branch_false_count,
                        stream_digest: branch_stream_digest,
                    }),
                    merge: merge
                        .as_deref()
                        .map(parse_sql_json)
                        .transpose()?
                        .map(|value| {
                            serde_json::from_value(value).map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    14,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })
                        })
                        .transpose()?,
                    summary: summary
                        .as_deref()
                        .map(parse_sql_json)
                        .transpose()?
                        .map(|value| {
                            serde_json::from_value(value).map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    15,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })
                        })
                        .transpose()?,
                })
            },
        )
        .optional()
        .map_err(storage_error)
}

fn load_trace(connection: &Connection, run_id: &str) -> Result<TraceView, RunError> {
    let run = load_run(connection, run_id)?;
    let checkpoints = {
        let mut statement = connection
            .prepare(
                "SELECT checkpoint_sequence,state,logical_order,snapshot_json,trace_head_hash,checkpoint_hash,committed_at FROM run_checkpoints WHERE run_id=?1 ORDER BY checkpoint_sequence",
            )
            .map_err(storage_error)?;
        let values = statement
            .query_map(params![run_id], |row| {
                let raw: String = row.get(3)?;
                Ok(CheckpointView {
                    sequence: row.get::<_, i64>(0)? as u64,
                    state: row.get(1)?,
                    logical_order: row.get::<_, i64>(2)? as u64,
                    snapshot: serde_json::from_str(&raw).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            raw.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                    trace_head_hash: row.get(4)?,
                    checkpoint_hash: row.get(5)?,
                    committed_at: row.get(6)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        drop(statement);
        values
    };
    let activations = {
        let mut statement = connection
            .prepare(
                "SELECT activation_id,node_instance_id,logical_order,attempt,outcome,input_json,output_json,input_digest,output_digest,provenance_json,failure_json,checkpoint_sequence,started_at,completed_at,elapsed_micros FROM run_activations WHERE run_id=?1 ORDER BY logical_order,attempt",
            )
            .map_err(storage_error)?;
        let values = statement
            .query_map(params![run_id], |row| {
                let input: String = row.get(5)?;
                let output: Option<String> = row.get(6)?;
                let provenance: String = row.get(9)?;
                let failure: Option<String> = row.get(10)?;
                Ok(ActivationView {
                    activation_id: row.get(0)?,
                    node_instance_id: row.get(1)?,
                    logical_order: row.get::<_, i64>(2)? as u64,
                    attempt: row.get::<_, i64>(3)? as u32,
                    outcome: row.get(4)?,
                    input: parse_sql_json(&input)?,
                    output: output.as_deref().map(parse_sql_json).transpose()?,
                    input_digest: row.get(7)?,
                    output_digest: row.get(8)?,
                    provenance: parse_sql_json(&provenance)?,
                    failure: failure.as_deref().map(parse_sql_json).transpose()?,
                    checkpoint_sequence: row.get::<_, i64>(11)? as u64,
                    timing: TimingView {
                        started_at: row.get(12)?,
                        completed_at: row.get(13)?,
                        elapsed_micros: row.get::<_, i64>(14)? as u64,
                    },
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        drop(statement);
        values
    };
    let events = {
        let mut statement = connection
            .prepare(
                "SELECT event_sequence,logical_order,checkpoint_sequence,phase,event_type,payload_json,previous_hash,event_hash,occurred_at FROM run_trace_events WHERE run_id=?1 ORDER BY event_sequence",
            )
            .map_err(storage_error)?;
        let values = statement
            .query_map(params![run_id], |row| {
                let payload: String = row.get(5)?;
                Ok(TraceEventView {
                    event_sequence: row.get::<_, i64>(0)? as u64,
                    logical_order: row.get::<_, Option<i64>>(1)?.map(|value| value as u64),
                    checkpoint_sequence: row.get::<_, i64>(2)? as u64,
                    phase: row.get(3)?,
                    event_type: row.get(4)?,
                    payload: parse_sql_json(&payload)?,
                    previous_hash: row.get(6)?,
                    event_hash: row.get(7)?,
                    occurred_at: row.get(8)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        drop(statement);
        values
    };
    let expected_trace_head: String = connection
        .query_row(
            "SELECT trace_head_hash FROM runs WHERE run_id=?1",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    verify_trace(
        run_id,
        &events,
        &checkpoints,
        &activations,
        &expected_trace_head,
    )?;
    Ok(TraceView {
        schema: TRACE_SCHEMA.into(),
        run: TraceRunIdentity {
            run_id: run.run_id,
            workflow_id: run.workflow_id,
            publication_event_id: run.publication_event_id,
            revision_id: run.revision_id,
            revision_digest: run.revision_digest,
            plan_id: run.plan_id,
            plan_digest: run.plan_digest,
        },
        terminal_state: run.durable.state,
        correctness: run.correctness,
        checkpoints,
        activations,
        events,
        safe_resource_facts: safe_resource_facts(),
        integrity_verified: true,
    })
}

fn verify_trace(
    run_id: &str,
    events: &[TraceEventView],
    checkpoints: &[CheckpointView],
    activations: &[ActivationView],
    expected_trace_head: &str,
) -> Result<(), RunError> {
    let mut previous = "genesis".to_owned();
    for (index, event) in events.iter().enumerate() {
        if event.event_sequence != index as u64 + 1 {
            return Err(RunError::Integrity(
                "Causal Trace event sequence is discontinuous".into(),
            ));
        }
        if event.previous_hash != previous {
            return Err(RunError::Integrity(
                "Causal Trace chain is discontinuous".into(),
            ));
        }
        let calculated = trace_event_hash(
            run_id,
            event.event_sequence,
            event.logical_order,
            event.checkpoint_sequence,
            &event.phase,
            &event.event_type,
            &event.payload,
            &event.previous_hash,
            event.occurred_at,
        )?;
        if calculated != event.event_hash {
            return Err(RunError::Integrity("Causal Trace event hash failed".into()));
        }
        previous = event.event_hash.clone();
    }
    if previous != expected_trace_head {
        return Err(RunError::Integrity(
            "Causal Trace head does not match the Run projection".into(),
        ));
    }
    for (index, checkpoint) in checkpoints.iter().enumerate() {
        if checkpoint.sequence != index as u64 + 1 {
            return Err(RunError::Integrity(
                "Durable Checkpoint sequence is discontinuous".into(),
            ));
        }
        if !events.iter().any(|event| {
            event.checkpoint_sequence == checkpoint.sequence
                && event.event_hash == checkpoint.trace_head_hash
        }) {
            return Err(RunError::Integrity(
                "Durable Checkpoint trace head is unavailable".into(),
            ));
        }
        let calculated = checkpoint_hash(
            run_id,
            checkpoint.sequence,
            &checkpoint.state,
            checkpoint.logical_order,
            &checkpoint.snapshot,
            &checkpoint.trace_head_hash,
            checkpoint.committed_at,
        )?;
        if calculated != checkpoint.checkpoint_hash {
            return Err(RunError::Integrity("Durable Checkpoint hash failed".into()));
        }
    }
    for activation in activations {
        if digest(&activation.input).map_err(RunError::Integrity)? != activation.input_digest {
            return Err(RunError::Integrity("Activation input digest failed".into()));
        }
        let output_digest = activation
            .output
            .as_ref()
            .map(digest)
            .transpose()
            .map_err(RunError::Integrity)?;
        if output_digest != activation.output_digest {
            return Err(RunError::Integrity(
                "Activation output digest failed".into(),
            ));
        }
        if !checkpoints
            .iter()
            .any(|checkpoint| checkpoint.sequence == activation.checkpoint_sequence)
        {
            return Err(RunError::Integrity(
                "Activation Durable Checkpoint is unavailable".into(),
            ));
        }
        if !events.iter().any(|event| {
            event.logical_order == Some(activation.logical_order)
                && event.checkpoint_sequence == activation.checkpoint_sequence
                && event.event_type == "activation_outcome"
                && event.payload["activation_id"].as_str()
                    == Some(activation.activation_id.as_str())
                && event.payload["outcome"].as_str() == Some(activation.outcome.as_str())
                && event.payload["input_digest"].as_str() == Some(activation.input_digest.as_str())
                && event.payload["output_digest"].as_str() == activation.output_digest.as_deref()
        }) {
            return Err(RunError::Integrity(
                "Activation Causal Trace event is unavailable".into(),
            ));
        }
    }
    Ok(())
}

struct StoredTraceEvent {
    run_id: String,
    event_sequence: u64,
    logical_order: Option<u64>,
    checkpoint_sequence: u64,
    phase: String,
    event_type: String,
    payload: Value,
    previous_hash: String,
    event_hash: String,
    occurred_at: i64,
}

#[allow(clippy::too_many_arguments)]
fn make_trace_event(
    run_id: &str,
    event_sequence: u64,
    logical_order: Option<u64>,
    checkpoint_sequence: u64,
    phase: &str,
    event_type: &str,
    payload: Value,
    previous_hash: &str,
    occurred_at: i64,
) -> Result<StoredTraceEvent, RunError> {
    let event_hash = trace_event_hash(
        run_id,
        event_sequence,
        logical_order,
        checkpoint_sequence,
        phase,
        event_type,
        &payload,
        previous_hash,
        occurred_at,
    )?;
    Ok(StoredTraceEvent {
        run_id: run_id.into(),
        event_sequence,
        logical_order,
        checkpoint_sequence,
        phase: phase.into(),
        event_type: event_type.into(),
        payload,
        previous_hash: previous_hash.into(),
        event_hash,
        occurred_at,
    })
}

#[allow(clippy::too_many_arguments)]
fn trace_event_hash(
    run_id: &str,
    event_sequence: u64,
    logical_order: Option<u64>,
    checkpoint_sequence: u64,
    phase: &str,
    event_type: &str,
    payload: &Value,
    previous_hash: &str,
    occurred_at: i64,
) -> Result<String, RunError> {
    digest(&json!({
        "schema": "canopy.trace-event-hash/v1alpha1",
        "run_id": run_id,
        "event_sequence": event_sequence,
        "logical_order": logical_order,
        "checkpoint_sequence": checkpoint_sequence,
        "phase": phase,
        "event_type": event_type,
        "payload": payload,
        "previous_hash": previous_hash,
        "occurred_at": occurred_at
    }))
    .map_err(RunError::Integrity)
}

fn insert_trace_event(
    transaction: &Transaction<'_>,
    event: &StoredTraceEvent,
) -> Result<(), RunError> {
    transaction
        .execute(
            "INSERT INTO run_trace_events(run_id,event_sequence,logical_order,checkpoint_sequence,phase,event_type,payload_json,previous_hash,event_hash,occurred_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                event.run_id,
                event.event_sequence as i64,
                event.logical_order.map(|value| value as i64),
                event.checkpoint_sequence as i64,
                event.phase,
                event.event_type,
                canonical_text(&event.payload)?,
                event.previous_hash,
                event.event_hash,
                event.occurred_at
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_checkpoint(
    transaction: &Transaction<'_>,
    run_id: &str,
    sequence: u64,
    state: &str,
    logical_order: u64,
    snapshot: &Value,
    trace_head_hash: &str,
    committed_at: i64,
) -> Result<(), RunError> {
    let checkpoint_hash = checkpoint_hash(
        run_id,
        sequence,
        state,
        logical_order,
        snapshot,
        trace_head_hash,
        committed_at,
    )?;
    transaction
        .execute(
            "INSERT INTO run_checkpoints(run_id,checkpoint_sequence,state,logical_order,snapshot_json,trace_head_hash,checkpoint_hash,committed_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                run_id,
                sequence as i64,
                state,
                logical_order as i64,
                canonical_text(snapshot)?,
                trace_head_hash,
                checkpoint_hash,
                committed_at
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn checkpoint_hash(
    run_id: &str,
    sequence: u64,
    state: &str,
    logical_order: u64,
    snapshot: &Value,
    trace_head_hash: &str,
    committed_at: i64,
) -> Result<String, RunError> {
    digest(&json!({
        "schema": "canopy.checkpoint-hash/v1alpha1",
        "run_id": run_id,
        "checkpoint_sequence": sequence,
        "state": state,
        "logical_order": logical_order,
        "snapshot": snapshot,
        "trace_head_hash": trace_head_hash,
        "committed_at": committed_at
    }))
    .map_err(RunError::Integrity)
}

fn next_trace_sequence(transaction: &Transaction<'_>, run_id: &str) -> Result<u64, RunError> {
    transaction
        .query_row(
            "SELECT COALESCE(MAX(event_sequence),0)+1 FROM run_trace_events WHERE run_id=?1",
            params![run_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value as u64)
        .map_err(storage_error)
}

fn connect(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

fn canonical_text<T: Serialize>(value: &T) -> Result<String, RunError> {
    String::from_utf8(canonical_bytes(value).map_err(RunError::Integrity)?)
        .map_err(|error| RunError::Integrity(error.to_string()))
}

fn parse_sql_json(value: &str) -> Result<Value, rusqlite::Error> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn storage_error(error: rusqlite::Error) -> RunError {
    RunError::Storage(error.to_string())
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn random_id(prefix: &str) -> String {
    let mut raw = [0_u8; 16];
    OsRng.fill_bytes(&mut raw);
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, raw);
    format!("{prefix}-{encoded}")
}

fn admission_request_digest(
    workflow_id: &str,
    request: &AdmitRunRequest,
) -> Result<String, RunError> {
    digest(&json!({
        "workflow_id": workflow_id,
        "request": request
    }))
    .map_err(RunError::Integrity)
}

fn validate_identifier(value: &str, field: &'static str) -> Result<(), RunError> {
    let valid = !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(RunError::Invalid(field))
    }
}

fn validate_tagged_digest(value: &str, field: &'static str) -> Result<(), RunError> {
    let valid = value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(RunError::Invalid(field))
    }
}

fn reject_sensitive_keys(value: &Value) -> Result<(), RunError> {
    const SENSITIVE: &[&str] = &[
        "authorization",
        "cookie",
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "access_token",
        "refresh_token",
        "id_token",
        "auth_token",
        "bearer_token",
        "session_token",
        "client_secret",
        "credential",
        "credentials",
        "secret_key",
        "signing_key",
        "ssh_key",
    ];
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                let normalized = key.to_ascii_lowercase().replace('-', "_");
                let sensitive_suffix = [
                    "_password",
                    "_passwd",
                    "_secret",
                    "_token",
                    "_credential",
                    "_credentials",
                    "_private_key",
                    "_api_key",
                ]
                .iter()
                .any(|suffix| normalized.ends_with(suffix));
                if SENSITIVE.contains(&normalized.as_str()) || sensitive_suffix {
                    return Err(RunError::Invalid("captured_invocation_sensitive_field"));
                }
                reject_sensitive_keys(nested)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                reject_sensitive_keys(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn outcome_name(outcome: &ActivationOutcome) -> &'static str {
    match outcome {
        ActivationOutcome::Success => "success",
        ActivationOutcome::Cancelled => "cancelled",
        ActivationOutcome::PermanentFailure => "permanent_failure",
    }
}

fn is_terminal(state: &str) -> bool {
    matches!(state, "succeeded" | "failed" | "cancelled")
}

fn counters_json(
    attempted: u64,
    succeeded: u64,
    cancelled: u64,
    failed: u64,
    output_count: u64,
) -> Value {
    json!({
        "attempted": attempted,
        "succeeded": succeeded,
        "cancelled": cancelled,
        "failed": failed,
        "output_count": output_count
    })
}

fn queue_profile() -> QueueProfileView {
    QueueProfileView {
        profile: QUEUE_PROFILE.into(),
        maximum_nonterminal_runs: MAX_NONTERMINAL_RUNS,
        maximum_hot_runs: MAX_HOT_RUNS,
        ready: QueueLimitView {
            count: READY_QUEUE_COUNT,
            bytes: READY_QUEUE_BYTES,
        },
        envelopes: QueueLimitView {
            count: ENVELOPE_QUEUE_COUNT,
            bytes: ENVELOPE_QUEUE_BYTES,
        },
        results: QueueLimitView {
            count: RESULT_QUEUE_COUNT,
            bytes: RESULT_QUEUE_BYTES,
        },
        writer: QueueLimitView {
            count: WRITER_QUEUE_COUNT,
            bytes: WRITER_QUEUE_BYTES,
        },
        live_ring_per_run: QueueLimitView {
            count: LIVE_RING_COUNT,
            bytes: LIVE_RING_BYTES,
        },
        subscribers: SubscriberLimitView {
            count: MAX_SSE_SUBSCRIBERS,
            mailbox: QueueLimitView {
                count: SUBSCRIBER_QUEUE_COUNT,
                bytes: SUBSCRIBER_QUEUE_BYTES,
            },
        },
        maximum_inline_invocation_bytes: MAX_INVOCATION_BYTES,
    }
}

fn safe_resource_facts() -> Value {
    json!({
        "profile": QUEUE_PROFILE,
        "native_executor_threads": 1,
        "scheduler": "bounded-weighted-fair-ready-order",
        "queues": queue_profile(),
        "checkpoint_policy": {
            "maximum_outcomes": CHECKPOINT_MAX_OUTCOMES,
            "maximum_bytes": CHECKPOINT_MAX_BYTES,
            "maximum_latency_millis": CHECKPOINT_MAX_LATENCY_MILLIS,
            "control_and_terminal_barriers": true
        },
        "credentials_recorded": false,
        "private_reasoning_recorded": false
    })
}

fn load_artifact_data(
    artifacts: &ArtifactService,
    artifact_id: &str,
) -> Result<(ArtifactReference, Value), RunError> {
    let (view, bytes, _, _, _) =
        artifacts
            .content(artifact_id, None)
            .map_err(|error| match error {
                ArtifactError::Storage(message) => RunError::Storage(message),
                ArtifactError::NotAuthorized => {
                    RunError::Integrity("Generate Items Artifact reference was denied".into())
                }
                other => RunError::Integrity(format!(
                    "Generate Items Artifact could not be verified: {other}"
                )),
            })?;
    let logical_data: Value = serde_json::from_slice(&bytes)
        .map_err(|_| RunError::Integrity("Generate Items Artifact is not valid JSON".into()))?;
    reject_sensitive_keys(&logical_data)?;
    Ok((view.reference, logical_data))
}

fn prepare_candidate_artifact(
    artifacts: &ArtifactService,
    candidate: &mut Candidate,
) -> Result<(), RunError> {
    if let Some(reference) = candidate.artifact.as_ref() {
        let (reference, logical_data) = load_artifact_data(artifacts, &reference.artifact_id)?;
        candidate.logical_data_override = Some(logical_data);
        candidate.artifact = Some(reference);
        return Ok(());
    }
    let Some(node) = candidate
        .plan
        .nodes
        .iter()
        .find(|node| node.contract_lock.name == "generate-items")
    else {
        return Ok(());
    };
    let node_id = node.node_instance_id.clone();
    let configuration = node.configuration.clone();
    let data = configuration.get("data").cloned().unwrap_or(Value::Null);
    if let Some(artifact_id) = configured_artifact_id(&data) {
        let (reference, logical_data) = load_artifact_data(artifacts, &artifact_id)?;
        candidate.logical_data_override = Some(logical_data);
        candidate.artifact = Some(reference);
        return Ok(());
    }

    reject_sensitive_keys(&data)?;
    let plaintext = canonical_bytes(&data).map_err(RunError::Integrity)?;
    let force = configuration["storage_mode"] == "artifact";
    if !force && plaintext.len() <= 4 * 1024 {
        return Ok(());
    }
    let count = configuration["count"].as_u64().unwrap_or_default();
    let reference_id = format!("run:{}:node:{node_id}:data", candidate.run_id);
    let view = artifacts
        .put(
            &plaintext,
            "application/json",
            &reference_id,
            "run_generate_data",
            count.max(1),
        )
        .map_err(|error| match error {
            ArtifactError::Storage(message) => RunError::Storage(message),
            other => RunError::Integrity(format!("Artifact preparation failed: {other}")),
        })?;
    candidate.artifact = Some(view.reference);
    Ok(())
}

fn configured_artifact_id(value: &Value) -> Option<String> {
    let handle = value.as_object()?.get("$artifact")?;
    match handle {
        Value::String(artifact_id) => Some(artifact_id.clone()),
        Value::Object(reference) => reference
            .get("artifact_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

fn candidate_weight(candidate: &Candidate) -> usize {
    canonical_bytes(&candidate.captured_invocation)
        .map(|bytes| bytes.len())
        .unwrap_or(MAX_INVOCATION_BYTES)
        .saturating_add(8 * 1024)
}

fn result_weight(result: &ManualActivationResult) -> usize {
    canonical_bytes(result)
        .map(|bytes| bytes.len())
        .unwrap_or(64 * 1024)
        .saturating_add(4 * 1024)
}

fn bounded_stream_data(value: &Value) -> Result<String, RunError> {
    let data = canonical_text(value)?;
    if data.len() > MAX_SSE_DATA_BYTES {
        Err(RunError::TooLarge("sse_event"))
    } else {
        Ok(data)
    }
}

fn frame_bytes(frame: &StreamFrame) -> usize {
    frame.id.len() + frame.data.len() + frame.event.len() + 32
}

fn stream_payload(run: &RunView, durability: &str, state: &str, terminal: bool) -> Value {
    json!({
        "run_id": run.run_id,
        "durability": durability,
        "state": state,
        "durable_checkpoint_sequence": run.durable.checkpoint_sequence,
        "logical_order": run.durable.logical_order,
        "terminal": terminal,
        "correctness_digest": run.correctness.digest
    })
}

struct ByteBudget {
    maximum: usize,
    used: AtomicUsize,
}

impl ByteBudget {
    fn new(maximum: usize) -> Self {
        Self {
            maximum,
            used: AtomicUsize::new(0),
        }
    }

    fn reserve(self: &Arc<Self>, bytes: usize) -> Option<BytePermit> {
        if bytes > self.maximum {
            return None;
        }
        let reserved = self
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(bytes)
                    .filter(|next| *next <= self.maximum)
            })
            .is_ok();
        reserved.then(|| BytePermit {
            budget: self.clone(),
            bytes,
        })
    }
}

struct BytePermit {
    budget: Arc<ByteBudget>,
    bytes: usize,
}

impl Drop for BytePermit {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

struct LiveHub {
    boot_epoch: String,
    runs: Mutex<HashMap<String, LiveRun>>,
}

struct LiveRun {
    durable_sequence: u64,
    next_live_sequence: u64,
    current: Option<LiveProgress>,
    ring: VecDeque<StoredFrame>,
    ring_bytes: usize,
    subscribers: Vec<async_mpsc::Sender<StreamFrame>>,
}

#[derive(Clone)]
struct StoredFrame {
    sequence: u64,
    bytes: usize,
    frame: StreamFrame,
}

impl LiveHub {
    fn new() -> Self {
        Self {
            boot_epoch: random_id("boot"),
            runs: Mutex::new(HashMap::new()),
        }
    }

    fn snapshot(&self, run_id: &str) -> Option<LiveProgress> {
        self.runs
            .lock()
            .ok()
            .and_then(|runs| runs.get(run_id).and_then(|run| run.current.clone()))
    }

    fn emit(
        &self,
        run_id: &str,
        durable_sequence: u64,
        event: &str,
        payload: Value,
        terminal: bool,
    ) {
        let Ok(data) = canonical_text(&payload) else {
            return;
        };
        if data.len() > MAX_SSE_DATA_BYTES {
            warn!(event = "run_sse_event_rejected", run_id, bytes = data.len());
            return;
        }
        let Ok(mut runs) = self.runs.lock() else {
            return;
        };
        for run in runs.values_mut() {
            run.subscribers.retain(|subscriber| !subscriber.is_closed());
        }
        if !runs.contains_key(run_id) {
            if event != "live" {
                return;
            }
            if runs.len() >= MAX_HOT_RUNS {
                if let Some(evict) = runs
                    .iter()
                    .find(|(_, run)| run.subscribers.is_empty())
                    .map(|(identity, _)| identity.clone())
                {
                    runs.remove(&evict);
                }
            }
            if runs.len() >= MAX_HOT_RUNS {
                return;
            }
            runs.insert(
                run_id.into(),
                LiveRun {
                    durable_sequence,
                    next_live_sequence: 1,
                    current: None,
                    ring: VecDeque::new(),
                    ring_bytes: 0,
                    subscribers: vec![],
                },
            );
        }
        let Some(run) = runs.get_mut(run_id) else {
            return;
        };
        run.durable_sequence = run.durable_sequence.max(durable_sequence);
        let sequence = run.next_live_sequence;
        run.next_live_sequence += 1;
        let id = cursor(run_id, run.durable_sequence, &self.boot_epoch, sequence);
        let frame = StreamFrame {
            event: event.into(),
            id,
            data,
        };
        let bytes = frame_bytes(&frame);
        if bytes > MAX_SSE_FRAME_BYTES {
            warn!(event = "run_sse_frame_rejected", run_id, bytes);
            return;
        }
        run.ring.push_back(StoredFrame {
            sequence,
            bytes,
            frame: frame.clone(),
        });
        run.ring_bytes = run.ring_bytes.saturating_add(bytes);
        while run.ring.len() > LIVE_RING_COUNT || run.ring_bytes > LIVE_RING_BYTES {
            if let Some(removed) = run.ring.pop_front() {
                run.ring_bytes = run.ring_bytes.saturating_sub(removed.bytes);
            } else {
                break;
            }
        }
        run.current = Some(LiveProgress {
            state: payload["state"].as_str().unwrap_or(event).into(),
            speculative: payload["durability"] == "speculative",
            boot_epoch: self.boot_epoch.clone(),
            sequence,
        });
        run.subscribers
            .retain(|subscriber| match subscriber.try_send(frame.clone()) {
                Ok(()) => true,
                Err(async_mpsc::error::TrySendError::Full(_)) => false,
                Err(async_mpsc::error::TrySendError::Closed(_)) => false,
            });
        if terminal {
            info!(event = "run_terminal_notified", run_id, sequence);
        }
    }

    fn subscribe(
        &self,
        run_id: &str,
        last_event_id: Option<&str>,
        snapshot: &RunView,
    ) -> Result<async_mpsc::Receiver<StreamFrame>, RunError> {
        let (sender, receiver) = async_mpsc::channel(SUBSCRIBER_QUEUE_COUNT);
        let mut runs = self
            .runs
            .lock()
            .map_err(|_| RunError::Storage("live Run hub is poisoned".into()))?;
        for run in runs.values_mut() {
            run.subscribers.retain(|subscriber| !subscriber.is_closed());
        }
        if !runs.contains_key(run_id) && runs.len() >= MAX_HOT_RUNS {
            if let Some(evict) = runs
                .iter()
                .find(|(_, run)| run.subscribers.is_empty())
                .map(|(identity, _)| identity.clone())
            {
                runs.remove(&evict);
            }
        }
        if !runs.contains_key(run_id) && runs.len() >= MAX_HOT_RUNS {
            return Err(RunError::SubscriberFull);
        }
        let run = runs.entry(run_id.into()).or_insert_with(|| LiveRun {
            durable_sequence: snapshot.durable.checkpoint_sequence,
            next_live_sequence: 1,
            current: None,
            ring: VecDeque::new(),
            ring_bytes: 0,
            subscribers: vec![],
        });
        let current_sequence = run.next_live_sequence.saturating_sub(1);
        let current_cursor = cursor(
            run_id,
            run.durable_sequence
                .max(snapshot.durable.checkpoint_sequence),
            &self.boot_epoch,
            current_sequence,
        );
        let snapshot_data = bounded_stream_data(&json!({
            "kind": "snapshot",
            "run": snapshot
        }))?;
        let snapshot_frame = StreamFrame {
            event: "snapshot".into(),
            id: current_cursor.clone(),
            data: snapshot_data,
        };
        match last_event_id {
            None | Some("") => {
                let _ = sender.try_send(snapshot_frame);
            }
            Some(last) => {
                let valid = parse_cursor(last).filter(|parsed| {
                    parsed.run_id == run_id
                        && parsed.boot_epoch == self.boot_epoch
                        && parsed.live_sequence <= current_sequence
                        && parsed.durable_sequence <= run.durable_sequence
                });
                let oldest = run
                    .ring
                    .front()
                    .map(|entry| entry.sequence)
                    .unwrap_or(current_sequence.saturating_add(1));
                let mut replayed = false;
                if let Some(parsed) =
                    valid.filter(|parsed| parsed.live_sequence.saturating_add(1) >= oldest)
                {
                    let suffix: Vec<_> = run
                        .ring
                        .iter()
                        .filter(|entry| entry.sequence > parsed.live_sequence)
                        .collect();
                    let suffix_bytes = suffix
                        .iter()
                        .fold(0_usize, |total, entry| total.saturating_add(entry.bytes));
                    if suffix.len() <= SUBSCRIBER_QUEUE_COUNT
                        && suffix_bytes <= SUBSCRIBER_QUEUE_BYTES
                    {
                        for entry in suffix {
                            let _ = sender.try_send(entry.frame.clone());
                        }
                        replayed = true;
                    }
                }
                if !replayed {
                    let gap = StreamFrame {
                        event: "gap".into(),
                        id: current_cursor.clone(),
                        data: bounded_stream_data(&json!({
                            "kind": "gap",
                            "run_id": run_id,
                            "reason": "cursor_unavailable",
                            "requested_cursor": last,
                            "resync_required": true
                        }))?,
                    };
                    let resync = StreamFrame {
                        event: "resync".into(),
                        id: current_cursor,
                        data: bounded_stream_data(&json!({
                            "kind": "resync",
                            "run": snapshot
                        }))?,
                    };
                    let _ = sender.try_send(gap);
                    let _ = sender.try_send(resync);
                }
            }
        }
        run.subscribers.push(sender);
        Ok(receiver)
    }
}

struct ParsedCursor {
    run_id: String,
    durable_sequence: u64,
    boot_epoch: String,
    live_sequence: u64,
}

fn cursor(run_id: &str, durable: u64, boot_epoch: &str, live: u64) -> String {
    format!("v1.{run_id}.{durable}.{boot_epoch}.{live}")
}

fn parse_cursor(value: &str) -> Option<ParsedCursor> {
    let mut fields = value.split('.');
    if fields.next()? != "v1" {
        return None;
    }
    let parsed = ParsedCursor {
        run_id: fields.next()?.into(),
        durable_sequence: fields.next()?.parse().ok()?,
        boot_epoch: fields.next()?.into(),
        live_sequence: fields.next()?.parse().ok()?,
    };
    fields.next().is_none().then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_preparation_failure_cannot_strand_a_requested_cancellation() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE workflow_drafts(workflow_id TEXT PRIMARY KEY);
             CREATE TABLE publication_events(event_id TEXT PRIMARY KEY);
             CREATE TABLE workflow_revisions(revision_id TEXT PRIMARY KEY);
             CREATE TABLE execution_plans(plan_id TEXT PRIMARY KEY);",
            )
            .unwrap();
        connection
            .execute_batch(
                "INSERT INTO workflow_drafts VALUES('workflow-test');
             INSERT INTO publication_events VALUES('event-test');
             INSERT INTO workflow_revisions VALUES('revision-test');
             INSERT INTO execution_plans VALUES('plan-test');",
            )
            .unwrap();
        initialize_schema(&connection).unwrap();
        connection.execute(
            "INSERT INTO runs(
                run_id,run_request_id,request_digest,workflow_id,publication_event_id,
                revision_id,revision_digest,plan_id,plan_digest,captured_invocation_json,
                state,checkpoint_sequence,logical_order,attempted,succeeded,cancelled,failed,
                output_count,correctness_digest,digest_complete,cancellation_request_id,
                cancellation_request_digest,trace_head_hash,admitted_at,started_at,updated_at,terminal_at
             ) VALUES(
                'run-cancel-preparation','request-cancel-preparation','sha256:request','workflow-test','event-test',
                'revision-test','sha256:revision','plan-test','sha256:plan','{}',
                'cancel_requested',0,0,0,0,0,0,0,NULL,0,'cancel-request','sha256:cancel','genesis',0,NULL,0,NULL
             )",
            [],
        ).unwrap();

        let run = preparation_failed_transaction(
            &mut connection,
            "run-cancel-preparation",
            "injected storage pressure",
            true,
        )
        .unwrap();

        assert_eq!(run.durable.state, "cancelled");
        assert!(run.durable.terminal);
        let stored: String = connection
            .query_row(
                "SELECT state FROM runs WHERE run_id='run-cancel-preparation'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, "cancelled");
    }

    #[test]
    fn cancellation_before_if_activation_is_terminal_and_idempotent() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE workflow_drafts(workflow_id TEXT PRIMARY KEY);
             CREATE TABLE publication_events(event_id TEXT PRIMARY KEY);
             CREATE TABLE workflow_revisions(revision_id TEXT PRIMARY KEY);
             CREATE TABLE execution_plans(plan_id TEXT PRIMARY KEY);",
            )
            .unwrap();
        initialize_schema(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO workflow_drafts VALUES('workflow-if-cancel');
             INSERT INTO publication_events VALUES('event-if-cancel');
             INSERT INTO workflow_revisions VALUES('revision-if-cancel');
             INSERT INTO execution_plans VALUES('plan-if-cancel');",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO runs(
                    run_id,run_request_id,request_digest,workflow_id,publication_event_id,
                    revision_id,revision_digest,plan_id,plan_digest,captured_invocation_json,
                    state,checkpoint_sequence,logical_order,attempted,succeeded,cancelled,failed,
                    output_count,correctness_digest,digest_complete,cancellation_request_id,
                    cancellation_request_digest,trace_head_hash,admitted_at,started_at,updated_at,terminal_at
                 ) VALUES(
                    'run-if-cancel','request-if-cancel','sha256:request','workflow-if-cancel','event-if-cancel',
                    'revision-if-cancel','sha256:revision','plan-if-cancel','sha256:plan','{}',
                    'queued',1,0,0,0,0,0,0,NULL,0,NULL,NULL,'genesis',0,NULL,0,NULL
                 )",
                [],
            )
            .unwrap();

        let request = CancelRunRequest {
            cancellation_request_id: "cancel-if-before-route".into(),
        };
        let cancelled =
            cancel_transaction(&mut connection, "run-if-cancel", request.clone(), false).unwrap();
        assert!(cancelled.accepted);
        assert!(!cancelled.already_terminal);
        assert_eq!(cancelled.run.durable.state, "cancelled");
        assert!(cancelled.run.durable.terminal);
        assert_eq!(cancelled.run.correctness.output_count, 0);

        let repeated =
            cancel_transaction(&mut connection, "run-if-cancel", request, false).unwrap();
        assert!(repeated.accepted);
        assert!(repeated.already_terminal);
        assert_eq!(repeated.run.durable.state, "cancelled");
        let activation_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM run_activations WHERE run_id='run-if-cancel'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(activation_count, 0);
    }

    #[test]
    fn bounded_queue_send_failure_releases_reserved_bytes() {
        let budget = Arc::new(ByteBudget::new(10));
        let (sender, receiver) = mpsc::sync_channel::<BytePermit>(1);
        sender.try_send(budget.reserve(6).unwrap()).unwrap();
        let second = budget.reserve(4).unwrap();
        match sender.try_send(second) {
            Err(TrySendError::Full(permit)) => drop(permit),
            Err(TrySendError::Disconnected(permit)) => {
                drop(permit);
                panic!("bounded queue disconnected unexpectedly")
            }
            Ok(()) => panic!("bounded queue accepted more than its configured capacity"),
        }
        assert!(budget.reserve(5).is_none());
        drop(receiver.try_recv().unwrap());
        assert!(budget.reserve(10).is_some());
    }

    #[test]
    fn if_runtime_fault_does_not_publish_partial_route_provenance() {
        let configuration = if_node::compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "$json.value"}]
        }))
        .unwrap();
        let mut envelopes = vec![
            GeneratedEnvelope {
                ordinal: 0,
                item: json!({"value": true}),
                logical_item: json!({"value": true}),
                logical_bytes: 16,
                provenance: json!({"ordinal": 0}),
            },
            GeneratedEnvelope {
                ordinal: 1,
                item: json!({"value": null}),
                logical_item: json!({"value": null}),
                logical_bytes: 16,
                provenance: json!({"ordinal": 1}),
            },
        ];
        let original = envelopes.clone();
        let error = configuration
            .route_batch(&mut envelopes, "if", "genesis")
            .unwrap_err();
        assert_eq!(error.code, "canopy.if.type");
        for (actual, expected) in envelopes.iter().zip(original.iter()) {
            assert_eq!(actual.item, expected.item);
            assert_eq!(actual.logical_item, expected.logical_item);
            assert_eq!(actual.provenance, expected.provenance);
        }
    }

    #[test]
    fn byte_budget_releases_capacity() {
        let budget = Arc::new(ByteBudget::new(10));
        let first = budget.reserve(8).unwrap();
        assert!(budget.reserve(3).is_none());
        drop(first);
        assert!(budget.reserve(10).is_some());
    }

    #[test]
    fn cursor_rejects_other_shapes() {
        let value = cursor("run-1", 2, "boot-1", 3);
        let parsed = parse_cursor(&value).unwrap();
        assert_eq!(parsed.run_id, "run-1");
        assert_eq!(parsed.durable_sequence, 2);
        assert_eq!(parsed.live_sequence, 3);
        assert!(parse_cursor("not-a-cursor").is_none());
    }

    #[test]
    fn rejects_obvious_secret_fields_from_retained_trace() {
        assert!(reject_sensitive_keys(&json!({"password": "do-not-store"})).is_err());
        assert!(reject_sensitive_keys(&json!({"service_access_token": "never"})).is_err());
        assert!(
            reject_sensitive_keys(&json!({"safe": {"manual": true, "token_count": 3}})).is_ok()
        );
    }

    #[test]
    fn replay_larger_than_one_mailbox_becomes_gap_and_resync() {
        let hub = LiveHub::new();
        let run_id = "run-mailbox-bound";
        for sequence in 0..=SUBSCRIBER_QUEUE_COUNT {
            hub.emit(
                run_id,
                1,
                "live",
                json!({
                    "state": "running",
                    "durability": "speculative",
                    "sequence": sequence
                }),
                false,
            );
        }
        let snapshot = RunView {
            schema: RUN_SCHEMA.into(),
            run_id: run_id.into(),
            run_request_id: "request-mailbox-bound".into(),
            workflow_id: "workflow-mailbox-bound".into(),
            publication_event_id: "event-mailbox-bound".into(),
            revision_id: "revision-mailbox-bound".into(),
            revision_digest: "sha256:revision".into(),
            plan_id: "plan-mailbox-bound".into(),
            plan_digest: "sha256:plan".into(),
            durable: DurableProgress {
                state: "queued".into(),
                checkpoint_sequence: 1,
                logical_order: 0,
                terminal: false,
                updated_at: 0,
            },
            live: None,
            correctness: CorrectnessView {
                canonicalization: CANONICALIZATION.into(),
                algorithm: DIGEST_ALGORITHM.into(),
                digest: None,
                complete: false,
                attempted: 0,
                succeeded: 0,
                cancelled: 0,
                failed: 0,
                output_count: 0,
            },
            generation: None,
            admitted_at: 0,
            started_at: None,
            terminal_at: None,
            queue_profile: queue_profile(),
        };
        let initial = cursor(run_id, 1, &hub.boot_epoch, 0);
        let mut receiver = hub.subscribe(run_id, Some(&initial), &snapshot).unwrap();
        assert_eq!(receiver.try_recv().unwrap().event, "gap");
        assert_eq!(receiver.try_recv().unwrap().event, "resync");
    }

    #[test]
    fn edit_fields_batch_preserves_artifact_data_and_numeric_item_linking() {
        let configuration = edit_fields::compile_configuration(&json!({
            "mode": "merge",
            "assignments": [
                {"path": ["eco"], "kind": "fixed", "value": true},
                {"path": ["label"], "kind": "expression", "source": "\"eco-\" + $json.index"}
            ]
        }))
        .unwrap();
        let artifact = ArtifactReference {
            artifact_id: "artifact-test".into(),
            format: "canopy.artifact+xchacha20poly1305/v1alpha1".into(),
            media_type: "application/json".into(),
            logical_bytes: 32,
            content_digest_algorithm: "blake3-256".into(),
        };
        let mut envelopes = vec![GeneratedEnvelope {
            ordinal: 12,
            item: json!({"index":12,"value":7,"data":{"$artifact":artifact}}),
            logical_item: json!({"index":12,"value":7,"data":{"payload":"eco"}}),
            logical_bytes: 64,
            provenance: json!({"ordinal":12}),
        }];
        let applied = apply_edit_fields_batch(
            &mut envelopes,
            Some(&configuration),
            Some("edit-fields"),
            Some(&artifact),
            0,
            0,
            "genesis",
        )
        .unwrap();
        assert_eq!(applied.transformed_count, 1);
        assert!(applied.transformed_logical_bytes > 0);
        assert!(applied.transformed_stream_digest.starts_with("sha256:"));
        assert_eq!(envelopes[0].item["eco"], true);
        assert_eq!(envelopes[0].item["label"], "eco-12");
        assert_eq!(
            envelopes[0].item["data"]["$artifact"]["artifact_id"],
            "artifact-test"
        );
        assert_eq!(
            envelopes[0].provenance["edit_fields_node_instance_id"],
            "edit-fields"
        );
        assert_eq!(envelopes[0].provenance["input_ordinal"], 12);
    }

    #[test]
    fn edit_fields_replace_preserves_artifact_data() {
        let configuration = edit_fields::compile_configuration(&json!({
            "mode": "replace",
            "assignments": [
                {"path": ["label"], "kind": "fixed", "value": "eco-replaced"}
            ]
        }))
        .unwrap();
        let artifact = ArtifactReference {
            artifact_id: "artifact-replace".into(),
            format: "canopy.artifact+xchacha20poly1305/v1alpha1".into(),
            media_type: "application/json".into(),
            logical_bytes: 32,
            content_digest_algorithm: "blake3-256".into(),
        };
        let mut envelopes = vec![GeneratedEnvelope {
            ordinal: 4,
            item: json!({"index":4,"value":7,"data":{"$artifact":artifact}}),
            logical_item: json!({"index":4,"value":7,"data":{"payload":"eco"}}),
            logical_bytes: 64,
            provenance: json!({"ordinal":4}),
        }];
        apply_edit_fields_batch(
            &mut envelopes,
            Some(&configuration),
            Some("edit-fields"),
            Some(&artifact),
            0,
            0,
            "genesis",
        )
        .unwrap();
        assert_eq!(envelopes[0].item["label"], "eco-replaced");
        assert_eq!(
            envelopes[0].item["data"]["$artifact"]["artifact_id"],
            "artifact-replace"
        );
    }

    #[test]
    fn transformed_logical_items_are_the_if_input() {
        let edit_configuration = edit_fields::compile_configuration(&json!({
            "mode": "merge",
            "assignments": [
                {"path": ["eco"], "kind": "fixed", "value": true}
            ]
        }))
        .unwrap();
        let if_configuration = if_node::compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "$json.eco === true"}]
        }))
        .unwrap();
        let mut envelopes = vec![GeneratedEnvelope {
            ordinal: 0,
            item: json!({"index":0,"value":1,"data":{}}),
            logical_item: json!({"index":0,"value":1,"data":{}}),
            logical_bytes: 32,
            provenance: json!({"ordinal":0}),
        }];
        apply_edit_fields_batch(
            &mut envelopes,
            Some(&edit_configuration),
            Some("edit-fields"),
            None,
            0,
            0,
            "genesis",
        )
        .unwrap();
        let branch = if_configuration
            .route_batch(&mut envelopes, "if-node", "genesis")
            .unwrap();
        assert_eq!(branch.true_count, 1);
        assert_eq!(branch.false_count, 0);
        assert_eq!(envelopes[0].logical_item["eco"], true);
        assert_eq!(envelopes[0].provenance["if_output_port"], "true");
    }

    #[test]
    fn edit_fields_batch_does_not_publish_partial_transform_state_on_error() {
        let configuration = edit_fields::compile_configuration(&json!({
            "mode": "merge",
            "assignments": [
                {"path": ["doubled"], "kind": "expression", "source": "$json.value * 2"}
            ]
        }))
        .unwrap();
        let mut envelopes = vec![
            GeneratedEnvelope {
                ordinal: 0,
                item: json!({"index":0,"value":2,"data":{}}),
                logical_item: json!({"index":0,"value":2,"data":{}}),
                logical_bytes: 32,
                provenance: json!({"ordinal":0}),
            },
            GeneratedEnvelope {
                ordinal: 1,
                item: json!({"index":1,"value":"bad","data":{}}),
                logical_item: json!({"index":1,"value":"bad","data":{}}),
                logical_bytes: 32,
                provenance: json!({"ordinal":1}),
            },
        ];
        let original = envelopes.clone();
        let error = apply_edit_fields_batch(
            &mut envelopes,
            Some(&configuration),
            Some("edit-fields"),
            None,
            7,
            123,
            "prior-digest",
        )
        .unwrap_err();
        assert_eq!(error.code, "canopy.expression.type");
        assert_eq!(envelopes[0].item, original[0].item);
        assert_eq!(envelopes[0].provenance, original[0].provenance);
        assert_eq!(envelopes[1].item, original[1].item);
        assert_eq!(envelopes[1].provenance, original[1].provenance);
    }
}
