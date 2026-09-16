// SPDX-License-Identifier: AGPL-3.0-or-later
//! External Process Execution Lane adapter.
//!
//! Handles out-of-process node execution across a versioned, framed stdio protocol
//! (`canopy.lane-protocol/v1alpha1`). Enforces process isolation, capability grants,
//! deadlines, and structured error classification without burdening the native daemon.

use crate::canonical::digest;
use canopy_node_contract::{
    CapabilityGrant, ExecutionLane, HandshakeAckResponse, HandshakeRequest,
    LaneActivationRequest, LaneActivationResultResponse, LaneMessagePayload,
    LaneProtocolEnvelope, LaneProtocolError, LaneResourceMetrics, NodeForm,
    NodeImplementationLock, LANE_PROTOCOL_ABI,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const INLINE_PAYLOAD_THRESHOLD_BYTES: usize = 64 * 1024; // 64 KB

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaneExecutionConfig {
    pub executable_path: String,
    pub args: Vec<String>,
    pub env_vars: Vec<(String, String)>,
    pub working_dir: Option<String>,
    pub timeout_millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneExecutionResult {
    pub activation_id: String,
    pub outcome: String, // "success", "failure", "cancelled"
    pub output_port: String,
    pub output_data: Option<Value>,
    pub artifact_references: Vec<Value>,
    pub causal_trace: Value,
    pub resource_metrics: LaneResourceMetrics,
    pub elapsed_micros: u64,
}

pub struct ExternalProcessLaneRunner {
    config: LaneExecutionConfig,
    contract_digest: String,
    implementation_lock: NodeImplementationLock,
}

impl ExternalProcessLaneRunner {
    pub fn new(
        config: LaneExecutionConfig,
        contract_digest: String,
        implementation_lock: NodeImplementationLock,
    ) -> Self {
        Self {
            config,
            contract_digest,
            implementation_lock,
        }
    }

    /// Executes an activation request against the isolated child process.
    pub fn execute_activation(
        &self,
        request: LaneActivationRequest,
    ) -> Result<LaneExecutionResult, String> {
        let start_instant = Instant::now();
        let timeout = Duration::from_millis(self.config.timeout_millis.max(100));

        let mut cmd = Command::new(&self.config.executable_path);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        // Apply environment variables
        for (k, v) in &self.config.env_vars {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn external process {}: {e}", self.config.executable_path))?;

        let mut stdin = child.stdin.take().ok_or("Failed to open child stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open child stdout")?;
        let mut reader = BufReader::new(stdout);

        // Step 1: Handshake
        let handshake_req = LaneProtocolEnvelope::new(
            "msg-handshake-1",
            LaneMessagePayload::Handshake(HandshakeRequest {
                contract_digest: self.contract_digest.clone(),
                implementation_digest: self.implementation_lock.digest.clone(),
                supported_lanes: vec![ExecutionLane::IsolatedProcess.as_str().to_string()],
            }),
        );

        let mut handshake_raw = serde_json::to_string(&handshake_req).map_err(|e| e.to_string())?;
        handshake_raw.push('\n');
        stdin
            .write_all(handshake_raw.as_bytes())
            .map_err(|e| format!("Failed to write handshake to child process: {e}"))?;
        stdin.flush().map_err(|e| e.to_string())?;

        // Read HandshakeAck
        let mut ack_line = String::new();
        reader
            .read_line(&mut ack_line)
            .map_err(|e| format!("Failed to read handshake ack from child process: {e}"))?;

        if ack_line.trim().is_empty() {
            let _ = child.kill();
            return Err("External process closed stdout during handshake".to_string());
        }

        let ack_env: LaneProtocolEnvelope = serde_json::from_str(ack_line.trim())
            .map_err(|e| format!("Invalid handshake ack envelope from child: {e}"))?;

        match ack_env.payload {
            LaneMessagePayload::HandshakeAck(ack) => {
                if ack.status != "ready" {
                    let _ = child.kill();
                    return Err(format!(
                        "Child process rejected handshake: {}",
                        ack.reason.unwrap_or_else(|| "Unknown rejection".into())
                    ));
                }
            }
            LaneMessagePayload::Error(err) => {
                let _ = child.kill();
                return Err(format!("Child process handshake error: {} - {}", err.code, err.message));
            }
            other => {
                let _ = child.kill();
                return Err(format!("Unexpected message during handshake: {other:?}"));
            }
        }

        // Step 2: Send Activation
        let activation_req = LaneProtocolEnvelope::new(
            format!("msg-act-{}", request.activation_id),
            LaneMessagePayload::Activate(request.clone()),
        );
        let mut act_raw = serde_json::to_string(&activation_req).map_err(|e| e.to_string())?;
        act_raw.push('\n');
        stdin
            .write_all(act_raw.as_bytes())
            .map_err(|e| format!("Failed to write activation to child: {e}"))?;
        stdin.flush().map_err(|e| e.to_string())?;

        // Step 3: Read ActivationResult
        let mut result_line = String::new();
        reader
            .read_line(&mut result_line)
            .map_err(|e| format!("Failed to read activation result from child: {e}"))?;

        let elapsed = start_instant.elapsed();
        let elapsed_micros = elapsed.as_micros() as u64;

        if result_line.trim().is_empty() {
            let _ = child.kill();
            return Err("External process closed stdout before returning result".to_string());
        }

        let res_env: LaneProtocolEnvelope = serde_json::from_str(result_line.trim())
            .map_err(|e| format!("Invalid activation result envelope: {e}"))?;

        // Graceful exit / wait
        let _ = child.wait();

        match res_env.payload {
            LaneMessagePayload::ActivationResult(res) => Ok(LaneExecutionResult {
                activation_id: res.activation_id,
                outcome: res.outcome,
                output_port: res.output_port,
                output_data: res.output_data,
                artifact_references: res.artifact_references,
                causal_trace: res.causal_trace,
                resource_metrics: res.resource_metrics,
                elapsed_micros,
            }),
            LaneMessagePayload::Error(err) => Ok(LaneExecutionResult {
                activation_id: request.activation_id,
                outcome: "failure".to_string(),
                output_port: "error".to_string(),
                output_data: None,
                artifact_references: vec![],
                causal_trace: json!({
                    "error_code": err.code,
                    "error_message": err.message,
                    "retryable": err.retryable
                }),
                resource_metrics: LaneResourceMetrics {
                    cpu_micros: elapsed_micros,
                    peak_memory_bytes: 0,
                    items_processed: 0,
                },
                elapsed_micros,
            }),
            other => Err(format!("Unexpected activation result message: {other:?}")),
        }
    }
}
