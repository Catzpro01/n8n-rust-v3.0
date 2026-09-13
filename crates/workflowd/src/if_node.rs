// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure deterministic If routing for the approved Ticket 09 contract.
//!
//! Conditions are compiled from the bounded native expression language. The
//! node evaluates every condition against the immutable input item and emits
//! exactly one boolean route; it never duplicates or drops an item.

use crate::expression::{self, EvalValue, ExpressionError, Program};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
}

pub fn compile_configuration(value: &Value) -> Result<CompiledConfiguration, IfError> {
    let configuration: Configuration = serde_json::from_value(value.clone()).map_err(|error| {
        IfError::configuration(format!("invalid If configuration: {error}"))
    })?;
    if configuration.conditions.is_empty() {
        return Err(IfError::configuration(
            "at least one condition is required",
        ));
    }
    if configuration.conditions.len() > MAX_CONDITIONS {
        return Err(IfError::configuration(format!(
            "condition limit is {MAX_CONDITIONS}"
        )));
    }
    let conditions = configuration
        .conditions
        .iter()
        .map(|condition| {
            expression::compile(&condition.expression).map_err(IfError::evaluation)
        })
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
        assert_eq!(all.route(&json!({"value": 5}), 0).unwrap().output_port, "true");
        assert_eq!(all.route(&json!({"value": 10}), 0).unwrap().output_port, "false");
        assert_eq!(any.route(&json!({"value": 10}), 0).unwrap().output_port, "true");
        assert_eq!(any.route(&json!({"value": 5}), 0).unwrap().output_port, "false");
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
