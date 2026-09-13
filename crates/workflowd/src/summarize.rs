// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic bounded reduction for the native Summarize / Output Digest node.
//!
//! Summarize consumes the already-closed Merge stream one record at a time. It
//! retains only counters, a bounded ordinal bitmap, and the algorithm-tagged
//! digest chain; it never materializes the merged item payload.

use crate::canonical::digest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

pub const SUMMARIZE_ABI: &str = "canopy.summarize/v1alpha1";
pub const OUTPUT_DIGEST_SCHEMA: &str = "canopy.output-digest/v1alpha1";
pub const OUTPUT_DIGEST_ALGORITHM: &str = "sha256-jcs";
pub const MAX_SUMMARIZE_ITEMS: u64 = 50_000;
pub const ECO_GENERATED_COUNT: u64 = 49_998;
pub const ECO_TRUE_COUNT: u64 = 24_999;
pub const ECO_FALSE_COUNT: u64 = 24_999;
pub const ECO_TOTAL_ACTIVATIONS: u64 = 100_000;
pub const ECO_EXPECTED_OUTPUT_DIGEST: &str =
    "sha256:1caaeb3901bd0a17d8875a65c3363c8fbbc8c8a6695519ccbad0005e57ae9fdd";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub operation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SummaryRecord {
    pub ordinal: u64,
    pub input_port: String,
    pub logical_item: Value,
    pub logical_bytes: u64,
    pub provenance: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub operation: String,
    pub total_count: u64,
    pub true_count: u64,
    pub false_count: u64,
    pub logical_bytes: u64,
    pub output_digest: String,
    pub first_ordinal: Option<u64>,
    pub last_ordinal: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummarizeError {
    pub code: String,
    pub message: String,
}

impl SummarizeError {
    fn configuration(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.summarize.configuration".into(),
            message: message.into(),
        }
    }

    fn input(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.summarize.input".into(),
            message: message.into(),
        }
    }

    fn integrity(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.summarize.integrity".into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for SummarizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SummarizeError {}

#[derive(Clone, Debug)]
pub struct CompiledConfiguration {
    operation: String,
}

impl CompiledConfiguration {
    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn summarize<Records>(&self, records: Records) -> Result<Summary, SummarizeError>
    where
        Records: IntoIterator<Item = Result<SummaryRecord, SummarizeError>>,
    {
        let mut reducer = Reducer::new(&self.operation);
        for record in records {
            reducer.accept(record?)?;
        }
        reducer.finish()
    }
}

struct Reducer {
    operation: String,
    seen_ordinals: Vec<u64>,
    total_count: u64,
    true_count: u64,
    false_count: u64,
    logical_bytes: u64,
    output_digest: String,
    first_ordinal: Option<u64>,
    last_ordinal: Option<u64>,
}

impl Reducer {
    fn new(operation: &str) -> Self {
        Self {
            operation: operation.into(),
            seen_ordinals: vec![0; (MAX_SUMMARIZE_ITEMS as usize).div_ceil(64)],
            total_count: 0,
            true_count: 0,
            false_count: 0,
            logical_bytes: 0,
            output_digest: "genesis".into(),
            first_ordinal: None,
            last_ordinal: None,
        }
    }

    fn accept(&mut self, record: SummaryRecord) -> Result<(), SummarizeError> {
        let ordinal = usize::try_from(record.ordinal)
            .map_err(|_| SummarizeError::input("Summary record ordinal is not representable"))?;
        if ordinal >= MAX_SUMMARIZE_ITEMS as usize {
            return Err(SummarizeError::input(
                "Summary record ordinal exceeds the bounded input",
            ));
        }
        let word = ordinal / 64;
        let bit = 1_u64 << (ordinal % 64);
        if self.seen_ordinals[word] & bit != 0 {
            return Err(SummarizeError::integrity(format!(
                "Summary record ordinal {} was duplicated",
                record.ordinal
            )));
        }
        self.seen_ordinals[word] |= bit;
        if record.provenance.is_null() {
            return Err(SummarizeError::integrity(
                "Summary record is missing provenance",
            ));
        }
        match record.input_port.as_str() {
            "true" => self.true_count = self.true_count.saturating_add(1),
            "false" => self.false_count = self.false_count.saturating_add(1),
            _ => {
                return Err(SummarizeError::input(
                    "Summary input port must be true or false",
                ))
            }
        }
        self.output_digest = digest(&json!({
            "schema": OUTPUT_DIGEST_SCHEMA,
            "algorithm": OUTPUT_DIGEST_ALGORITHM,
            "previous": self.output_digest,
            "ordinal": record.ordinal,
            "input_port": record.input_port,
            "item": record.logical_item,
        }))
        .map_err(SummarizeError::integrity)?;
        self.logical_bytes = self
            .logical_bytes
            .checked_add(record.logical_bytes)
            .ok_or_else(|| SummarizeError::input("Summary logical byte count overflowed"))?;
        self.total_count = self.total_count.saturating_add(1);
        self.first_ordinal.get_or_insert(record.ordinal);
        self.last_ordinal = Some(record.ordinal);
        Ok(())
    }

    fn finish(self) -> Result<Summary, SummarizeError> {
        if self.true_count.saturating_add(self.false_count) != self.total_count {
            return Err(SummarizeError::integrity(
                "Summary counters do not cover every input record",
            ));
        }
        Ok(Summary {
            operation: self.operation,
            total_count: self.total_count,
            true_count: self.true_count,
            false_count: self.false_count,
            logical_bytes: self.logical_bytes,
            output_digest: self.output_digest,
            first_ordinal: self.first_ordinal,
            last_ordinal: self.last_ordinal,
        })
    }
}

pub fn compile_configuration(value: &Value) -> Result<CompiledConfiguration, SummarizeError> {
    let configuration: Configuration = serde_json::from_value(value.clone()).map_err(|error| {
        SummarizeError::configuration(format!("invalid Summarize configuration: {error}"))
    })?;
    if configuration.operation != "output_digest" {
        return Err(SummarizeError::configuration(
            "only output_digest Summarize operation is supported",
        ));
    }
    Ok(CompiledConfiguration {
        operation: configuration.operation,
    })
}

pub fn validate_configuration(value: &Value) -> Result<(), SummarizeError> {
    compile_configuration(value).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(ordinal: u64, input_port: &str, value: Value) -> SummaryRecord {
        SummaryRecord {
            ordinal,
            input_port: input_port.into(),
            logical_item: value,
            logical_bytes: 16,
            provenance: json!({"ordinal": ordinal, "if_output_port": input_port}),
        }
    }

    #[test]
    fn output_digest_reduces_counters_without_payload_materialization() {
        let configuration = compile_configuration(&json!({
            "operation": "output_digest"
        }))
        .unwrap();
        let summary = configuration
            .summarize(vec![
                Ok(record(0, "true", json!({"value": 2}))),
                Ok(record(1, "false", json!({"value": 3}))),
                Ok(record(2, "true", json!({"value": 4}))),
            ])
            .unwrap();
        assert_eq!(summary.operation, "output_digest");
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.true_count, 2);
        assert_eq!(summary.false_count, 1);
        assert_eq!(summary.logical_bytes, 48);
        assert_eq!(summary.first_ordinal, Some(0));
        assert_eq!(summary.last_ordinal, Some(2));
        assert!(summary.output_digest.starts_with("sha256:"));
    }

    #[test]
    fn duplicate_and_unknown_routes_are_structured_failures() {
        let configuration = compile_configuration(&json!({
            "operation": "output_digest"
        }))
        .unwrap();
        let duplicate = configuration.summarize(vec![
            Ok(record(4, "true", json!({"value": 4}))),
            Ok(record(4, "false", json!({"value": 4}))),
        ]);
        assert_eq!(duplicate.unwrap_err().code, "canopy.summarize.integrity");

        let unknown = configuration.summarize(vec![Ok(record(5, "other", json!({"value": 5})))]);
        assert_eq!(unknown.unwrap_err().code, "canopy.summarize.input");
    }

    #[test]
    fn unsupported_operation_is_rejected_before_publication() {
        let error = compile_configuration(&json!({
            "operation": "materialize_all"
        }))
        .unwrap_err();
        assert_eq!(error.code, "canopy.summarize.configuration");
    }

    #[test]
    fn frozen_eco_fixture_has_independent_expected_digest() {
        let configuration = compile_configuration(&json!({
            "operation": "output_digest"
        }))
        .unwrap();
        let mut records = Vec::with_capacity(ECO_GENERATED_COUNT as usize);
        for (input_port, start) in [("true", 0_u64), ("false", 1_u64)] {
            let step = 2_u64;
            for ordinal in (start..ECO_GENERATED_COUNT).step_by(step as usize) {
                let parity = if ordinal % 2 == 0 { "even" } else { "odd" };
                let logical_item = json!({
                    "index": ordinal,
                    "value": ordinal,
                    "data": null,
                    "parity": parity,
                    "label": format!("eco-{ordinal}"),
                });
                let logical_bytes = serde_jcs::to_vec(&logical_item).unwrap().len() as u64;
                records.push(Ok(SummaryRecord {
                    ordinal,
                    input_port: input_port.into(),
                    logical_item,
                    logical_bytes,
                    provenance: json!({
                        "if_output_port": input_port,
                        "ordinal": ordinal,
                    }),
                }));
            }
        }
        let summary = configuration.summarize(records).unwrap();
        assert_eq!(summary.total_count, ECO_GENERATED_COUNT);
        assert_eq!(summary.true_count, ECO_TRUE_COUNT);
        assert_eq!(summary.false_count, ECO_FALSE_COUNT);
        assert_eq!(summary.output_digest, ECO_EXPECTED_OUTPUT_DIGEST);
    }
}
