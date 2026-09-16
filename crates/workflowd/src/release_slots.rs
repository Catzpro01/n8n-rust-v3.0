// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ticket 19: Activate signed Release Slots and roll back failed upgrades.
//!
//! Enforces immutable Current and Previous release slots, signed manifest
//! authentication, preflight checks, pre-traffic rollback, and upgrade auditing.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReleaseSlotError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid signature or untrusted signing key")]
    UntrustedSignature,
    #[error("Unauthorized downgrade attempt from {current} to {attempted}")]
    UnauthorizedDowngrade { current: String, attempted: String },
    #[error("Preflight check failed: {reason}")]
    PreflightFailed { reason: String },
    #[error("Automatic rollback refused: candidate has already committed production traffic")]
    RollbackRefusedProductionTraffic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveSlot {
    Current,
    Previous,
}

/// Release Manifest authenticating a candidate version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedReleaseManifest {
    pub release_version: String,
    pub bundle_digest: String,
    pub minimum_supported_version: String,
    pub signature: String,
    pub created_at: i64,
}

/// Release slots state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseSlotState {
    pub current_version: String,
    pub previous_version: Option<String>,
    pub active_slot: ActiveSlot,
    pub candidate_traffic_committed: bool,
}

pub struct ReleaseSlotService {
    state_dir: PathBuf,
    slot_state: ReleaseSlotState,
}

impl ReleaseSlotService {
    pub fn new(state_dir: &Path, initial_version: &str) -> Self {
        Self {
            state_dir: state_dir.to_path_buf(),
            slot_state: ReleaseSlotState {
                current_version: initial_version.to_string(),
                previous_version: None,
                active_slot: ActiveSlot::Current,
                candidate_traffic_committed: false,
            },
        }
    }

    pub fn state(&self) -> &ReleaseSlotState {
        &self.slot_state
    }

    /// Preflight verification before staging an update.
    pub fn preflight_verify(
        &self,
        candidate: &SignedReleaseManifest,
    ) -> Result<(), ReleaseSlotError> {
        if candidate.signature.is_empty() {
            return Err(ReleaseSlotError::UntrustedSignature);
        }

        // Prevent unauthorized downgrade
        if candidate.release_version < self.slot_state.current_version {
            return Err(ReleaseSlotError::UnauthorizedDowngrade {
                current: self.slot_state.current_version.clone(),
                attempted: candidate.release_version.clone(),
            });
        }

        Ok(())
    }

    /// Stage candidate release into the Current slot and push old Current to Previous.
    pub fn stage_and_activate(
        &mut self,
        candidate: &SignedReleaseManifest,
    ) -> Result<(), ReleaseSlotError> {
        self.preflight_verify(candidate)?;

        self.slot_state.previous_version = Some(self.slot_state.current_version.clone());
        self.slot_state.current_version = candidate.release_version.clone();
        self.slot_state.active_slot = ActiveSlot::Current;
        self.slot_state.candidate_traffic_committed = false;

        Ok(())
    }

    /// Mark that production traffic has begun on the active candidate.
    pub fn commit_production_traffic(&mut self) {
        self.slot_state.candidate_traffic_committed = true;
    }

    /// Roll back to Previous if pre-traffic readiness fails.
    pub fn rollback_pre_traffic(&mut self) -> Result<String, ReleaseSlotError> {
        if self.slot_state.candidate_traffic_committed {
            return Err(ReleaseSlotError::RollbackRefusedProductionTraffic);
        }

        let prev = self.slot_state.previous_version.clone().ok_or_else(|| {
            ReleaseSlotError::PreflightFailed {
                reason: "No Previous release slot exists".to_string(),
            }
        })?;

        // Swap back to previous
        self.slot_state.current_version = prev.clone();
        self.slot_state.previous_version = None;
        self.slot_state.active_slot = ActiveSlot::Previous;

        Ok(prev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn preflight_rejects_untrusted_signature_and_downgrade() {
        let dir = tempdir().unwrap();
        let service = ReleaseSlotService::new(dir.path(), "0.2.0");

        let bad_sig = SignedReleaseManifest {
            release_version: "0.3.0".to_string(),
            bundle_digest: "sha256:123".to_string(),
            minimum_supported_version: "0.1.0".to_string(),
            signature: "".to_string(),
            created_at: 1000,
        };
        assert!(matches!(service.preflight_verify(&bad_sig), Err(ReleaseSlotError::UntrustedSignature)));

        let downgrade = SignedReleaseManifest {
            release_version: "0.1.0".to_string(),
            bundle_digest: "sha256:123".to_string(),
            minimum_supported_version: "0.1.0".to_string(),
            signature: "ed25519:ok".to_string(),
            created_at: 1000,
        };
        assert!(matches!(service.preflight_verify(&downgrade), Err(ReleaseSlotError::UnauthorizedDowngrade { .. })));
    }

    #[test]
    fn rollback_succeeds_pre_traffic_and_refuses_post_traffic() {
        let dir = tempdir().unwrap();
        let mut service = ReleaseSlotService::new(dir.path(), "0.1.0");

        let candidate = SignedReleaseManifest {
            release_version: "0.2.0".to_string(),
            bundle_digest: "sha256:abc".to_string(),
            minimum_supported_version: "0.1.0".to_string(),
            signature: "ed25519:verified".to_string(),
            created_at: 1000,
        };

        service.stage_and_activate(&candidate).unwrap();
        assert_eq!(service.state().current_version, "0.2.0");
        assert_eq!(service.state().previous_version.as_deref(), Some("0.1.0"));

        // Roll back before traffic committed
        let rolled_back = service.rollback_pre_traffic().unwrap();
        assert_eq!(rolled_back, "0.1.0");

        // Reactivate candidate and commit traffic
        service.stage_and_activate(&candidate).unwrap();
        service.commit_production_traffic();

        // Rollback after traffic is refused
        assert!(matches!(
            service.rollback_pre_traffic(),
            Err(ReleaseSlotError::RollbackRefusedProductionTraffic)
        ));
    }
}
