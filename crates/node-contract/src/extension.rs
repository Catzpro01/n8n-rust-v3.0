// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const LANE_PROTOCOL_ABI: &str = "canopy.lane-protocol/v1alpha1";

/// Identifies the runtime form of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeForm {
    /// In-process native Rust node.
    Native,
    /// Out-of-process executable communicating over framed stdio/IPC.
    ExternalProcess,
    /// Isolated sandboxed WebAssembly module.
    Wasm,
    /// AI Agent Engine bounded to blueprints and model routes.
    AgentEngine,
}

impl Default for NodeForm {
    fn default() -> Self {
        Self::Native
    }
}

/// Identifies the physical/virtual execution environment assigned to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionLane {
    /// Pure in-process thread pool. Reserved for verified Rust native nodes.
    NativeCpu,
    /// Subprocess isolated via OS sandbox, cgroups, and stdio framing.
    IsolatedProcess,
    /// Sandboxed WebAssembly runtime with capability-gated host calls.
    WasmSandbox,
    /// Network-isolated remote worker (A2A or distributed agent pool).
    RemoteWorker,
}

impl ExecutionLane {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NativeCpu => "native-cpu",
            Self::IsolatedProcess => "isolated-process",
            Self::WasmSandbox => "wasm-sandbox",
            Self::RemoteWorker => "remote-worker",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "native-cpu" => Some(Self::NativeCpu),
            "isolated-process" => Some(Self::IsolatedProcess),
            "wasm-sandbox" => Some(Self::WasmSandbox),
            "remote-worker" => Some(Self::RemoteWorker),
            _ => None,
        }
    }

    /// Checks if this lane provides strictly equal or higher isolation than `other`.
    pub fn is_at_least_as_isolated_as(&self, other: &Self) -> bool {
        let rank = |lane: &Self| match lane {
            Self::NativeCpu => 0,
            Self::WasmSandbox => 1,
            Self::IsolatedProcess => 2,
            Self::RemoteWorker => 3,
        };
        rank(self) >= rank(other)
    }
}

/// Immutable lock representing a verified executable implementation of a Node Contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NodeImplementationLock {
    pub api_version: String,
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub digest: String,
    pub form: NodeForm,
    pub entrypoint: String,
    pub runtime_requirements: Value,
}

/// Declared capability grant passed to isolated executions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub kind: String,
    pub scope: String,
    pub parameters: Value,
}

/// Bounded secret lease handle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretLeaseHandle {
    pub lease_id: String,
    pub expires_at_epoch_ms: u64,
}

/// Wire envelope for all messages exchanged across the external process lane protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaneProtocolEnvelope {
    pub protocol_version: String,
    pub message_id: String,
    pub payload: LaneMessagePayload,
}

impl LaneProtocolEnvelope {
    pub fn new(message_id: impl Into<String>, payload: LaneMessagePayload) -> Self {
        Self {
            protocol_version: LANE_PROTOCOL_ABI.to_string(),
            message_id: message_id.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LaneMessagePayload {
    Handshake(HandshakeRequest),
    HandshakeAck(HandshakeAckResponse),
    Activate(LaneActivationRequest),
    ActivationResult(LaneActivationResultResponse),
    Cancel(CancelRequest),
    Error(LaneProtocolError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub contract_digest: String,
    pub implementation_digest: String,
    pub supported_lanes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeAckResponse {
    pub status: String,
    pub implementation_version: String,
    pub accepted_lane: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaneActivationRequest {
    pub activation_id: String,
    pub node_instance_id: String,
    pub logical_order: u64,
    pub deadline_epoch_millis: Option<u64>,
    pub input_port: String,
    pub input_data: Value,
    pub artifact_references: Vec<Value>,
    pub capability_grants: Vec<CapabilityGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaneActivationResultResponse {
    pub activation_id: String,
    pub outcome: String, // "success", "failure", "cancelled", "suspended"
    pub output_port: String,
    pub output_data: Option<Value>,
    pub artifact_references: Vec<Value>,
    pub causal_trace: Value,
    pub resource_metrics: LaneResourceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LaneResourceMetrics {
    pub cpu_micros: u64,
    pub peak_memory_bytes: u64,
    pub items_processed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CancelRequest {
    pub activation_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaneProtocolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
