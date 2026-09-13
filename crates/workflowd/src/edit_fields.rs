// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure Edit Fields transformation for the approved Ticket 08 contract.

use crate::expression::{self, EvalValue, ExpressionError, Program};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

pub const EDIT_FIELDS_ABI: &str = "canopy.edit-fields/v1alpha2";
const MAX_ASSIGNMENTS: usize = 256;
const MAX_PATH_SEGMENTS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    Merge,
    Replace,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssignmentValue {
    Fixed { value: Value },
    Expression { source: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Assignment {
    pub path: Vec<PathSegment>,
    #[serde(flatten)]
    pub value: AssignmentValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub mode: OutputMode,
    pub assignments: Vec<Assignment>,
}

#[derive(Clone, Debug)]
enum CompiledValue {
    Fixed(Value),
    Expression(Program),
}

#[derive(Clone, Debug)]
struct CompiledAssignment {
    path: Vec<PathSegment>,
    value: CompiledValue,
}

#[derive(Clone, Debug)]
pub struct CompiledConfiguration {
    mode: OutputMode,
    assignments: Vec<CompiledAssignment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransformResult {
    pub item: Value,
    pub item_index: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformError {
    pub code: String,
    pub message: String,
}

impl TransformError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    fn configuration(message: impl Into<String>) -> Self {
        Self::new("canopy.edit-fields.configuration", message)
    }

    fn input(message: impl Into<String>) -> Self {
        Self::new("canopy.edit-fields.input", message)
    }

    fn path(message: impl Into<String>) -> Self {
        Self::new("canopy.edit-fields.path", message)
    }

    fn expression(error: ExpressionError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

impl fmt::Display for TransformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TransformError {}

pub fn compile_configuration(value: &Value) -> Result<CompiledConfiguration, TransformError> {
    let configuration: Configuration = serde_json::from_value(value.clone()).map_err(|error| {
        TransformError::configuration(format!("invalid Edit Fields configuration: {error}"))
    })?;
    if configuration.assignments.is_empty() {
        return Err(TransformError::configuration(
            "at least one assignment is required",
        ));
    }
    if configuration.assignments.len() > MAX_ASSIGNMENTS {
        return Err(TransformError::configuration(format!(
            "assignment limit is {MAX_ASSIGNMENTS}"
        )));
    }
    let mut compiled: Vec<CompiledAssignment> = Vec::with_capacity(configuration.assignments.len());
    for assignment in configuration.assignments {
        validate_path(&assignment.path)?;
        for previous in &compiled {
            if paths_conflict(&previous.path, &assignment.path) {
                return Err(TransformError::configuration(
                    "duplicate or parent/child assignment paths are not allowed",
                ));
            }
        }
        let value = match assignment.value {
            AssignmentValue::Fixed { value } => CompiledValue::Fixed(value),
            AssignmentValue::Expression { source } => CompiledValue::Expression(
                expression::compile(&source).map_err(TransformError::expression)?,
            ),
        };
        compiled.push(CompiledAssignment {
            path: assignment.path,
            value,
        });
    }
    Ok(CompiledConfiguration {
        mode: configuration.mode,
        assignments: compiled,
    })
}

pub fn validate_configuration(value: &Value) -> Result<(), TransformError> {
    compile_configuration(value).map(|_| ())
}

impl CompiledConfiguration {
    pub fn mode(&self) -> &OutputMode {
        &self.mode
    }

    pub fn apply(&self, input: &Value, item_index: u64) -> Result<TransformResult, TransformError> {
        let mut output = match self.mode {
            OutputMode::Merge => input
                .as_object()
                .cloned()
                .map(Value::Object)
                .ok_or_else(|| TransformError::input("merge mode requires an object input"))?,
            OutputMode::Replace => Value::Object(Map::new()),
        };
        for assignment in &self.assignments {
            let value = match &assignment.value {
                CompiledValue::Fixed(value) => EvalValue::Json(value.clone()),
                CompiledValue::Expression(program) => program
                    .evaluate(input, item_index)
                    .map_err(TransformError::expression)?,
            };
            match value {
                EvalValue::Missing => remove_path(&mut output, &assignment.path)?,
                EvalValue::Json(value) => set_path(&mut output, &assignment.path, value)?,
            }
        }
        Ok(TransformResult {
            item: output,
            item_index,
        })
    }
}

fn validate_path(path: &[PathSegment]) -> Result<(), TransformError> {
    if path.is_empty() || path.len() > MAX_PATH_SEGMENTS {
        return Err(TransformError::configuration(format!(
            "path must contain between one and {MAX_PATH_SEGMENTS} segments"
        )));
    }
    for segment in path {
        if let PathSegment::Key(key) = segment {
            if key.is_empty() {
                return Err(TransformError::configuration("path keys must not be empty"));
            }
        }
    }
    Ok(())
}

fn paths_conflict(left: &[PathSegment], right: &[PathSegment]) -> bool {
    let shared = left.len().min(right.len());
    left[..shared] == right[..shared]
}

fn container_for(next: &PathSegment) -> Value {
    match next {
        PathSegment::Key(_) => Value::Object(Map::new()),
        PathSegment::Index(_) => Value::Array(Vec::new()),
    }
}

fn set_path(target: &mut Value, path: &[PathSegment], value: Value) -> Result<(), TransformError> {
    let Some((head, tail)) = path.split_first() else {
        return Err(TransformError::path("cannot assign an empty path"));
    };
    match head {
        PathSegment::Key(key) => {
            let object = target
                .as_object_mut()
                .ok_or_else(|| TransformError::path("a key path requires an object"))?;
            if tail.is_empty() {
                object.insert(key.clone(), value);
                return Ok(());
            }
            let child = object
                .entry(key.clone())
                .or_insert_with(|| container_for(&tail[0]));
            set_path(child, tail, value)
        }
        PathSegment::Index(index) => {
            let array = target
                .as_array_mut()
                .ok_or_else(|| TransformError::path("an index path requires an array"))?;
            if *index > 16_384 {
                return Err(TransformError::path(
                    "array path index exceeds the safe bound",
                ));
            }
            if array.len() <= *index {
                array.resize_with(index + 1, || Value::Null);
            }
            if tail.is_empty() {
                array[*index] = value;
                return Ok(());
            }
            if array[*index].is_null() {
                array[*index] = container_for(&tail[0]);
            }
            set_path(&mut array[*index], tail, value)
        }
    }
}

fn remove_path(target: &mut Value, path: &[PathSegment]) -> Result<(), TransformError> {
    let Some((head, tail)) = path.split_first() else {
        return Err(TransformError::path("cannot remove an empty path"));
    };
    match head {
        PathSegment::Key(key) => {
            let Some(object) = target.as_object_mut() else {
                return Err(TransformError::path("a key path requires an object"));
            };
            if tail.is_empty() {
                object.remove(key);
                return Ok(());
            }
            let Some(child) = object.get_mut(key) else {
                return Ok(());
            };
            remove_path(child, tail)
        }
        PathSegment::Index(_) => Err(TransformError::path(
            "a Missing result cannot omit an array index",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn configuration() -> Value {
        json!({
            "mode": "merge",
            "assignments": [
                {"path": ["eco"], "kind": "fixed", "value": true},
                {"path": ["parity"], "kind": "expression", "source": "$json.value % 2 === 0 ? \"even\" : \"odd\""},
                {"path": ["doubled"], "kind": "expression", "source": "$json.value * 2"},
                {"path": ["label"], "kind": "expression", "source": "\"eco-\" + $json.index"}
            ]
        })
    }

    #[test]
    fn applies_frozen_eco_assignments_against_immutable_input() {
        let compiled = compile_configuration(&configuration()).unwrap();
        let input = json!({"index": 12, "value": 7, "data": {"keep": true}});
        let output = compiled.apply(&input, 12).unwrap();
        assert_eq!(
            output.item,
            json!({
                "index": 12,
                "value": 7,
                "data": {"keep": true},
                "eco": true,
                "parity": "odd",
                "doubled": 14,
                "label": "eco-12"
            })
        );
        assert_eq!(
            input,
            json!({"index": 12, "value": 7, "data": {"keep": true}})
        );
    }

    #[test]
    fn supports_replace_nested_paths_and_missing_omission() {
        let compiled = compile_configuration(&json!({
            "mode": "replace",
            "assignments": [
                {"path": ["meta", "name"], "kind": "fixed", "value": "Ada"},
                {"path": ["meta", "index"], "kind": "expression", "source": "$itemIndex"},
                {"path": ["optional"], "kind": "expression", "source": "$json.not_there"}
            ]
        }))
        .unwrap();
        assert_eq!(
            compiled.apply(&json!({"value": 4}), 3).unwrap().item,
            json!({"meta": {"name": "Ada", "index": 3}})
        );
    }

    #[test]
    fn rejects_duplicate_or_parent_paths_before_execution() {
        let error = compile_configuration(&json!({
            "mode": "merge",
            "assignments": [
                {"path": ["a"], "kind": "fixed", "value": 1},
                {"path": ["a", "b"], "kind": "fixed", "value": 2}
            ]
        }))
        .unwrap_err();
        assert_eq!(error.code, "canopy.edit-fields.configuration");
    }

    #[test]
    fn merge_requires_an_object_and_expression_errors_are_stable() {
        let compiled = compile_configuration(&configuration()).unwrap();
        let input_error = compiled.apply(&json!([1, 2]), 0).unwrap_err();
        assert_eq!(input_error.code, "canopy.edit-fields.input");
        let expression = compile_configuration(&json!({
            "mode": "merge",
            "assignments": [{"path": ["x"], "kind": "expression", "source": "$json.value + true"}]
        }))
        .unwrap()
        .apply(&json!({"value": 1}), 0)
        .unwrap_err();
        assert_eq!(expression.code, "canopy.expression.type");
    }
}
