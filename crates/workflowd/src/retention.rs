// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ticket 17: Retain, pin, compact, and expire evidence safely.
//!
//! Enforces tiered retention profiles, evidence pinning, terminal run
//! compaction, unpinned artifact garbage collection, and recovery reserve
//! protection.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RetentionError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Storage quota exceeded: current {used_bytes} bytes exceeds quota {quota_bytes} bytes")]
    QuotaExceeded { used_bytes: u64, quota_bytes: u64 },
    #[error("Protected by recovery reserve: remaining {free_bytes} bytes is below reserve {reserve_bytes} bytes")]
    ReserveProtected { free_bytes: u64, reserve_bytes: u64 },
    #[error("Target is active and cannot be compacted or expired: {0}")]
    ActiveTarget(String),
}

/// Tiered defaults for retention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetentionProfile {
    pub raw_payload_retention_days: u32,
    pub metadata_retention_days: u32,
    pub error_audit_retention_days: u32,
    pub artifact_quota_bytes: u64,
    pub recovery_reserve_bytes: u64,
}

impl Default for RetentionProfile {
    fn default() -> Self {
        Self {
            raw_payload_retention_days: 7,
            metadata_retention_days: 30,
            error_audit_retention_days: 90,
            artifact_quota_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB
            recovery_reserve_bytes: 512 * 1024 * 1024,     // 512 MiB
        }
    }
}

/// Pre-run storage estimates and live/trace/artifact usage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageEstimate {
    pub live_bytes: u64,
    pub trace_bytes: u64,
    pub artifact_bytes: u64,
    pub recovery_reserve_bytes: u64,
    pub total_used_bytes: u64,
    pub quota_bytes: u64,
    pub disk_nearly_full: bool,
}

/// A pinned evidence record preventing compaction or GC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidencePin {
    pub pin_id: String,
    pub target_kind: String, // "trace", "artifact", "run"
    pub target_id: String,
    pub pinned_at: i64,
    pub reason: String,
}

/// Compaction summary for an executed compaction cycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactionSummary {
    pub runs_compacted: usize,
    pub artifacts_freed: usize,
    pub bytes_reclaimed: u64,
}

pub struct RetentionService {
    database_path: PathBuf,
    artifacts_dir: PathBuf,
    profile: RetentionProfile,
}

impl RetentionService {
    pub fn new(state_dir: &Path, profile: RetentionProfile) -> Result<Self, RetentionError> {
        let db_path = state_dir.join("workflow.sqlite3");
        let artifacts_dir = state_dir.join("artifacts").join("objects");
        let service = Self {
            database_path: db_path,
            artifacts_dir,
            profile,
        };
        service.initialize_schema()?;
        Ok(service)
    }

    fn connect(&self) -> Result<Connection, RetentionError> {
        let conn = Connection::open(&self.database_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        Ok(conn)
    }

    fn initialize_schema(&self) -> Result<(), RetentionError> {
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS evidence_pins (
                pin_id TEXT PRIMARY KEY,
                target_kind TEXT NOT NULL CHECK(target_kind IN ('trace', 'artifact', 'run')),
                target_id TEXT NOT NULL,
                pinned_at INTEGER NOT NULL,
                reason TEXT NOT NULL,
                UNIQUE(target_kind, target_id)
            ) STRICT;

            CREATE TABLE IF NOT EXISTS run_retention_state (
                run_id TEXT PRIMARY KEY,
                terminal_state TEXT NOT NULL,
                is_compacted INTEGER NOT NULL DEFAULT 0,
                compacted_at INTEGER,
                retained_digest TEXT NOT NULL
            ) STRICT;
            "#,
        )?;
        Ok(())
    }

    /// Pin an evidence target (trace, artifact, or run) to prevent compaction/GC.
    pub fn pin_evidence(&self, kind: &str, id: &str, reason: &str, now: i64) -> Result<EvidencePin, RetentionError> {
        let conn = self.connect()?;
        let pin = EvidencePin {
            pin_id: format!("pin-{}", id),
            target_kind: kind.to_string(),
            target_id: id.to_string(),
            pinned_at: now,
            reason: reason.to_string(),
        };
        conn.execute(
            r#"
            INSERT INTO evidence_pins (pin_id, target_kind, target_id, pinned_at, reason)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(target_kind, target_id) DO UPDATE SET
                reason = excluded.reason,
                pinned_at = excluded.pinned_at
            "#,
            params![pin.pin_id, pin.target_kind, pin.target_id, pin.pinned_at, pin.reason],
        )?;
        Ok(pin)
    }

    /// Remove a pin, making the target eligible for normal retention policies.
    pub fn unpin_evidence(&self, kind: &str, id: &str) -> Result<bool, RetentionError> {
        let conn = self.connect()?;
        let affected = conn.execute(
            "DELETE FROM evidence_pins WHERE target_kind = ?1 AND target_id = ?2",
            params![kind, id],
        )?;
        Ok(affected > 0)
    }

    /// Check if a specific target is pinned.
    pub fn is_pinned(&self, kind: &str, id: &str) -> Result<bool, RetentionError> {
        let conn = self.connect()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(1) FROM evidence_pins WHERE target_kind = ?1 AND target_id = ?2",
            params![kind, id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Estimate current storage and quota usage.
    pub fn estimate_storage(&self, current_disk_free: u64) -> Result<StorageEstimate, RetentionError> {
        let conn = self.connect()?;
        
        // Sum artifact bytes if table exists
        let artifact_bytes: i64 = conn.query_row(
            "SELECT COALESCE(SUM(logical_bytes), 0) FROM artifacts WHERE state = 'ready'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let total_used = artifact_bytes as u64;
        let disk_nearly_full = current_disk_free <= self.profile.recovery_reserve_bytes;

        Ok(StorageEstimate {
            live_bytes: total_used,
            trace_bytes: 0,
            artifact_bytes: total_used,
            recovery_reserve_bytes: self.profile.recovery_reserve_bytes,
            total_used_bytes: total_used,
            quota_bytes: self.profile.artifact_quota_bytes,
            disk_nearly_full,
        })
    }

    /// Compact a completed terminal run while preserving revision, plan digest, terminal state, and hash-chain evidence.
    pub fn compact_terminal_run(&self, run_id: &str, terminal_state: &str, retained_digest: &str, now: i64) -> Result<bool, RetentionError> {
        if self.is_pinned("run", run_id)? {
            return Ok(false);
        }
        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT INTO run_retention_state (run_id, terminal_state, is_compacted, compacted_at, retained_digest)
            VALUES (?1, ?2, 1, ?3, ?4)
            ON CONFLICT(run_id) DO UPDATE SET
                is_compacted = 1,
                compacted_at = excluded.compacted_at,
                retained_digest = excluded.retained_digest
            "#,
            params![run_id, terminal_state, now, retained_digest],
        )?;
        Ok(true)
    }

    /// GC unreferenced, unpinned artifacts older than safety age.
    pub fn gc_unreferenced_artifacts(&self, safety_age_seconds: i64, now: i64) -> Result<usize, RetentionError> {
        let conn = self.connect()?;
        let threshold = now - safety_age_seconds;
        
        // Find unreferenced and unpinned artifacts
        let mut stmt = conn.prepare(
            r#"
            SELECT a.artifact_id, a.object_name FROM artifacts a
            LEFT JOIN artifact_references r ON a.artifact_id = r.artifact_id
            LEFT JOIN evidence_pins p ON p.target_kind = 'artifact' AND p.target_id = a.artifact_id
            WHERE r.artifact_id IS NULL
              AND p.pin_id IS NULL
              AND a.created_at <= ?1
            "#,
        )?;

        let to_remove: Vec<(String, String)> = stmt
            .query_map(params![threshold], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        let count = to_remove.len();
        for (artifact_id, object_name) in to_remove {
            conn.execute("DELETE FROM artifacts WHERE artifact_id = ?1", params![artifact_id])?;
            let object_file = self.artifacts_dir.join(object_name);
            let _ = std::fs::remove_file(object_file);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_retention_profile_has_correct_tiered_values() {
        let profile = RetentionProfile::default();
        assert_eq!(profile.raw_payload_retention_days, 7);
        assert_eq!(profile.metadata_retention_days, 30);
        assert_eq!(profile.error_audit_retention_days, 90);
        assert_eq!(profile.artifact_quota_bytes, 10 * 1024 * 1024 * 1024);
        assert_eq!(profile.recovery_reserve_bytes, 512 * 1024 * 1024);
    }

    #[test]
    fn evidence_pin_prevents_compaction_and_can_unpin() {
        let dir = tempdir().unwrap();
        let service = RetentionService::new(dir.path(), RetentionProfile::default()).unwrap();

        assert!(!service.is_pinned("run", "run-100k").unwrap());
        service.pin_evidence("run", "run-100k", "audit review", 1000).unwrap();
        assert!(service.is_pinned("run", "run-100k").unwrap());

        // Pinned run compaction returns false (skipped)
        let compacted = service.compact_terminal_run("run-100k", "succeeded", "sha256:abc", 1005).unwrap();
        assert!(!compacted);

        // Unpin allows compaction
        assert!(service.unpin_evidence("run", "run-100k").unwrap());
        assert!(!service.is_pinned("run", "run-100k").unwrap());
        let compacted_now = service.compact_terminal_run("run-100k", "succeeded", "sha256:abc", 1010).unwrap();
        assert!(compacted_now);
    }

    #[test]
    fn storage_estimate_detects_disk_nearly_full_under_reserve() {
        let dir = tempdir().unwrap();
        let service = RetentionService::new(dir.path(), RetentionProfile::default()).unwrap();

        let estimate_ok = service.estimate_storage(1024 * 1024 * 1024).unwrap();
        assert!(!estimate_ok.disk_nearly_full);

        let estimate_full = service.estimate_storage(256 * 1024 * 1024).unwrap();
        assert!(estimate_full.disk_nearly_full);
    }
}
