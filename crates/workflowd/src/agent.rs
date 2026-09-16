// SPDX-License-Identifier: AGPL-3.0-or-later
//! AI Agent Platform & Durable Capability-Bound Turns (ADR 0061).
//!
//! Enforces:
//! - 6 typed subports: Agent Engine, Memory, Skill, MCP/Tool, Policy, Output.
//! - Locked Agent Blueprints & Model Routes.
//! - Durable TurnRequest / TurnResponse lifecycle with state checkpoints.
//! - Strict capability boundaries and zero ambient credentials.
//! - Ordered pre-side-effect fallback; post-side-effect unknown states become `Uncertain`.
//! - Bounded Output Contract validation before any downstream emission.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const AGENT_TURN_ABI: &str = "canopy.agent-turn/v1alpha1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRoute {
    pub primary_model_id: String,
    pub fallback_model_ids: Vec<String>,
    pub temperature_thousandths: u32,
    pub timeout_millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnBudgets {
    pub max_tokens: u64,
    pub max_wall_time_ms: u64,
    pub max_tool_invocations: u32,
}

impl Default for TurnBudgets {
    fn default() -> Self {
        Self {
            max_tokens: 8192,
            max_wall_time_ms: 30_000,
            max_tool_invocations: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSubports {
    pub engine_ref: String,
    pub memory_refs: Vec<String>,
    pub skill_refs: Vec<String>,
    pub mcp_tool_refs: Vec<String>,
    pub policy_ref: String,
    pub output_contract_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentBlueprintLock {
    pub blueprint_id: String,
    pub version: String,
    pub digest: String,
    pub model_route: ModelRoute,
    pub subports: AgentSubports,
    pub budgets: TurnBudgets,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnOutcome {
    Success,
    FallbackApplied,
    Uncertain,
    BudgetExceeded,
    OutputContractFailed,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TurnRequest {
    pub turn_id: String,
    pub run_id: String,
    pub node_instance_id: String,
    pub logical_order: u64,
    pub input_message: String,
    pub memory_context: Vec<Value>,
    pub granted_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TurnResponse {
    pub turn_id: String,
    pub outcome: TurnOutcome,
    pub active_model_id: String,
    pub validated_output: Option<Value>,
    pub tool_calls_executed: u32,
    pub tokens_consumed: u64,
    pub trace_facts: Value,
}

/// Deterministic built-in Fake Agent Engine for testing and vertical slice verification.
pub struct FakeAgentEngine {
    blueprint: AgentBlueprintLock,
    simulate_primary_failure_pre_side_effect: bool,
    simulate_side_effect_uncertainty: bool,
    simulate_invalid_output: bool,
}

impl FakeAgentEngine {
    pub fn new(blueprint: AgentBlueprintLock) -> Self {
        Self {
            blueprint,
            simulate_primary_failure_pre_side_effect: false,
            simulate_side_effect_uncertainty: false,
            simulate_invalid_output: false,
        }
    }

    pub fn with_pre_side_effect_failure(mut self) -> Self {
        self.simulate_primary_failure_pre_side_effect = true;
        self
    }

    pub fn with_uncertain_side_effect(mut self) -> Self {
        self.simulate_side_effect_uncertainty = true;
        self
    }

    pub fn with_invalid_output(mut self) -> Self {
        self.simulate_invalid_output = true;
        self
    }

    /// Executes an Agent Turn adhering strictly to ADR 0061 rules.
    pub fn execute_turn(&self, req: &TurnRequest) -> TurnResponse {
        let mut tokens_consumed = 150;
        let mut tool_calls = 0;

        // Rule 1: Post-side-effect failure cannot be retried -> Outcome must be Uncertain
        if self.simulate_side_effect_uncertainty {
            return TurnResponse {
                turn_id: req.turn_id.clone(),
                outcome: TurnOutcome::Uncertain,
                active_model_id: self.blueprint.model_route.primary_model_id.clone(),
                validated_output: None,
                tool_calls_executed: 1,
                tokens_consumed: 300,
                trace_facts: json!({
                    "error": "Disconnection after external tool write initiated",
                    "idempotency_state": "unknown"
                }),
            };
        }

        // Rule 2: Pre-side-effect primary failure triggers ordered fallback
        let (active_model, outcome) = if self.simulate_primary_failure_pre_side_effect {
            let fallback = self
                .blueprint
                .model_route
                .fallback_model_ids
                .first()
                .cloned()
                .unwrap_or_else(|| "fallback-default".into());
            tokens_consumed += 120;
            (fallback, TurnOutcome::FallbackApplied)
        } else {
            (self.blueprint.model_route.primary_model_id.clone(), TurnOutcome::Success)
        };

        // Rule 3: Bounded memory and tool access
        if !req.granted_tools.is_empty() {
            tool_calls += 1;
        }

        // Rule 4: Output Contract validation
        if self.simulate_invalid_output {
            return TurnResponse {
                turn_id: req.turn_id.clone(),
                outcome: TurnOutcome::OutputContractFailed,
                active_model_id: active_model,
                validated_output: None,
                tool_calls_executed: tool_calls,
                tokens_consumed,
                trace_facts: json!({
                    "error": "Model response failed schema validation and repair budget exhausted"
                }),
            };
        }

        let output_data = json!({
            "response": format!("Response to '{}' via model {}", req.input_message, active_model),
            "memory_items_consulted": req.memory_context.len(),
            "tools_used": tool_calls
        });

        TurnResponse {
            turn_id: req.turn_id.clone(),
            outcome,
            active_model_id: active_model,
            validated_output: Some(output_data),
            tool_calls_executed: tool_calls,
            tokens_consumed,
            trace_facts: json!({
                "subports": {
                    "engine": self.blueprint.subports.engine_ref,
                    "policy": self.blueprint.subports.policy_ref,
                    "output_contract": self.blueprint.subports.output_contract_ref
                }
            }),
        }
    }
}
