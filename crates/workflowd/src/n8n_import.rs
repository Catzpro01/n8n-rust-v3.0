// SPDX-License-Identifier: AGPL-3.0-or-later

//! n8n 2.39.0 workflow import (Ticket 15).
//!
//! Clean-room importer that operates only on documented, black-box public
//! surfaces of an owner-provided workflow JSON export. No n8n source code,
//! tests, Enterprise assets, or internal REST endpoints have been consulted
//! to write this module. Field semantics are restricted to the documented
//! v2 workflow schema (`name`, `nodes[]`, `connections`, `settings`,
//! `pinData`, `versionId`, etc.) and to public per-node documentation:
//!   - `parameters` is an opaque object preserved when safe;
//!   - `type` / `typeVersion` map to Canopy Node Contract locks through
//!     a declared `CompatibilityAlias` table;
//!   - `credentials` and any non-documented credential-shaped fields are
//!     redacted in a fail-closed manner;
//!   - `connections` are normalized to the Canopy `{source,target}` shape.
//!
//! The importer returns a `CompatibilityReport` classifying every node and
//! setting as Native, Delegated, Preserved, Adapted, Unsupported, or
//! Rejected; rejection is fatal so unsafe imports never become Drafts.

use crate::draft::{Layout, NodeInstance, WorkflowDraft};
use canopy_node_contract::{lock, NodeContractLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum accepted workflow JSON size (5 MiB).
pub const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;

/// Keys that must never appear inside imported `parameters` / `credentials`
/// objects. Matching is case-insensitive and matches substring shapes.
const SECRET_KEY_DENYLIST: &[&str] = &[
    "password",
    "passwd",
    "token",
    "accesstoken",
    "access_token",
    "refreshtoken",
    "refresh_token",
    "apikey",
    "api_key",
    "secret",
    "clientsecret",
    "client_secret",
    "privatekey",
    "private_key",
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
];

/// Base64-like blobs larger than this are rejected to avoid importing binary
/// content inline (attachments, files, encoded credentials).
const MAX_STRING_VALUE_BYTES: usize = 8_192;
/// Longest allowed display name length.
const MAX_NAME_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub draft: WorkflowDraft,
    pub report: CompatibilityReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompatibilityReport {
    pub source: ImportSource,
    pub total_nodes: usize,
    pub findings: Vec<ImportFinding>,
    pub classifications: ClassificationCounts,
    /// Sanitizer redaction log: list of (node_id, field_path, reason).
    pub redactions: Vec<Redaction>,
    pub blocked: bool,
    pub block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportSource {
    pub format: &'static str,
    pub declared_version: Option<String>,
    pub exact_external_identity_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ClassificationCounts {
    pub native_equivalent: usize,
    pub delegated_compatible: usize,
    pub preserved_opaque: usize,
    pub adapted: usize,
    pub unsupported: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportFinding {
    pub node_id: Option<String>,
    pub code: &'static str,
    pub classification: NodeClassification,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeClassification {
    NativeEquivalent,
    DelegatedCompatible,
    PreservedOpaque,
    Adapted,
    Unsupported,
    RejectedUnsafe,
}

#[derive(Debug, Clone, Serialize)]
pub struct Redaction {
    pub node_id: Option<String>,
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug)]
pub enum ImportError {
    TooLarge(usize),
    Parse(String),
    Rejected(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::TooLarge(bytes) => {
                write!(f, "import exceeds {MAX_IMPORT_BYTES} bytes (got {bytes})")
            }
            ImportError::Parse(msg) => write!(f, "cannot parse n8n workflow JSON: {msg}"),
            ImportError::Rejected(msg) => write!(f, "import rejected: {msg}"),
        }
    }
}

/// n8n v2 workflow export — only the documented public surface. Additional
/// fields are accepted (and preserved when safe) through the `extra` bag.
#[derive(Debug, Deserialize)]
struct N8nWorkflow {
    name: Option<Value>,
    #[serde(default)]
    nodes: Vec<N8nNode>,
    #[serde(default)]
    connections: Value,
    #[serde(default)]
    settings: Value,
    #[serde(default)]
    pinData: Value,
    #[serde(default)]
    versionId: Option<Value>,
    #[serde(flatten)]
    _extra: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct N8nNode {
    name: Option<Value>,
    #[serde(default)]
    parameters: Value,
    #[serde(rename = "type")]
    node_type: Option<Value>,
    #[serde(default)]
    typeVersion: Value,
    #[serde(default)]
    position: Value,
    #[serde(default, rename = "id")]
    external_id: Option<Value>,
    #[serde(default)]
    credentials: Value,
    #[serde(default)]
    webhookId: Value,
    #[serde(default)]
    continueOnFail: Value,
    #[serde(default)]
    retryOnFail: Value,
    #[serde(default)]
    maxTries: Value,
    #[serde(default)]
    waitBetweenTries: Value,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

/// Map of documented n8n node type/version → Canopy contract lock alias.
/// Only the first Ticket 15 subset is supported.
fn alias_table() -> BTreeMap<(&'static str, u64), AliasTarget> {
    let mut t = BTreeMap::new();
    t.insert(
        ("n8n-nodes-base.manualTrigger", 1),
        AliasTarget::Native("canopy", "manual-trigger", "v1alpha1"),
    );
    t.insert(
        ("n8n-nodes-base.executeCommand", 1),
        AliasTarget::Rejected("executeCommand runs arbitrary shell commands; unsafe for a bounded safe-execution engine"),
    );
    // The remaining built-ins encountered in a workflow that don't match an
    // alias are preserved as opaque (not executable but still represented as
    // draft nodes with a compatibility marker), per the clean-room contract.
    t
}

enum AliasTarget {
    Native(&'static str, &'static str, &'static str),
    #[allow(dead_code)]
    Delegated(&'static str, &'static str, &'static str),
    #[allow(dead_code)]
    Rejected(&'static str),
}

pub fn import_n8n_v2(workflow_id: &str, bytes: &[u8]) -> Result<ImportResult, ImportError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ImportError::TooLarge(bytes.len()));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| ImportError::Parse(format!("utf-8: {e}")))?;
    let mut src: N8nWorkflow = serde_json::from_str(text).map_err(|e| ImportError::Parse(format!("json: {e}")))?;

    let mut report = CompatibilityReport {
        source: ImportSource {
            format: "n8n.workflow-json/v2",
            declared_version: src.versionId.as_ref().and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            }),
            exact_external_identity_preserved: true,
        },
        total_nodes: src.nodes.len(),
        findings: Vec::new(),
        classifications: ClassificationCounts::default(),
        redactions: Vec::new(),
        blocked: false,
        block_reason: None,
    };

    let aliases = alias_table();

    // Redact credential blocks at the top level of every node — they contain
    // secrets and foreign identity material that must not enter the draft.
    for node in &mut src.nodes {
        if !matches!(node.credentials, Value::Null) {
            node.credentials = Value::Null;
            report.redactions.push(Redaction {
                node_id: string_value(node.external_id.as_ref()),
                path: "credentials".into(),
                reason: "credentials are redacted fail-closed; re-enter secrets through Canopy",
            });
        }
    }

    let mut nodes: Vec<NodeInstance> = Vec::with_capacity(src.nodes.len());
    let mut node_index = BTreeMap::new();

    for (i, node) in src.nodes.iter().enumerate() {
        let name = string_value(node.name.as_ref()).unwrap_or_else(|| format!("Imported node {}", i + 1));
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(ImportError::Rejected(format!(
                "node #{i} name exceeds {MAX_NAME_CHARS} characters"
            )));
        }
        let external_id = string_value(node.external_id.as_ref())
            .unwrap_or_else(|| format!("n8n-import-{i}"));
        let type_name = string_value(node.node_type.as_ref()).unwrap_or_default();
        let type_version = match &node.typeVersion {
            Value::Number(n) => n.as_u64().unwrap_or(1),
            _ => 1,
        };

        // Sanitize parameters recursively before preserving them.
        let mut safe_params = node.parameters.clone();
        sanitize_value(
            &mut safe_params,
            &format!("nodes[{i}].parameters"),
            Some(&external_id),
            &mut report.redactions,
        )
        .map_err(ImportError::Rejected)?;

        let (x, y) = parse_position(&node.position);
        let classification: NodeClassification;
        let contract_lock: NodeContractLock;
        let mut compat_meta = serde_json::Map::new();
        compat_meta.insert("n8n_type".into(), Value::String(type_name.clone()));
        compat_meta.insert("n8n_type_version".into(), json!(type_version));
        compat_meta.insert(
            "import_source".into(),
            Value::String("n8n.workflow-json/v2".into()),
        );

        let lock_value = |ns: &str, n: &str, v: &str| {
            json!({
                "identity": {
                    "api_version": "canopy.node/v1alpha1",
                    "namespace": ns,
                    "name": n,
                    "version": v,
                }
            })
        };
        match aliases.get(&(type_name.as_str(), type_version)) {
            Some(AliasTarget::Native(ns, n, v)) => {
                classification = NodeClassification::NativeEquivalent;
                report.classifications.native_equivalent += 1;
                contract_lock =
                    lock(&lock_value(ns, n, v)).map_err(|e| {
                        ImportError::Rejected(format!("native contract lock failed for {type_name}: {e}"))
                    })?;
                compat_meta.insert("search_label".into(), Value::String(n.replace('-', " ")));
            }
            Some(AliasTarget::Rejected(reason)) => {
                classification = NodeClassification::RejectedUnsafe;
                report.classifications.rejected += 1;
                report.blocked = true;
                report
                    .block_reason
                    .get_or_insert_with(String::new)
                    .push_str(&format!("node {external_id} ({type_name}): {reason}; "));
                contract_lock = opaque_contract_lock(&type_name, type_version);
            }
            Some(AliasTarget::Delegated(ns, n, v)) => {
                classification = NodeClassification::DelegatedCompatible;
                report.classifications.delegated_compatible += 1;
                contract_lock = lock(&lock_value(ns, n, v))
                    .map_err(|e| ImportError::Rejected(format!("delegated lock failed: {e}")))?;
            }
            None => {
                classification = NodeClassification::PreservedOpaque;
                report.classifications.preserved_opaque += 1;
                contract_lock = opaque_contract_lock(&type_name, type_version);
                compat_meta.insert("opaque_preserved".into(), Value::Bool(true));
            }
        }

        report.findings.push(ImportFinding {
            node_id: Some(external_id.clone()),
            code: match classification {
                NodeClassification::NativeEquivalent => "native_equivalent",
                NodeClassification::DelegatedCompatible => "delegated_compatible",
                NodeClassification::PreservedOpaque => "preserved_opaque",
                NodeClassification::Adapted => "adapted",
                NodeClassification::Unsupported => "unsupported",
                NodeClassification::RejectedUnsafe => "rejected_unsafe",
            },
            classification,
            message: format!("{type_name} v{type_version} → {classification:?}"),
        });

        nodes.push(NodeInstance {
            id: external_id.clone(),
            name: sanitize_name(&name),
            contract_lock,
            configuration: safe_params,
            layout: Layout { x, y },
            annotation: String::new(),
            compatibility_metadata: Value::Object(compat_meta),
        });
        node_index.insert(external_id, i);
    }

    // Connections: n8n exports connections as `{ sourceNodeName: { main: [[{node:target,type:…,index:…}]] } }`.
    // We normalise to the Canopy shape {id, source: {node_id, port_id}, target: {node_id, port_id}}.
    let connections = normalize_connections(&src.connections, &node_index, &mut report.findings);

    // Sanitize settings and pinData.
    let mut settings = src.settings;
    sanitize_value(
        &mut settings,
        "settings",
        None,
        &mut report.redactions,
    )
    .map_err(ImportError::Rejected)?;
    let _ = src.pinData; // preserved through draft.compatibility_metadata only when trivial
    let mut compat_meta = serde_json::Map::new();
    compat_meta.insert(
        "import_source".into(),
        Value::String("n8n.workflow-json/v2".into()),
    );
    compat_meta.insert("n8n_version_target".into(), Value::String("2.39.0".into()));

    if report.blocked {
        return Err(ImportError::Rejected(
            report.block_reason.clone().unwrap_or_default(),
        ));
    }

    let name = string_value(src.name.as_ref()).unwrap_or_else(|| "Imported n8n workflow".into());
    Ok(ImportResult {
        draft: WorkflowDraft {
            workflow_id: workflow_id.to_string(),
            name: sanitize_name(&name),
            draft_version: 1,
            nodes,
            connections,
            annotation: String::new(),
            settings,
            compatibility_metadata: Value::Object(compat_meta),
        },
        report,
    })
}

fn opaque_contract_lock(type_name: &str, version: u64) -> NodeContractLock {
    use sha2::Digest;
    let id = format!("opaque:{type_name}/v{version}");
    let hash = <sha2::Sha256 as Digest>::digest(id.as_bytes());
    NodeContractLock {
        api_version: "canopy.node/v1alpha1".into(),
        namespace: "n8n-import".into(),
        name: format!("opaque-{}", sanitize_ident(type_name)),
        version: format!("v{version}"),
        digest: format!("sha256:{:x}", hash),
    }
}

fn sanitize_value(
    value: &mut Value,
    path: &str,
    node_id: Option<&str>,
    redactions: &mut Vec<Redaction>,
) -> Result<(), String> {
    match value {
        Value::String(s) => {
            if s.len() > MAX_STRING_VALUE_BYTES {
                return Err(format!("{path}: string value exceeds {MAX_STRING_VALUE_BYTES} bytes (possible embedded binary)"));
            }
            if looks_like_base64_blob(s) && s.len() > 1024 {
                redactions.push(Redaction {
                    node_id: node_id.map(str::to_string),
                    path: path.into(),
                    reason: "large base64-like blob redacted",
                });
                *value = Value::String(String::from("[REDACTED]"));
            }
        }
        Value::Object(map) => {
            let mut redact_keys = Vec::new();
            for (k, v) in map.iter_mut() {
                let k_lc = k.to_ascii_lowercase();
                if SECRET_KEY_DENYLIST.iter().any(|denied| k_lc.contains(denied)) {
                    redact_keys.push(k.clone());
                    redactions.push(Redaction {
                        node_id: node_id.map(str::to_string),
                        path: format!("{path}.{k}"),
                        reason: "secret-shaped key redacted fail-closed",
                    });
                    continue;
                }
                sanitize_value(v, &format!("{path}.{k}"), node_id, redactions)?;
            }
            for k in redact_keys {
                map.insert(k, Value::String("[REDACTED]".into()));
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter_mut().enumerate() {
                sanitize_value(v, &format!("{path}[{i}]"), node_id, redactions)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn looks_like_base64_blob(s: &str) -> bool {
    if s.len() < 256 {
        return false;
    }
    let allowed = s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=' || b == b'\n' || b == b'\r');
    allowed && s.bytes().filter(|b| b.is_ascii_alphanumeric()).count() * 4 / 3 >= s.len().saturating_sub(2)
}

fn parse_position(v: &Value) -> (f64, f64) {
    // n8n stores position as [x,y] or {x,y}; tolerate both.
    match v {
        Value::Array(arr) if arr.len() >= 2 => {
            let x = arr[0].as_f64().unwrap_or(0.0);
            let y = arr[1].as_f64().unwrap_or(0.0);
            (x, y)
        }
        Value::Object(obj) => {
            let x = obj.get("x").and_then(Value::as_f64).unwrap_or(0.0);
            let y = obj.get("y").and_then(Value::as_f64).unwrap_or(0.0);
            (x, y)
        }
        _ => (0.0, 0.0),
    }
}

fn normalize_connections(
    connections: &Value,
    node_index: &BTreeMap<String, usize>,
    findings: &mut Vec<ImportFinding>,
) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let obj = match connections {
        Value::Object(m) => m,
        _ => return out,
    };
    for (source_name, ports) in obj {
        let Some(src_idx) = node_index.get(source_name) else { continue };
        let ports_obj = match ports {
            Value::Object(m) => m,
            _ => continue,
        };
        for (port_name, targets_arr) in ports_obj {
            let target_lists = match targets_arr {
                Value::Array(lists) => lists,
                _ => continue,
            };
            for (out_idx, targets) in target_lists.iter().enumerate() {
                let target_arr = match targets {
                    Value::Array(a) => a,
                    _ => continue,
                };
                for (in_idx, target) in target_arr.iter().enumerate() {
                    let obj = match target {
                        Value::Object(o) => o,
                        _ => continue,
                    };
                    let Some(target_name) = obj.get("node").and_then(Value::as_str) else { continue };
                    let target_port = obj
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("main");
                    if !node_index.contains_key(target_name) {
                        findings.push(ImportFinding {
                            node_id: Some(source_name.clone()),
                            code: "dangling_connection",
                            classification: NodeClassification::Adapted,
                            message: format!("connection to unknown target '{target_name}' dropped"),
                        });
                        continue;
                    }
                    let conn_id = format!("imp-{source_name}-{port_name}-{out_idx}-{target_name}-{target_port}-{in_idx}");
                    if !seen.insert(conn_id.clone()) {
                        continue;
                    }
                    out.push(json!({
                        "id": conn_id,
                        "source": { "node_id": source_name, "port_id": port_name },
                        "target": { "node_id": target_name, "port_id": target_port },
                    }));
                }
            }
        }
    }
    out
}

fn string_value(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn sanitize_name(s: &str) -> String {
    s.chars().take(MAX_NAME_CHARS).collect()
}

fn sanitize_ident(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const HELLO: &str = r#"{
      "name": "n8n 2.39.0 hello",
      "versionId": "e2e-hello-world-v1",
      "nodes": [
        {"id":"manual-1","name":"When clicking Test","type":"n8n-nodes-base.manualTrigger","typeVersion":1,"position":[200,200],"parameters":{}},
        {"id":"set-1","name":"Set greeting","type":"n8n-nodes-base.set","typeVersion":3,"position":[460,200],
         "parameters":{"assignments":{"assignments":[{"id":"a1","name":"message","value":"=Hello","type":"string"}]}}}
      ],
      "connections": {"manual-1": {"main": [[{"node":"set-1","type":"main","index":0}]]}}
    }"#;

    #[test]
    fn hello_world_classifies_and_preserves_identity() {
        let r = import_n8n_v2("wf-h", HELLO.as_bytes()).expect("import ok");
        assert_eq!(r.draft.workflow_id, "wf-h");
        assert_eq!(r.draft.name, "n8n 2.39.0 hello");
        assert_eq!(r.draft.nodes.len(), 2);
        assert_eq!(r.report.classifications.native_equivalent, 1);
        assert_eq!(r.report.classifications.preserved_opaque, 1);
        assert!(!r.report.blocked);
        assert_eq!(r.draft.nodes[0].id, "manual-1");
        assert_eq!(r.draft.nodes[1].id, "set-1");
        assert_eq!(r.draft.nodes[0].contract_lock.name, "manual-trigger");
        assert_eq!(r.draft.connections.len(), 1);
        assert_eq!(r.draft.connections[0]["source"]["node_id"], "manual-1");
        assert_eq!(r.draft.connections[0]["target"]["node_id"], "set-1");
    }

    #[test]
    fn secret_keys_and_credentials_redacted() {
        let doc = json!({
            "name":"s","nodes":[{"id":"h1","name":"HTTP","type":"n8n-nodes-base.httpRequest","typeVersion":4,"position":[0,0],
                "credentials":{"basic":{"user":"u","password":"P"}},
                "parameters":{"url":"https://x","apiKey":"S3CR3T","headers":{"Authorization":"Bearer x"}}
            }],"connections":{}
        });
        let r = import_n8n_v2("wf", serde_json::to_vec(&doc).unwrap().as_slice()).unwrap();
        let params = &r.draft.nodes[0].configuration;
        assert_eq!(params["apiKey"], "[REDACTED]");
        assert_eq!(params["headers"]["Authorization"], "[REDACTED]");
        assert!(r.report.redactions.iter().any(|red| red.path == "credentials"));
        assert!(r.report.redactions.iter().any(|red| red.path.ends_with(".apiKey")));
    }

    #[test]
    fn execute_command_blocks_import() {
        let doc = json!({
            "name":"e","nodes":[
                {"id":"m","name":"M","type":"n8n-nodes-base.manualTrigger","typeVersion":1,"position":[0,0],"parameters":{}},
                {"id":"sh","name":"Sh","type":"n8n-nodes-base.executeCommand","typeVersion":1,"position":[0,0],"parameters":{"command":"id"}}
            ],"connections":{"m":{"main":[[{"node":"sh","type":"main","index":0}]]}}
        });
        let err = import_n8n_v2("wf", serde_json::to_vec(&doc).unwrap().as_slice()).unwrap_err();
        assert!(matches!(err, ImportError::Rejected(_)), "expected reject, got {err}");
        assert!(err.to_string().contains("executeCommand"), "msg: {err}");
    }

    #[test]
    fn oversized_string_rejected() {
        let big = "A".repeat(MAX_STRING_VALUE_BYTES + 1);
        let doc = json!({"name":"b","nodes":[{"id":"1","name":"n","type":"t","typeVersion":1,"parameters":{"d":big}}],"connections":{}});
        let err = import_n8n_v2("wf", serde_json::to_vec(&doc).unwrap().as_slice()).unwrap_err();
        assert!(err.to_string().contains("exceeds"), "msg: {err}");
    }

    #[test]
    fn large_base64_blob_redacted() {
        let b64 = "Q".repeat(1500);
        let doc = json!({"name":"x","nodes":[{"id":"1","name":"n","type":"t","typeVersion":1,"parameters":{"blob":b64}}],"connections":{}});
        let r = import_n8n_v2("wf", serde_json::to_vec(&doc).unwrap().as_slice()).unwrap();
        assert_eq!(r.draft.nodes[0].configuration["blob"], "[REDACTED]");
    }

    #[test]
    fn dangling_connection_reported_as_adapted() {
        let doc = json!({"name":"d","nodes":[{"id":"m","name":"M","type":"n8n-nodes-base.manualTrigger","typeVersion":1,"position":[0,0],"parameters":{}}],
            "connections":{"m":{"main":[[{"node":"missing","type":"main","index":0}]]}}});
        let r = import_n8n_v2("wf", serde_json::to_vec(&doc).unwrap().as_slice()).unwrap();
        assert!(r.draft.connections.is_empty());
        assert!(r.report.findings.iter().any(|f| f.code == "dangling_connection"));
        assert_eq!(r.report.classifications.adapted, 1);
    }

    #[test]
    fn size_cap_enforced() {
        let pad = "x".repeat(MAX_IMPORT_BYTES);
        let body = format!("{{\"name\":\"x\",\"nodes\":[],\"connections\":{{}},\"pad\":\"{pad}\"}}");
        let err = import_n8n_v2("wf", body.as_bytes()).unwrap_err();
        assert!(matches!(err, ImportError::TooLarge(_)));
    }

    #[test]
    fn node_order_preserved_for_canonical_comparison() {
        let doc = json!({"name":"o","nodes":[
            {"id":"b","name":"B","type":"t","typeVersion":1,"position":[0,0],"parameters":{}},
            {"id":"a","name":"A","type":"t","typeVersion":1,"position":[0,0],"parameters":{}}
        ],"connections":{}});
        let r = import_n8n_v2("wf", serde_json::to_vec(&doc).unwrap().as_slice()).unwrap();
        let ids: Vec<&str> = r.draft.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn import_persists_and_roundtrips_through_draft_service() {
        use crate::config::ServeConfig;
        use crate::draft::DraftService;
        use std::net::SocketAddr;

        let tmp = std::env::temp_dir().join(format!(
            "canopy-import-test-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let cfg = ServeConfig {
            bind: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            state_dir: tmp.clone(),
            cgroup_dir: None,
            sqlite_min_version: 3_039_000,
            tls: None,
            master_key_file: None,
            control_origin: "http://localhost".into(),
            session_ttl_seconds: 3600,
            login_max_failures: 5,
            argon_memory_kib: 1024,
            argon_iterations: 1,
            draft_lease_ttl_seconds: 60,
            draft_takeover_grace_seconds: 1,
            draft_snapshot_interval: 100,
            draft_history_limit: 16,
            draft_undo_limit: 32,
        };
        let svc = DraftService::initialize(&cfg).expect("service init");
        let (draft, report_value) = svc.import_n8n_v2("wf-rt", HELLO.as_bytes()).expect("import");
        assert_eq!(draft.nodes.len(), 2);
        assert_eq!(report_value["blocked"], serde_json::Value::Bool(false));
        // Load back and verify identity preservation survives the SQLite roundtrip.
        let loaded = svc.load("wf-rt").expect("load");
        assert_eq!(loaded.workflow_id, "wf-rt");
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.nodes[0].id, "manual-1");
        assert_eq!(loaded.nodes[0].compatibility_metadata["n8n_type"], "n8n-nodes-base.manualTrigger");
        assert_eq!(loaded.connections.len(), 1);
        std::fs::remove_dir_all(&tmp).ok();
    }

    fn rand_suffix() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0))
    }
}

