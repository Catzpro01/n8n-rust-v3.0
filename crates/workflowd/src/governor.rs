// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adaptive Resource Governor.
//!
//! The governor samples cgroup v2 counters at a fixed cadence and turns them
//! into concrete scheduling decisions the rest of the engine can obey without
//! reading sysfs or running ad-hoc heuristics:
//!
//! - **Concurrency budget** for native Run activation slots (`hot_run_slots`).
//! - **Batch sizes** for envelope micro-batches, checkpoint intervals, and
//!   artifact spill thresholds.
//! - **Admission pressure** signals: external ingress should be rejected with
//!   `Retry-After` guidance before durable admission so that overload is
//!   visible to the caller instead of becoming silent memory growth.
//! - **Pressure signals** for CPU/memory/IO/disk, converted to boolean
//!   reduce-concurrency, spill-now, suspend-now actions.
//! - **Weighted fairness**: small interactive Runs are chosen before large
//!   batch Runs when pressure is non-zero, while preserving FIFO order when
//!   there is spare capacity.
//! - **Control-plane reserve**: status, health, trace, and cancellation paths
//!   keep a bounded share of queue budget so they cannot be starved by data
//!   plane envelopes.
//!
//! The governor never changes Logical Order, correctness digests, plans, or
//! published revisions. It only shapes *how much* work is dispatched in each
//! tick and when new work is admitted.

use crate::cgroup::{self, ResourceSample, DISK_SAMPLE_INTERVAL, SAMPLE_INTERVAL};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Absolute floor for hot Run slots so that status, cancellation, and at least
/// one hot Run remain dispatchable even on a very constrained profile.
pub const MIN_HOT_RUN_SLOTS: usize = 1;

/// Absolute ceiling for hot Run slots on an unbounded / many-core host. The
/// governor never exceeds this even if the cgroup advertises many CPUs,
/// because each additional slot pays a queue-synchronization and checkpoint
/// overhead that dominates after a handful of concurrent native Runs.
pub const MAX_HOT_RUN_SLOTS: usize = 4;

/// Ceiling for hot Run slots reserved for the control plane (health, status,
/// cancellation, trace fan-out). Data-plane Runs compete for the remaining
/// slots.
pub const CONTROL_PLANE_RESERVED_SLOTS: usize = 1;

/// Retry-After window returned to callers when admission is shed, in seconds.
pub const ADMISSION_RETRY_AFTER_SECONDS: u64 = 2;

/// Pressure thresholds (0-100 scale matching PSI avg10 percentages). They are
/// intentionally conservative: the governor sheds load *before* the kernel
/// invokes OOM or throttles the daemon into unresponsiveness.
const CPU_PRESSURE_REDUCE: f64 = 25.0;
const CPU_PRESSURE_SUSPEND: f64 = 70.0;
const MEMORY_PRESSURE_REDUCE: f64 = 5.0;
const MEMORY_HIGH_WATERMARK: f64 = 0.80;
const MEMORY_SUSPEND_WATERMARK: f64 = 0.92;
const IO_PRESSURE_REDUCE: f64 = 10.0;

#[derive(Debug, Clone, Serialize)]
pub struct GovernorDecision {
    /// When the last sample was taken (monotonic Instant serialised as millis
    /// since governor construction).
    pub sampled_at_millis: u64,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub io: IoSnapshot,
    pub pids: PidsSnapshot,
    pub disk: DiskSnapshot,
    pub pressure: PressureSnapshot,
    pub accelerator: AcceleratorSnapshot,
    pub allowed_hot_run_slots: usize,
    pub envelope_micro_batch_count: usize,
    pub max_ready_queue_count: usize,
    pub max_ready_queue_bytes: usize,
    pub spill_to_artifact_threshold_bytes: usize,
    pub checkpoint_max_outcomes: usize,
    pub checkpoint_max_bytes: usize,
    pub admission_open: bool,
    pub admission_retry_after_seconds: u64,
    pub reduce_concurrency: bool,
    pub durable_suspend_pressure: bool,
    pub suspend_reason: Option<&'static str>,
    pub fairness_class: FairnessClass,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuSnapshot {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_cores: Option<f64>,
    pub throttled_pct: f64,
    pub throttled_usec_delta: u64,
    pub usage_pct_of_quota: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    pub available: bool,
    pub current_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
    pub usage_pct_of_limit: f64,
    pub high_events_delta: u64,
    pub ooms_delta: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IoSnapshot {
    pub available: bool,
    pub rbytes_delta: u64,
    pub wbytes_delta: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PidsSnapshot {
    pub available: bool,
    pub current: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    pub usage_pct_of_limit: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskSnapshot {
    pub available: bool,
    pub available_bytes: u64,
    pub reserved_bytes: u64,
    pub below_reserve: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PressureSnapshot {
    pub cpu_some_avg10: f64,
    pub memory_some_avg10: f64,
    pub io_some_avg10: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AcceleratorSnapshot {
    pub policy: &'static str,
    pub eligible: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FairnessClass {
    /// No pressure observed; admit in FIFO order.
    Fifo,
    /// Pressure observed; prefer small/interactive Runs (low expected cost)
    /// before heavyweight Eco/Generate Runs to bound interactive latency.
    WeightedInteractiveFirst,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunWorkClass {
    /// Small Runs with at most a handful of activations (Manual Trigger only,
    /// single activation, small output).
    Interactive,
    /// Heavyweight Runs that stream many items (Generate / Eco 100K).
    Batch,
}

pub struct Governor {
    cgroup_dir: Option<PathBuf>,
    state_dir: PathBuf,
    reserved_disk_bytes: u64,
    inner: Mutex<GovernorInner>,
}

struct GovernorInner {
    last_sample: ResourceSample,
    last_sample_instant: Instant,
    last_disk_sample_instant: Instant,
    previous: PreviousCounters,
    decision: GovernorDecision,
    start_instant: Instant,
}

#[derive(Debug, Clone, Default)]
struct PreviousCounters {
    cpu_throttled_usec: u64,
    cpu_usage_usec: u64,
    memory_high_events: u64,
    memory_ooms: u64,
    io_rbytes: u64,
    io_wbytes: u64,
    initialized: bool,
}

impl Governor {
    pub fn new(
        cgroup_dir: Option<PathBuf>,
        state_dir: PathBuf,
        reserved_disk_bytes: u64,
    ) -> Arc<Self> {
        let now = Instant::now();
        let sample = cgroup::sample(cgroup_dir.as_deref(), Some(&state_dir), reserved_disk_bytes);
        let previous = PreviousCounters::from_sample(&sample);
        let decision = Self::decide(&sample, &previous, now, now);
        let inner = GovernorInner {
            last_sample: sample,
            last_sample_instant: now,
            last_disk_sample_instant: now,
            previous,
            decision,
            start_instant: now,
        };
        Arc::new(Self {
            cgroup_dir,
            state_dir,
            reserved_disk_bytes,
            inner: Mutex::new(inner),
        })
    }

    /// Return the latest decision, refreshing samples if the sampling interval
    /// has elapsed. This is a cheap lock; sampling is O(several small file
    /// reads) and skipped when not enough wall time has passed.
    pub fn decision(&self) -> GovernorDecision {
        let mut inner = self.inner.lock().expect("governor poisoned");
        let now = Instant::now();
        let needs_cpu_sample = now.duration_since(inner.last_sample_instant) >= SAMPLE_INTERVAL;
        let needs_disk_sample =
            now.duration_since(inner.last_disk_sample_instant) >= DISK_SAMPLE_INTERVAL;
        if !needs_cpu_sample && !needs_disk_sample {
            return inner.decision.clone();
        }
        let mut sample = if needs_cpu_sample {
            cgroup::sample(
                self.cgroup_dir.as_deref(),
                Some(&self.state_dir),
                self.reserved_disk_bytes,
            )
        } else {
            inner.last_sample.clone()
        };
        if needs_disk_sample && !needs_cpu_sample {
            sample.disk = cgroup::sample(
                self.cgroup_dir.as_deref(),
                Some(&self.state_dir),
                self.reserved_disk_bytes,
            )
            .disk;
        }
        let decision = Self::decide(&sample, &inner.previous, inner.start_instant, now);
        inner.previous = PreviousCounters::from_sample(&sample);
        inner.last_sample = sample;
        if needs_cpu_sample {
            inner.last_sample_instant = now;
        }
        if needs_disk_sample {
            inner.last_disk_sample_instant = now;
        }
        inner.decision = decision.clone();
        decision
    }

    /// Classify a Run for weighted-fair admission ordering.
    pub fn classify_run(
        generated_count: u64,
        invocation_bytes: usize,
        expected_generates_large_stream: bool,
    ) -> RunWorkClass {
        if expected_generates_large_stream && generated_count == 0 && invocation_bytes > 4 * 1024 {
            RunWorkClass::Batch
        } else if generated_count > 1024 {
            RunWorkClass::Batch
        } else {
            RunWorkClass::Interactive
        }
    }

    fn decide(
        sample: &ResourceSample,
        previous: &PreviousCounters,
        start: Instant,
        now: Instant,
    ) -> GovernorDecision {
        let elapsed_micros = now.duration_since(start).as_micros().max(1) as u64;
        // We don't track last wall delta explicitly; reconstruct it
        // conservatively as the cgroup sample interval.
        let wall_delta_micros = SAMPLE_INTERVAL.as_micros().max(1) as u64;

        // CPU view
        let quota_cores = sample.cpu.quota_cores;
        let cpu_throttled_delta = sample
            .cpu
            .throttled_usec
            .saturating_sub(previous.cpu_throttled_usec);
        let cpu_usage_delta = sample
            .cpu
            .usage_usec
            .saturating_sub(previous.cpu_usage_usec);
        // throttled_pct = percent of wall-time the kernel throttled us.
        let throttled_pct = if wall_delta_micros > 0 {
            (cpu_throttled_delta as f64 / wall_delta_micros as f64) * 100.0
        } else {
            0.0
        };
        let usage_pct_of_quota = match quota_cores {
            Some(cores) if cores > 0.0 && wall_delta_micros > 0 => {
                let quota_usec_delta = (cores * wall_delta_micros as f64).max(1.0) as u64;
                (cpu_usage_delta.min(quota_usec_delta) as f64 / quota_usec_delta as f64) * 100.0
            }
            _ => 0.0,
        };

        // Memory view
        let mem_limit = sample.memory.max_bytes.unwrap_or(0);
        let mem_used = sample.memory.current_bytes;
        let mem_pct = if mem_limit > 0 {
            (mem_used as f64 / mem_limit as f64) * 100.0
        } else {
            0.0
        };
        let memory_high_events_delta = sample
            .memory
            .high_events
            .saturating_sub(previous.memory_high_events);
        let ooms_delta = sample.memory.ooms.saturating_sub(previous.memory_ooms);

        // IO view
        let io_rbytes_delta = sample.io.rbytes.saturating_sub(previous.io_rbytes);
        let io_wbytes_delta = sample.io.wbytes.saturating_sub(previous.io_wbytes);

        // Pids view
        let pids_limit = sample.pids.max;
        let pids_pct = match pids_limit {
            Some(max) if max > 0 => (sample.pids.current as f64 / max as f64) * 100.0,
            _ => 0.0,
        };

        let cpu_pressure = sample.pressure.cpu.some_avg10;
        let mem_pressure = sample.pressure.memory.some_avg10;
        let io_pressure = sample.pressure.io.some_avg10;

        let reduce_cpu = throttled_pct > 20.0
            || usage_pct_of_quota > 85.0
            || cpu_pressure > CPU_PRESSURE_REDUCE;
        let suspend_cpu = throttled_pct > 60.0 || cpu_pressure > CPU_PRESSURE_SUSPEND;
        let reduce_memory = mem_pct > MEMORY_HIGH_WATERMARK * 100.0
            || memory_high_events_delta > 0
            || mem_pressure > MEMORY_PRESSURE_REDUCE;
        let suspend_memory = mem_pct > MEMORY_SUSPEND_WATERMARK * 100.0
            || ooms_delta > 0;
        let reduce_io = io_pressure > IO_PRESSURE_REDUCE;
        let disk_below_reserve = sample.disk.below_reserve;

        let reduce_concurrency = reduce_cpu || reduce_memory || reduce_io;
        let durable_suspend_pressure = suspend_cpu
            || suspend_memory
            || disk_below_reserve
            || sample.pressure.memory.full_avg10 > 50.0;

        let suspend_reason = if disk_below_reserve {
            Some("managed_disk_reserve_crossed")
        } else if suspend_memory {
            Some("memory_pressure_suspension")
        } else if suspend_cpu {
            Some("cpu_throttle_suspension")
        } else {
            None
        };

        // Choose hot run slots. At 0.5 CPU quotas we only want 1 hot slot; at
        // higher quotas grow up to MAX_HOT_RUN_SLOTS.
        let quota_slots = match quota_cores {
            Some(cores) if cores <= 0.5 => 1,
            Some(cores) if cores <= 1.0 => 1,
            Some(cores) if cores <= 2.0 => 2,
            Some(cores) => {
                // one slot per core, capped
                (cores as usize).min(MAX_HOT_RUN_SLOTS)
            }
            None => {
                // No explicit quota (host has all CPUs); default to the number
                // of cores but cap conservatively.
                std::thread::available_parallelism()
                    .map(|n| n.get().min(MAX_HOT_RUN_SLOTS))
                    .unwrap_or(2)
            }
        };
        let mut slots = quota_slots;
        if reduce_concurrency {
            slots = slots.saturating_sub(1);
        }
        if slots < MIN_HOT_RUN_SLOTS {
            slots = MIN_HOT_RUN_SLOTS;
        }
        // Reserve one slot for control-plane work (status, cancel, health).
        let control_reserved = CONTROL_PLANE_RESERVED_SLOTS.min(slots);
        let _data_plane_slots = slots.saturating_sub(control_reserved).max(1);

        // Tune micro-batch and checkpoint sizes to shed memory when pressured.
        let envelope_micro_batch_count = if reduce_memory { 16 } else { 64 };
        let checkpoint_max_outcomes = if reduce_memory { 256 } else { 1024 };
        let checkpoint_max_bytes = if reduce_memory {
            256 * 1024
        } else {
            1024 * 1024
        };
        let spill_threshold_bytes = if reduce_memory {
            64 * 1024
        } else {
            4 * 1024 * 1024
        };

        // Queue caps shrink under pressure so backpressure reaches callers
        // quickly instead of piling up in-process.
        let max_ready_queue_count = if reduce_concurrency { 256 } else { 1024 };
        let max_ready_queue_bytes = if reduce_memory {
            1024 * 1024
        } else {
            4 * 1024 * 1024
        };

        // Admission is closed to new external work when we would have to
        // durably suspend or when queue pressure is at the limit. Already
        // admitted Runs continue; cancellation/status/health always work.
        let admission_open = !durable_suspend_pressure && !reduce_concurrency;
        let admission_retry_after_seconds = if admission_open {
            0
        } else {
            ADMISSION_RETRY_AFTER_SECONDS
        };

        let fairness_class = if reduce_concurrency {
            FairnessClass::WeightedInteractiveFirst
        } else {
            FairnessClass::Fifo
        };

        GovernorDecision {
            sampled_at_millis: (elapsed_micros / 1000) as u64,
            cpu: CpuSnapshot {
                available: sample.cpu.available,
                quota_cores,
                throttled_pct,
                throttled_usec_delta: cpu_throttled_delta,
                usage_pct_of_quota,
            },
            memory: MemorySnapshot {
                available: sample.memory.available,
                current_bytes: mem_used,
                max_bytes: sample.memory.max_bytes,
                usage_pct_of_limit: mem_pct,
                high_events_delta: memory_high_events_delta,
                ooms_delta,
            },
            io: IoSnapshot {
                available: sample.io.available,
                rbytes_delta: io_rbytes_delta,
                wbytes_delta: io_wbytes_delta,
            },
            pids: PidsSnapshot {
                available: sample.pids.available,
                current: sample.pids.current,
                max: sample.pids.max,
                usage_pct_of_limit: pids_pct,
            },
            disk: DiskSnapshot {
                available: sample.disk.available,
                available_bytes: sample.disk.available_bytes,
                reserved_bytes: sample.disk.reserved_bytes,
                below_reserve: sample.disk.below_reserve,
            },
            pressure: PressureSnapshot {
                cpu_some_avg10: cpu_pressure,
                memory_some_avg10: mem_pressure,
                io_some_avg10: io_pressure,
            },
            accelerator: AcceleratorSnapshot {
                policy: "off",
                eligible: false,
            },
            allowed_hot_run_slots: slots,
            envelope_micro_batch_count,
            max_ready_queue_count,
            max_ready_queue_bytes,
            spill_to_artifact_threshold_bytes: spill_threshold_bytes,
            checkpoint_max_outcomes,
            checkpoint_max_bytes,
            admission_open,
            admission_retry_after_seconds,
            reduce_concurrency,
            durable_suspend_pressure,
            suspend_reason,
            fairness_class,
        }
    }
}

impl PreviousCounters {
    fn from_sample(sample: &ResourceSample) -> Self {
        PreviousCounters {
            cpu_throttled_usec: sample.cpu.throttled_usec,
            cpu_usage_usec: sample.cpu.usage_usec,
            memory_high_events: sample.memory.high_events,
            memory_ooms: sample.memory.ooms,
            io_rbytes: sample.io.rbytes,
            io_wbytes: sample.io.wbytes,
            initialized: true,
        }
    }
}

/// Return the documented Eco profile resource targets. Useful for tests and
/// the benchmark manifest writer.
pub fn eco_profile_targets() -> serde_json::Value {
    serde_json::json!({
        "cpu_quota_cores": 0.5,
        "memory_max_bytes": 500 * 1024 * 1024,
        "swap_max_bytes": 0,
        "managed_disk_reserve_bytes": cgroup::DEFAULT_MANAGED_DISK_RESERVE_BYTES,
        "objective": {
            "correctness_digest_stable": true,
            "logical_order_deterministic": true,
            "oom_kills_zero": true,
            "throttle_tolerance_pct": 80.0,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_sample() -> ResourceSample {
        ResourceSample::default()
    }

    fn sample_with_cpu_quota(quota: Option<f64>) -> ResourceSample {
        let mut sample = empty_sample();
        sample.cpu.available = true;
        sample.cpu.quota_cores = quota;
        sample.cpu.period_usec = Some(100_000);
        sample.memory.max_bytes = Some(500 * 1024 * 1024);
        sample.memory.current_bytes = 100 * 1024 * 1024;
        sample.disk.available = true;
        sample.disk.available_bytes = 20 * 1024 * 1024 * 1024;
        sample.disk.reserved_bytes = cgroup::DEFAULT_MANAGED_DISK_RESERVE_BYTES;
        sample
    }

    #[test]
    fn eco_half_core_picks_one_slot_and_admits() {
        let sample = sample_with_cpu_quota(Some(0.5));
        let now = Instant::now();
        let previous = PreviousCounters::from_sample(&sample);
        let decision = Governor::decide(&sample, &previous, now, now);
        assert_eq!(decision.allowed_hot_run_slots, 1);
        assert!(decision.admission_open);
        assert!(!decision.durable_suspend_pressure);
        assert_eq!(decision.envelope_micro_batch_count, 64);
    }

    #[test]
    fn high_memory_pressure_reduces_batches_and_suspends() {
        let mut sample = sample_with_cpu_quota(Some(2.0));
        sample.memory.current_bytes = 480 * 1024 * 1024; // ~96% of 500 MiB
        let now = Instant::now();
        let previous = PreviousCounters::from_sample(&sample);
        let decision = Governor::decide(&sample, &previous, now, now);
        assert!(decision.durable_suspend_pressure);
        assert!(!decision.admission_open);
        assert_eq!(decision.suspend_reason, Some("memory_pressure_suspension"));
        assert_eq!(decision.envelope_micro_batch_count, 16);
        assert_eq!(decision.spill_to_artifact_threshold_bytes, 64 * 1024);
    }

    #[test]
    fn disk_reserve_crossed_suspends() {
        let mut sample = sample_with_cpu_quota(Some(2.0));
        sample.disk.below_reserve = true;
        sample.disk.available_bytes = 1024;
        let now = Instant::now();
        let previous = PreviousCounters::from_sample(&sample);
        let decision = Governor::decide(&sample, &previous, now, now);
        assert!(decision.durable_suspend_pressure);
        assert_eq!(decision.suspend_reason, Some("managed_disk_reserve_crossed"));
        assert!(!decision.admission_open);
    }

    #[test]
    fn two_cores_allows_two_slots() {
        let sample = sample_with_cpu_quota(Some(2.0));
        let now = Instant::now();
        let previous = PreviousCounters::from_sample(&sample);
        let decision = Governor::decide(&sample, &previous, now, now);
        assert_eq!(decision.allowed_hot_run_slots, 2);
        assert!(decision.admission_open);
    }

    #[test]
    fn accelerator_policy_is_off_by_default() {
        let sample = sample_with_cpu_quota(Some(1.0));
        let now = Instant::now();
        let previous = PreviousCounters::from_sample(&sample);
        let decision = Governor::decide(&sample, &previous, now, now);
        assert_eq!(decision.accelerator.policy, "off");
        assert!(!decision.accelerator.eligible);
    }

    #[test]
    fn classify_run_distinguishes_interactive_and_batch() {
        assert_eq!(
            Governor::classify_run(0, 128, false),
            RunWorkClass::Interactive
        );
        assert_eq!(
            Governor::classify_run(0, 32 * 1024, true),
            RunWorkClass::Batch
        );
        assert_eq!(
            Governor::classify_run(2048, 0, false),
            RunWorkClass::Batch
        );
    }
}
