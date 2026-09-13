// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure deterministic If routing for the approved Ticket 09 contract.
//!
//! Conditions are compiled from the bounded native expression language. The
//! node evaluates every condition against the immutable input item and emits
//! exactly one boolean route; it never duplicates or drops an item.

use crate::{
    canonical::digest,
    expression::{self, EvalValue, ExpressionError, Program},
    generate_engine::GeneratedEnvelope,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

pub const IF_ABI: &str = "canopy.if/v1alpha1";
const MAX_CONDITIONS: usize = 64;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Logic {
    All,
    Any,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub expression: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub logic: Logic,
    pub conditions: Vec<Condition>,
}

#[derive(Clone, Debug)]
pub struct CompiledConfiguration {
    logic: Logic,
    conditions: Vec<Program>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteResult {
    pub output_port: &'static str,
    pub condition_results: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchBatch {
    pub true_count: u64,
    pub false_count: u64,
    pub stream_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IfError {
    pub code: String,
    pub message: String,
}

impl IfError {
    fn configuration(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.if.configuration".into(),
            message: message.into(),
        }
    }

    fn evaluation(error: ExpressionError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }

    fn type_error(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.if.type".into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for IfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for IfError {}

impl CompiledConfiguration {
    pub fn route(&self, input: &Value, item_index: u64) -> Result<RouteResult, IfError> {
        let mut condition_results = Vec::with_capacity(self.conditions.len());
        for condition in &self.conditions {
            let result = condition
                .evaluate(input, item_index)
                .map_err(IfError::evaluation)?;
            let EvalValue::Json(Value::Bool(result)) = result else {
                return Err(IfError::type_error(
                    "If conditions must evaluate to JSON booleans",
                ));
            };
            condition_results.push(result);
        }
        let matched = match self.logic {
            Logic::All => condition_results.iter().all(|result| *result),
            Logic::Any => condition_results.iter().any(|result| *result),
        };
        Ok(RouteResult {
            output_port: if matched { "true" } else { "false" },
            condition_results,
        })
    }

    pub fn route_batch(
        &self,
        envelopes: &mut [GeneratedEnvelope],
        node_id: &str,
        previous_digest: &str,
    ) -> Result<BranchBatch, IfError> {
        let mut true_count = 0_u64;
        let mut false_count = 0_u64;
        let mut stream_digest = previous_digest.to_owned();
        let mut staged = envelopes.to_vec();
        for envelope in &mut staged {
            let input_digest = digest(&envelope.logical_item).map_err(|message| IfError {
                code: "canopy.if.digest_failed".into(),
                message,
            })?;
            let route = self.route(&envelope.logical_item, envelope.ordinal)?;
            if route.output_port == "true" {
                true_count = true_count.saturating_add(1);
            } else {
                false_count = false_count.saturating_add(1);
            }
            if let Some(provenance) = envelope.provenance.as_object_mut() {
                provenance.insert("if_node_instance_id".into(), json!(node_id));
                provenance.insert("if_input_digest".into(), json!(input_digest));
                provenance.insert("if_output_port".into(), json!(route.output_port));
                provenance.insert(
                    "if_condition_results".into(),
                    json!(route.condition_results.clone()),
                );
            }
            stream_digest = digest(&json!({
                "schema": "canopy.if-route-chain/v1alpha1",
                "previous": &stream_digest,
                "ordinal": envelope.ordinal,
                "output_port": route.output_port,
                "condition_results": &route.condition_results,
                "item": &envelope.logical_item,
            }))
            .map_err(|message| IfError {
                code: "canopy.if.digest_failed".into(),
                message,
            })?;
        }
        envelopes.clone_from_slice(&staged);
        Ok(BranchBatch {
            true_count,
            false_count,
            stream_digest,
        })
    }
}

pub fn compile_configuration(value: &Value) -> Result<CompiledConfiguration, IfError> {
    let configuration: Configuration = serde_json::from_value(value.clone())
        .map_err(|error| IfError::configuration(format!("invalid If configuration: {error}")))?;
    if configuration.conditions.is_empty() {
        return Err(IfError::configuration("at least one condition is required"));
    }
    if configuration.conditions.len() > MAX_CONDITIONS {
        return Err(IfError::configuration(format!(
            "condition limit is {MAX_CONDITIONS}"
        )));
    }
    let conditions = configuration
        .conditions
        .iter()
        .map(|condition| expression::compile(&condition.expression).map_err(IfError::evaluation))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CompiledConfiguration {
        logic: configuration.logic,
        conditions,
    })
}

pub fn validate_configuration(value: &Value) -> Result<(), IfError> {
    compile_configuration(value).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn even() -> Value {
        json!({
            "logic": "all",
            "conditions": [{"expression": "$json.value % 2 === 0"}]
        })
    }

    #[test]
    fn routes_each_item_to_exactly_one_named_port() {
        let configuration = compile_configuration(&even()).unwrap();
        assert_eq!(
            configuration.route(&json!({"value": 4}), 0).unwrap(),
            RouteResult {
                output_port: "true",
                condition_results: vec![true]
            }
        );
        assert_eq!(
            configuration.route(&json!({"value": 5}), 1).unwrap(),
            RouteResult {
                output_port: "false",
                condition_results: vec![false]
            }
        );
    }

    #[test]
    fn all_and_any_are_deterministic_and_cover_composition() {
        let all = compile_configuration(&json!({
            "logic": "all",
            "conditions": [
                {"expression": "$json.value > 0"},
                {"expression": "$json.value < 10"}
            ]
        }))
        .unwrap();
        let any = compile_configuration(&json!({
            "logic": "any",
            "conditions": [
                {"expression": "$json.value === 0"},
                {"expression": "$json.value === 10"}
            ]
        }))
        .unwrap();
        assert_eq!(
            all.route(&json!({"value": 5}), 0).unwrap().output_port,
            "true"
        );
        assert_eq!(
            all.route(&json!({"value": 10}), 0).unwrap().output_port,
            "false"
        );
        assert_eq!(
            any.route(&json!({"value": 10}), 0).unwrap().output_port,
            "true"
        );
        assert_eq!(
            any.route(&json!({"value": 5}), 0).unwrap().output_port,
            "false"
        );
    }

    #[test]
    fn batch_routing_is_exactly_one_port_and_replay_stable() {
        let configuration = compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "$json.value >= 2"}]
        }))
        .unwrap();
        let mut envelopes = vec![
            GeneratedEnvelope {
                ordinal: 0,
                item: json!({"value": 1}),
                logical_item: json!({"value": 1}),
                logical_bytes: 11,
                provenance: json!({"ordinal": 0}),
            },
            GeneratedEnvelope {
                ordinal: 1,
                item: json!({"value": 2}),
                logical_item: json!({"value": 2}),
                logical_bytes: 11,
                provenance: json!({"ordinal": 1}),
            },
            GeneratedEnvelope {
                ordinal: 2,
                item: json!({"value": 3}),
                logical_item: json!({"value": 3}),
                logical_bytes: 11,
                provenance: json!({"ordinal": 2}),
            },
        ];
        let replay = envelopes.clone();
        let first = configuration
            .route_batch(&mut envelopes, "if-node", "genesis")
            .unwrap();
        let mut replay_envelopes = replay;
        let second = configuration
            .route_batch(&mut replay_envelopes, "if-node", "genesis")
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.true_count, 2);
        assert_eq!(first.false_count, 1);
        assert!(first.stream_digest.starts_with("sha256:"));
        assert_eq!(
            envelopes
                .iter()
                .map(|envelope| envelope.provenance["if_output_port"].as_str())
                .collect::<Vec<_>>(),
            vec![Some("false"), Some("true"), Some("true")]
        );
        assert!(envelopes.iter().all(|envelope| {
            envelope.provenance["if_node_instance_id"] == "if-node"
                && envelope.provenance["if_condition_results"]
                    .as_array()
                    .is_some_and(|results| results.len() == 1)
        }));
    }

    #[test]
    fn batch_routing_does_not_publish_partial_provenance_on_error() {
        let configuration = compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "$json.value === true"}]
        }))
        .unwrap();
        let mut envelopes = vec![
            GeneratedEnvelope {
                ordinal: 0,
                item: json!({"value": true}),
                logical_item: json!({"value": true}),
                logical_bytes: 11,
                provenance: json!({"ordinal": 0}),
            },
            GeneratedEnvelope {
                ordinal: 1,
                item: json!({}),
                logical_item: json!({}),
                logical_bytes: 2,
                provenance: json!({"ordinal": 1}),
            },
        ];
        let original = envelopes.clone();
        let error = configuration
            .route_batch(&mut envelopes, "if-node", "genesis")
            .unwrap_err();
        assert_eq!(error.code, "canopy.expression.missing");
        assert_eq!(envelopes[0].provenance, original[0].provenance);
        assert_eq!(envelopes[1].provenance, original[1].provenance);
    }

    #[test]
    fn missing_null_and_non_boolean_results_are_structured_errors() {
        let missing = compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "$json.missing === true"}]
        }))
        .unwrap();
        let missing_error = missing.route(&json!({}), 0).unwrap_err();
        assert_eq!(missing_error.code, "canopy.expression.missing");

        let null = compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "$json.value"}]
        }))
        .unwrap();
        let null_error = null.route(&json!({"value": null}), 0).unwrap_err();
        assert_eq!(null_error.code, "canopy.if.type");
    }

    #[test]
    fn invalid_configuration_is_rejected_before_publication() {
        let error = compile_configuration(&json!({
            "logic": "all",
            "conditions": []
        }))
        .unwrap_err();
        assert_eq!(error.code, "canopy.if.configuration");

        let error = compile_configuration(&json!({
            "logic": "all",
            "conditions": [{"expression": "value = 1"}]
        }))
        .unwrap_err();
        assert_eq!(error.code, "canopy.expression.unsupported");
    }
}
