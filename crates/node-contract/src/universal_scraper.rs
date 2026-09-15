// SPDX-License-Identifier: Apache-2.0
//! Normative contract for `8n-nodes-base.universalScraper`.
//!
//! The crate root owns the nine `NORMATIVE_FIELDS` and the strict shape checks.
//! This module is the scraper layer on top: which execution modes exist, where
//! a mode is declared, and which modes actually have a dependency stack pinned
//! in `Cargo.lock`.
//!
//! `configuration` accepts only `defaults`, `editor_hints` and `schema`, so the
//! mode is a default parameter, not a top-level key.

use serde_json::Value;

pub const NODE_NAMESPACE: &str = "8n-nodes-base";
pub const NODE_NAME: &str = "universalScraper";
pub const MODE_DEFAULT_KEY: &str = "mode";

/// Output port every mode must declare. The scraper engine hands tabular rows
/// downstream without touching disk, so the port name is part of the contract.
pub const ROWS_OUTPUT_PORT: &str = "rows";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScraperMode {
    FastHttp,
    DeepCrawl,
    BrowserHeadless,
    DataTransform,
}

impl ScraperMode {
    pub const ALL: [ScraperMode; 4] = [
        ScraperMode::FastHttp,
        ScraperMode::DeepCrawl,
        ScraperMode::BrowserHeadless,
        ScraperMode::DataTransform,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ScraperMode::FastHttp => "FastHttp",
            ScraperMode::DeepCrawl => "DeepCrawl",
            ScraperMode::BrowserHeadless => "BrowserHeadless",
            ScraperMode::DataTransform => "DataTransform",
        }
    }

    pub fn parse(value: &str) -> Option<ScraperMode> {
        ScraperMode::ALL
            .into_iter()
            .find(|mode| mode.as_str() == value)
    }

    /// A mode is buildable only when every crate it needs is pinned.
    ///
    /// `spider`, `chromiumoxide` and `polars` were dropped for MSRV 1.85.1 and
    /// `csv` was never added to the workspace, so modes 3 and 4 currently have
    /// no stack behind them. FastHttp and DeepCrawl are covered by the locked
    /// `reqwest` / `scraper` / `governor` / `backoff` set.
    pub fn dependency_stack_locked(self) -> bool {
        matches!(self, ScraperMode::FastHttp | ScraperMode::DeepCrawl)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ScraperError {
    #[error("configuration must be an object")]
    ConfigurationNotObject,
    #[error("configuration.defaults must be an object")]
    DefaultsNotObject,
    #[error("configuration.defaults.mode must be a string")]
    ModeNotString,
    #[error("unknown scraper mode: {0}")]
    UnknownMode(String),
    #[error("scraper mode {0} has no dependency stack locked in Cargo.lock")]
    UnlockedStack(String),
    #[error("ports.outputs must declare a `{0}` port")]
    MissingRowsOutput(String),
}

/// Read the declared mode and refuse one that cannot be built yet.
pub fn declared_mode(contract: &Value) -> Result<ScraperMode, ScraperError> {
    let configuration = contract
        .get("configuration")
        .and_then(Value::as_object)
        .ok_or(ScraperError::ConfigurationNotObject)?;
    let defaults = configuration
        .get("defaults")
        .and_then(Value::as_object)
        .ok_or(ScraperError::DefaultsNotObject)?;
    let raw = defaults
        .get(MODE_DEFAULT_KEY)
        .and_then(Value::as_str)
        .ok_or(ScraperError::ModeNotString)?;
    let mode = ScraperMode::parse(raw).ok_or_else(|| ScraperError::UnknownMode(raw.to_owned()))?;
    if mode.dependency_stack_locked() {
        Ok(mode)
    } else {
        Err(ScraperError::UnlockedStack(mode.as_str().to_owned()))
    }
}

/// Shape of the `rows` port. The crate-root validator already enforces the
/// allowed port keys; this pins the two the engine depends on.
pub fn declares_rows_output(contract: &Value) -> bool {
    contract
        .get("ports")
        .and_then(|ports| ports.get("outputs"))
        .and_then(Value::as_array)
        .map(|outputs| {
            outputs
                .iter()
                .any(|output| output.get("id").and_then(Value::as_str) == Some(ROWS_OUTPUT_PORT))
        })
        .unwrap_or(false)
}

/// Scraper-specific gate. Run the crate-root `validate` first: it owns the
/// normative fields and would reject a contract this function cannot judge.
pub fn check(contract: &Value) -> Result<ScraperMode, ScraperError> {
    let mode = declared_mode(contract)?;
    if declares_rows_output(contract) {
        Ok(mode)
    } else {
        Err(ScraperError::MissingRowsOutput(ROWS_OUTPUT_PORT.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn contract(mode: &str) -> Value {
        json!({
            "configuration": { "defaults": { "mode": mode } },
            "ports": {
                "inputs": [],
                "outputs": [{ "id": "rows", "cardinality": "many" }],
            },
        })
    }

    #[test]
    fn locked_modes_are_accepted() {
        assert_eq!(
            declared_mode(&contract("FastHttp")).unwrap(),
            ScraperMode::FastHttp
        );
        assert_eq!(
            declared_mode(&contract("DeepCrawl")).unwrap(),
            ScraperMode::DeepCrawl
        );
    }

    #[test]
    fn modes_without_a_locked_stack_are_refused() {
        for mode in ["BrowserHeadless", "DataTransform"] {
            assert_eq!(
                declared_mode(&contract(mode)).unwrap_err(),
                ScraperError::UnlockedStack(mode.to_owned()),
                "{mode} has no crate behind it yet and must not pass the gate"
            );
        }
    }

    #[test]
    fn unknown_mode_is_named_in_the_error() {
        assert_eq!(
            declared_mode(&contract("Telepathy")).unwrap_err(),
            ScraperError::UnknownMode("Telepathy".to_owned())
        );
    }

    #[test]
    fn mode_must_be_a_string() {
        let value = json!({ "configuration": { "defaults": { "mode": 3 } } });
        assert_eq!(
            declared_mode(&value).unwrap_err(),
            ScraperError::ModeNotString
        );
    }

    #[test]
    fn missing_defaults_is_reported_as_such() {
        let value = json!({ "configuration": {} });
        assert_eq!(
            declared_mode(&value).unwrap_err(),
            ScraperError::DefaultsNotObject
        );
    }

    #[test]
    fn rows_output_port_is_required() {
        let value = json!({
            "configuration": { "defaults": { "mode": "FastHttp" } },
            "ports": { "inputs": [], "outputs": [{ "id": "html" }] },
        });
        assert_eq!(
            check(&value).unwrap_err(),
            ScraperError::MissingRowsOutput(ROWS_OUTPUT_PORT.to_owned())
        );
        assert_eq!(check(&contract("FastHttp")).unwrap(), ScraperMode::FastHttp);
    }

    #[test]
    fn every_mode_round_trips_through_its_own_name() {
        for mode in ScraperMode::ALL {
            assert_eq!(ScraperMode::parse(mode.as_str()), Some(mode));
        }
    }
}
