// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic, bounded Merge reduction for the native branch-stream contract.
//!
//! Merge is deliberately a reducer over closed input streams. The reducer owns
//! only counters and digest state; callers provide iterators backed by bounded
//! Artifact readers and decide where merged records are written.

use crate::canonical::digest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

pub const MERGE_ABI: &str = "canopy.merge/v1alpha1";
pub const MERGE_CHAIN_SCHEMA: &str = "canopy.merge-chain/v1alpha1";
pub const MERGE_MODE_TRUE_THEN_FALSE: &str = "true_then_false";
pub const MAX_MERGE_ITEMS: u64 = 50_000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub mode: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MergeRecord {
    pub ordinal: u64,
    pub item: Value,
    pub logical_item: Value,
    pub logical_bytes: u64,
    pub provenance: Value,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MergeSummary {
    pub mode: String,
    pub true_count: u64,
    pub false_count: u64,
    pub output_count: u64,
    pub logical_bytes: u64,
    pub stream_digest: String,
    pub first_output_ordinal: Option<u64>,
    pub last_output_ordinal: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeError {
    pub code: String,
    pub message: String,
}

impl MergeError {
    fn configuration(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.merge.configuration".into(),
            message: message.into(),
        }
    }

    fn input(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.merge.input".into(),
            message: message.into(),
        }
    }

    fn integrity(message: impl Into<String>) -> Self {
        Self {
            code: "canopy.merge.integrity".into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for MergeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for MergeError {}

#[derive(Clone, Debug)]
pub struct CompiledConfiguration {
    mode: String,
}

impl CompiledConfiguration {
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// Reduce a true stream followed by a false stream. Each accepted record is
    /// handed to `emit` immediately, so this function never materializes either
    /// input or the merged output.
    pub fn merge_ordered<IT, IF, ET, EF, Emit>(
        &self,
        true_records: IT,
        false_records: IF,
        mut emit: Emit,
    ) -> Result<MergeSummary, MergeError>
    where
        IT: IntoIterator<Item = Result<MergeRecord, ET>>,
        IF: IntoIterator<Item = Result<MergeRecord, EF>>,
        ET: Into<MergeError>,
        EF: Into<MergeError>,
        Emit: FnMut(&MergeRecord, &'static str) -> Result<(), MergeError>,
    {
        let mut reducer = Reducer::new(&self.mode);
        for record in true_records {
            let record = record.map_err(Into::into)?;
            reducer.accept("true", record, &mut emit)?;
        }
        for record in false_records {
            let record = record.map_err(Into::into)?;
            reducer.accept("false", record, &mut emit)?;
        }
        reducer.finish()
    }
}

struct Reducer {
    mode: String,
    phase: &'static str,
    // A fixed-size ordinal bitmap detects duplicate/lost links without retaining
    // any item payload or allocating in proportion to the stream size.
    seen_ordinals: Vec<u64>,
    true_count: u64,
    false_count: u64,
    output_count: u64,
    logical_bytes: u64,
    stream_digest: String,
    first_output_ordinal: Option<u64>,
    last_output_ordinal: Option<u64>,
}

impl Reducer {
    fn new(mode: &str) -> Self {
        Self {
            mode: mode.into(),
            phase: "true",
            seen_ordinals: vec![0; (MAX_MERGE_ITEMS as usize).div_ceil(64)],
            true_count: 0,
            false_count: 0,
            output_count: 0,
            logical_bytes: 0,
            stream_digest: "genesis".into(),
            first_output_ordinal: None,
            last_output_ordinal: None,
        }
    }

    fn accept<Emit>(
        &mut self,
        input_port: &'static str,
        record: MergeRecord,
        emit: &mut Emit,
    ) -> Result<(), MergeError>
    where
        Emit: FnMut(&MergeRecord, &'static str) -> Result<(), MergeError>,
    {
        if input_port == "false" && self.phase == "true" {
            self.phase = "false";
        }
        if input_port != self.phase {
            return Err(MergeError::input(format!(
                "Merge input order changed after {} closed",
                self.phase
            )));
        }
        if record.provenance.is_null() {
            return Err(MergeError::integrity("Merge record is missing provenance"));
        }
        let ordinal = usize::try_from(record.ordinal)
            .map_err(|_| MergeError::input("Merge record ordinal is not representable"))?;
        if ordinal >= MAX_MERGE_ITEMS as usize {
            return Err(MergeError::input(
                "Merge record ordinal exceeds the bounded input",
            ));
        }
        let word = ordinal / 64;
        let bit = 1_u64 << (ordinal % 64);
        if self.seen_ordinals[word] & bit != 0 {
            return Err(MergeError::integrity(format!(
                "Merge record ordinal {} was duplicated",
                record.ordinal
            )));
        }
        self.seen_ordinals[word] |= bit;
        if self.output_count >= MAX_MERGE_ITEMS {
            return Err(MergeError::input("Merge output item limit exceeded"));
        }
        self.stream_digest = digest(&json!({
            "schema": MERGE_CHAIN_SCHEMA,
            "previous": self.stream_digest,
            "input_port": input_port,
            "ordinal": record.ordinal,
            "item": record.logical_item,
        }))
        .map_err(MergeError::integrity)?;
        self.logical_bytes = self
            .logical_bytes
            .checked_add(record.logical_bytes)
            .ok_or_else(|| MergeError::input("Merge logical byte count overflowed"))?;
        self.output_count += 1;
        match input_port {
            "true" => self.true_count += 1,
            "false" => self.false_count += 1,
            _ => unreachable!(),
        }
        self.first_output_ordinal.get_or_insert(record.ordinal);
        self.last_output_ordinal = Some(record.ordinal);
        emit(&record, input_port)
    }

    fn finish(self) -> Result<MergeSummary, MergeError> {
        if self.true_count.saturating_add(self.false_count) != self.output_count {
            return Err(MergeError::integrity(
                "Merge counters do not cover every output record",
            ));
        }
        Ok(MergeSummary {
            mode: self.mode,
            true_count: self.true_count,
            false_count: self.false_count,
            output_count: self.output_count,
            logical_bytes: self.logical_bytes,
            stream_digest: self.stream_digest,
            first_output_ordinal: self.first_output_ordinal,
            last_output_ordinal: self.last_output_ordinal,
        })
    }
}

pub fn compile_configuration(value: &Value) -> Result<CompiledConfiguration, MergeError> {
    let configuration: Configuration = serde_json::from_value(value.clone()).map_err(|error| {
        MergeError::configuration(format!("invalid Merge configuration: {error}"))
    })?;
    if configuration.mode != MERGE_MODE_TRUE_THEN_FALSE {
        return Err(MergeError::configuration(
            "only true_then_false Merge ordering is supported",
        ));
    }
    Ok(CompiledConfiguration {
        mode: configuration.mode,
    })
}

pub fn validate_configuration(value: &Value) -> Result<(), MergeError> {
    compile_configuration(value).map(|_| ())
}

pub fn record_from_value(value: Value) -> Result<MergeRecord, MergeError> {
    serde_json::from_value(value)
        .map_err(|error| MergeError::integrity(format!("invalid Merge record: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(ordinal: u64, port: &str) -> MergeRecord {
        MergeRecord {
            ordinal,
            item: json!({"ordinal": ordinal}),
            logical_item: json!({"ordinal": ordinal}),
            logical_bytes: 10,
            provenance: json!({"if_output_port": port}),
        }
    }

    #[test]
    fn true_then_false_is_independent_of_branch_close_interleaving() {
        let configuration = compile_configuration(&json!({"mode": "true_then_false"})).unwrap();
        let mut output = Vec::new();
        let summary = configuration
            .merge_ordered(
                vec![Ok(record(0, "true")), Ok(record(2, "true"))],
                vec![Ok(record(1, "false")), Ok(record(3, "false"))],
                |record, port| {
                    output.push((port, record.ordinal));
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(
            output,
            [("true", 0), ("true", 2), ("false", 1), ("false", 3)]
        );
        assert_eq!(summary.true_count, 2);
        assert_eq!(summary.false_count, 2);
        assert_eq!(summary.output_count, 4);
    }

    #[test]
    fn duplicate_provenance_is_rejected_without_silent_loss() {
        let configuration = compile_configuration(&json!({"mode": "true_then_false"})).unwrap();
        let error = configuration
            .merge_ordered(
                vec![Ok(record(0, "true"))],
                vec![Ok(record(0, "false"))],
                |_, _| Ok(()),
            )
            .unwrap_err();
        assert_eq!(error.code, "canopy.merge.integrity");
    }

    #[test]
    fn unsupported_mode_is_typed() {
        let error = compile_configuration(&json!({"mode": "arrival_order"})).unwrap_err();
        assert_eq!(error.code, "canopy.merge.configuration");
    }
}
