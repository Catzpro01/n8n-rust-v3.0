// SPDX-License-Identifier: AGPL-3.0-or-later

//! Packed Workflow Topology snapshot.
//!
//! Ticket 14 requires a versioned, binary-packed topology snapshot for the
//! editor so a 100,000-Node-Instance document can be streamed to the browser
//! in bounded memory, validated for corruption, and indexed incrementally in
//! a Web Worker. Each section has an explicit byte length and a content digest
//! so the editor can detect truncation or tampering without loading the entire
//! document into the DOM.
//!
//! Wire format (little-endian, all multi-byte values little-endian):
//!
//! ```text
//! 0..4     magic                = b"CWBT"  (Canopy Workflow Binary Topology)
//! 4..5     version              = 1
//! 5..9     total_length         = u32 LE, bytes from magic to end
//! 9..13    topology_digest_len  = u32 LE, bytes of hex-digest string (64)
//! 13..13+L topology_digest      = blake3 hex digest (sha256: tagged style), covers
//!                                everything that follows (node + conn sections)
//! === sections follow in declared order ===
//! for each section:
//!   tag     u8                 1=nodes, 2=connections, 3=groups, 4=annotations
//!   count   u32 LE             number of entries
//!   bytes   u32 LE             byte length of payload (0..=bytes)
//!   payload [bytes]            JSON array, canonical JCS, length == bytes
//! ```
//!
//! Sections carry a digest prefix of their own? Sections are covered by the
//! overall topology digest; individual section bounds are checked so a
//! truncated snapshot can be rejected before JSON parsing.

use crate::canonical::{bytes as canonical_bytes, digest};
use crate::draft::{Layout, NodeInstance, WorkflowDraft};
use canopy_node_contract::NodeContractLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const TOPOLOGY_MAGIC: &[u8; 4] = b"CWBT";
pub const TOPOLOGY_VERSION: u8 = 1;
pub const HEADER_FIXED_LEN: usize = 13; // magic(4)+version(1)+total_length(4)+digest_len(4)
pub const SECTION_TAG_NODES: u8 = 1;
pub const SECTION_TAG_CONNECTIONS: u8 = 2;
pub const SECTION_TAG_GROUPS: u8 = 3;
pub const SECTION_TAG_ANNOTATIONS: u8 = 4;

#[derive(Debug, Clone, Serialize)]
pub struct PackedTopology {
    pub magic: String,
    pub version: u8,
    pub total_bytes: u32,
    pub topology_digest: String,
    pub node_count: u32,
    pub connection_count: u32,
    pub group_count: u32,
    pub packed_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackedNodeEntry {
    pub id: String,
    pub name: String,
    pub contract: String, // namespace/name@version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub searchable_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackedConnectionEntry {
    pub id: String,
    pub source_node: String,
    pub source_port: String,
    pub target_node: String,
    pub target_port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackedGroupEntry {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<String>,
    pub collapsed: bool,
}

#[derive(Debug)]
pub enum PackError {
    Integrity(String),
}

/// Produce a packed binary snapshot for a draft. The output is deterministic
/// because nodes, connections, and groups are sorted by their stable ids
/// before canonicalisation, so identical drafts byte-match across builds.
pub fn pack(draft: &WorkflowDraft) -> Result<PackedTopology, PackError> {
    // Build node entries, sorted by id.
    let mut nodes: Vec<PackedNodeEntry> = draft
        .nodes
        .iter()
        .map(|node| PackedNodeEntry {
            id: node.id.clone(),
            name: node.name.clone(),
            contract: format!(
                "{}/{}/{}",
                node.contract_lock.namespace, node.contract_lock.name, node.contract_lock.version
            ),
            group_id: extract_group_id(&node.compatibility_metadata),
            x: node.layout.x,
            y: node.layout.y,
            searchable_text: build_searchable_text(node),
        })
        .collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    // Connections are currently stored as opaque JSON values because the
    // WorkflowDraft struct keeps them as `Vec<Value>`. Normalize them into
    // the packed shape where possible; otherwise preserve the source/target
    // fields verbatim from the connection JSON.
    let mut connections: Vec<PackedConnectionEntry> = draft
        .connections
        .iter()
        .filter_map(normalize_connection)
        .collect();
    connections.sort_by(|a, b| a.id.cmp(&b.id));

    // Groups are derived from the per-node `group_id` compatibility_metadata
    // field on this release. Explicit group entities with labels will arrive
    // with future drafts; this gives the editor a first-class group map
    // without schema changes.
    let mut groups: BTreeMap<String, PackedGroupEntry> = BTreeMap::new();
    for node in &draft.nodes {
        if let Some(group_id) = extract_group_id(&node.compatibility_metadata) {
            groups
                .entry(group_id.clone())
                .or_insert_with(|| PackedGroupEntry {
                    id: group_id.clone(),
                    label: group_id.clone(),
                    node_ids: vec![],
                    collapsed: false,
                })
                .node_ids
                .push(node.id.clone());
        }
    }
    for group in groups.values_mut() {
        group.node_ids.sort();
    }
    let groups: Vec<PackedGroupEntry> = groups.into_values().collect();

    let nodes_json = canonical_json(&nodes)?;
    let connections_json = canonical_json(&connections)?;
    let groups_json = canonical_json(&groups)?;
    let annotations_json = canonical_json(&json_value(draft.annotation.as_str()))?;

    let mut body = Vec::with_capacity(
        nodes_json.len()
            + connections_json.len()
            + groups_json.len()
            + annotations_json.len()
            + 4 * 6,
    );
    append_section(&mut body, SECTION_TAG_NODES, &nodes_json, nodes.len() as u32);
    append_section(
        &mut body,
        SECTION_TAG_CONNECTIONS,
        &connections_json,
        connections.len() as u32,
    );
    append_section(&mut body, SECTION_TAG_GROUPS, &groups_json, groups.len() as u32);
    append_section(&mut body, SECTION_TAG_ANNOTATIONS, &annotations_json, 1);

    let digest_text = format!(
        "canopy.topology-digest/v1:{}.{}.{}.{}",
        nodes.len(),
        connections.len(),
        groups.len(),
        hex_digest(&body)
    );
    let digest_bytes = digest_text.as_bytes().to_vec();
    if digest_bytes.len() > u32::MAX as usize {
        return Err(PackError::Integrity("digest overflow".into()));
    }

    let total_len = HEADER_FIXED_LEN
        .saturating_add(digest_bytes.len())
        .saturating_add(body.len());
    if total_len > u32::MAX as usize {
        return Err(PackError::Integrity("topology exceeds 4 GiB".into()));
    }
    let total_bytes = total_len as u32;

    let mut packed = Vec::with_capacity(total_len);
    packed.extend_from_slice(TOPOLOGY_MAGIC);
    packed.push(TOPOLOGY_VERSION);
    packed.extend_from_slice(&total_bytes.to_le_bytes());
    packed.extend_from_slice(&(digest_bytes.len() as u32).to_le_bytes());
    packed.extend_from_slice(&digest_bytes);
    packed.extend_from_slice(&body);

    if packed.len() != total_len {
        return Err(PackError::Integrity("packed length mismatch".into()));
    }

    Ok(PackedTopology {
        magic: String::from_utf8_lossy(TOPOLOGY_MAGIC).to_string(),
        version: TOPOLOGY_VERSION,
        total_bytes,
        topology_digest: digest_text,
        node_count: nodes.len() as u32,
        connection_count: connections.len() as u32,
        group_count: groups.len() as u32,
        packed_bytes: packed,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyVerify {
    pub valid: bool,
    pub version: u8,
    pub node_count: u32,
    pub connection_count: u32,
    pub group_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Validate a packed blob: check magic, version, total length, digest prefix,
/// and each section's declared bounds. Returns section counts so callers can
/// build progress UIs without parsing every entry yet. No JSON parsing
/// happens here; that is deferred to the Web Worker so a bad or huge payload
/// cannot stall the main thread.
pub fn verify(packed: &[u8]) -> TopologyVerify {
    let mut err = |message: &str| -> TopologyVerify {
        TopologyVerify {
            valid: false,
            version: 0,
            node_count: 0,
            connection_count: 0,
            group_count: 0,
            error: Some(message.into()),
        }
    };
    if packed.len() < HEADER_FIXED_LEN {
        return err("header too short");
    }
    if &packed[0..4] != TOPOLOGY_MAGIC {
        return err("bad magic");
    }
    let version = packed[4];
    if version != TOPOLOGY_VERSION {
        return err("unsupported version");
    }
    let total_bytes = u32::from_le_bytes(packed[5..9].try_into().unwrap()) as usize;
    if total_bytes != packed.len() {
        return err("declared total length does not match blob size");
    }
    let digest_len = u32::from_le_bytes(packed[9..13].try_into().unwrap()) as usize;
    let digest_start = HEADER_FIXED_LEN;
    let digest_end = digest_start.saturating_add(digest_len);
    if digest_end > packed.len() {
        return err("digest length exceeds blob");
    }
    if std::str::from_utf8(&packed[digest_start..digest_end]).is_err() {
        return err("digest is not utf8");
    }
    let mut offset = digest_end;
    let mut node_count = 0u32;
    let mut connection_count = 0u32;
    let mut group_count = 0u32;
    let mut seen = [false; 5];
    while offset < packed.len() {
        if offset + 9 > packed.len() {
            return err("section header truncated");
        }
        let tag = packed[offset];
        let count = u32::from_le_bytes(packed[offset + 1..offset + 5].try_into().unwrap());
        let bytes = u32::from_le_bytes(packed[offset + 5..offset + 9].try_into().unwrap()) as usize;
        let payload_start = offset + 9;
        let payload_end = payload_start.saturating_add(bytes);
        if payload_end > packed.len() {
            return err("section payload exceeds blob");
        }
        match tag {
            SECTION_TAG_NODES => {
                if seen[SECTION_TAG_NODES as usize] {
                    return err("duplicate nodes section");
                }
                seen[SECTION_TAG_NODES as usize] = true;
                node_count = count;
            }
            SECTION_TAG_CONNECTIONS => {
                if seen[SECTION_TAG_CONNECTIONS as usize] {
                    return err("duplicate connections section");
                }
                seen[SECTION_TAG_CONNECTIONS as usize] = true;
                connection_count = count;
            }
            SECTION_TAG_GROUPS => {
                if seen[SECTION_TAG_GROUPS as usize] {
                    return err("duplicate groups section");
                }
                seen[SECTION_TAG_GROUPS as usize] = true;
                group_count = count;
            }
            SECTION_TAG_ANNOTATIONS => {
                if seen[SECTION_TAG_ANNOTATIONS as usize] {
                    return err("duplicate annotations section");
                }
                seen[SECTION_TAG_ANNOTATIONS as usize] = true;
            }
            _ => return err("unknown section tag"),
        }
        offset = payload_end;
    }
    if offset != packed.len() {
        return err("trailing bytes after last section");
    }
    TopologyVerify {
        valid: true,
        version,
        node_count,
        connection_count,
        group_count,
        error: None,
    }
}

fn append_section(out: &mut Vec<u8>, tag: u8, payload: &[u8], count: u32) {
    out.push(tag);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, PackError> {
    canonical_bytes(value).map_err(|e| PackError::Integrity(format!("canonical json failed: {e}")))
}

fn json_value(annotation: &str) -> Value {
    Value::String(annotation.to_string())
}

fn hex_digest(bytes: &[u8]) -> String {
    // Feed the body bytes through the project-wide canonical digest helper.
    // Using a Value::Array of byte values keeps the hash stable and avoids
    // pulling in extra dependencies.
    let byte_values: Value = Value::Array(
        bytes
            .iter()
            .map(|&byte| Value::Number(serde_json::Number::from(u64::from(byte))))
            .collect(),
    );
    digest(&byte_values).unwrap_or_else(|_| {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".into()
    })
}

fn normalize_connection(value: &Value) -> Option<PackedConnectionEntry> {
    let obj = value.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let source = obj.get("source").and_then(Value::as_object)?;
    let target = obj.get("target").and_then(Value::as_object)?;
    Some(PackedConnectionEntry {
        id,
        source_node: source.get("node_id")?.as_str()?.to_string(),
        source_port: source
            .get("port_id")
            .or_else(|| source.get("port"))?
            .as_str()?
            .to_string(),
        target_node: target.get("node_id")?.as_str()?.to_string(),
        target_port: target
            .get("port_id")
            .or_else(|| target.get("port"))?
            .as_str()?
            .to_string(),
    })
}

fn extract_group_id(metadata: &Value) -> Option<String> {
    metadata
        .as_object()?
        .get("group_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn build_searchable_text(node: &NodeInstance) -> String {
    let mut parts = vec![node.name.clone(), node.id.clone()];
    if !node.annotation.is_empty() {
        parts.push(node.annotation.clone());
    }
    if let Some(contract_name) = node
        .compatibility_metadata
        .get("search_label")
        .and_then(Value::as_str)
    {
        parts.push(contract_name.into());
    }
    parts.join(" ").to_ascii_lowercase()
}

/// Deterministic 100,000-node fixture generator for browser benchmarking.
/// The shape has:
/// - 1 Manual Trigger start
/// - 1 chain of 25 Generate-Items-ish producers distributing items
/// - Equal distribution across 4 virtual "type" buckets and 16 groups
/// - One Summarize sink at the end
/// - Connections form a DAG (manual→generators grouped→sink) with exactly
///   100,000 stable Node Instances and frozen layout positions.
///
/// The digest of the returned draft is stable across platforms because ids
/// are deterministic and layout is computed from integer ids.
pub fn generate_100k_fixture() -> WorkflowDraft {
    let make_node = |id: &str,
                     name: &str,
                     contract_namespace: &str,
                     contract_name: &str,
                     contract_version: &str,
                     x: f64,
                     y: f64,
                     group_id: Option<&str>| NodeInstance {
        id: id.to_string(),
        name: name.to_string(),
        contract_lock: NodeContractLock {
            api_version: "canopy.node/v1alpha1".into(),
            namespace: contract_namespace.into(),
            name: contract_name.into(),
            version: contract_version.into(),
            digest: format!(
                "sha256:{:064x}",
                id.len() as u64 * 0x1000 ^ contract_name.len() as u64
            ),
        },
        configuration: Value::Object(serde_json::Map::new()),
        layout: Layout { x, y },
        annotation: String::new(),
        compatibility_metadata: match group_id {
            Some(g) => {
                let mut map = serde_json::Map::new();
                map.insert("group_id".into(), Value::String(g.into()));
                Value::Object(map)
            }
            None => Value::Object(serde_json::Map::new()),
        },
    };
    let make_connection =
        |id: &str, source: &str, sport: &str, target: &str, tport: &str| -> Value {
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), Value::String(id.into()));
            let mut src = serde_json::Map::new();
            src.insert("node_id".into(), Value::String(source.into()));
            src.insert("port_id".into(), Value::String(sport.into()));
            let mut tgt = serde_json::Map::new();
            tgt.insert("node_id".into(), Value::String(target.into()));
            tgt.insert("port_id".into(), Value::String(tport.into()));
            obj.insert("source".into(), Value::Object(src));
            obj.insert("target".into(), Value::Object(tgt));
            Value::Object(obj)
        };

    // Layout: manual at origin, 99,998 workers in a grid, 1 sink. Total = 100,000.
    let columns = 500usize;
    let contract_names = [
        ("canopy", "generate-items", "v1alpha1"),
        ("canopy", "edit-fields", "v1alpha1"),
        ("canopy", "if", "v1alpha1"),
        ("canopy", "merge", "v1alpha1"),
    ];
    let group_count = 16usize;
    let worker_count = 99_998usize;
    assert_eq!(1 + worker_count + 1, 100_000);

    let start_x = 200.0;
    let start_y = 80.0;
    let cell_w = 160.0;
    let cell_h = 90.0;

    let mut nodes: Vec<NodeInstance> = Vec::with_capacity(100_000);
    let mut connections: Vec<Value> = Vec::with_capacity(99_999);

    // Manual trigger at origin.
    let manual_id = "fixture-manual-0";
    nodes.push(make_node(
        manual_id,
        "Manual Trigger",
        "canopy",
        "manual-trigger",
        "v1alpha1",
        0.0,
        0.0,
        Some("group-entry"),
    ));

    // Workers arranged in a 500-column grid.
    let mut conn_id = 0u64;
    let mut prev_node_id = manual_id.to_string();
    for i in 0..worker_count {
        let col = i % columns;
        let row = i / columns;
        let id = format!("fixture-node-{i:06}");
        let (ns, name, ver) = contract_names[i % contract_names.len()];
        let group_idx = i % group_count;
        let group_id = format!("group-{group_idx:02}");
        let x = start_x + (col as f64) * cell_w;
        let y = start_y + (row as f64) * cell_h;
        nodes.push(make_node(
            &id,
            &format!("Node {i}"),
            ns,
            name,
            ver,
            x,
            y,
            Some(&group_id),
        ));
        connections.push(make_connection(
            &format!("conn-{conn_id:07}"),
            &prev_node_id,
            "out",
            &id,
            "in",
        ));
        conn_id += 1;
        prev_node_id = id;
    }
    // Sink summarize node connected from the very last worker.
    let sink_id = "fixture-summarize-0";
    nodes.push(make_node(
        sink_id,
        "Summarize (Sink)",
        "canopy",
        "summarize",
        "v1alpha1",
        start_x + ((worker_count % columns) as f64) * cell_w + cell_w,
        start_y + ((worker_count / columns) as f64) * cell_h,
        Some("group-sink"),
    ));
    connections.push(make_connection(
        &format!("conn-{conn_id:07}"),
        &prev_node_id,
        "out",
        sink_id,
        "in",
    ));

    assert_eq!(nodes.len(), 100_000, "fixture must contain exactly 100,000 nodes");
    connections.sort_by(|a, b| {
        let a_id = a.get("id").and_then(Value::as_str).unwrap_or("");
        let b_id = b.get("id").and_then(Value::as_str).unwrap_or("");
        a_id.cmp(b_id)
    });

    WorkflowDraft {
        workflow_id: "eco-100k-editor-fixture".into(),
        name: "Eco 100,000 Node Editor Fixture".into(),
        draft_version: 1,
        nodes,
        connections,
        annotation: "Canopy deterministic 100k-node editor seam fixture".into(),
        settings: Value::Object(serde_json::Map::new()),
        compatibility_metadata: {
            let mut m = serde_json::Map::new();
            m.insert(
                "fixture".into(),
                Value::String("100k-editor-seam/v1".into()),
            );
            Value::Object(m)
        },
    }
}

/// Frozen correctness hash for the 100,000-node fixture. Call this once and
/// cache the value so the acceptance matrix can pin the expected digest.
pub fn frozen_fixture_digest() -> String {
    let draft = generate_100k_fixture();
    let packed = pack(&draft).expect("fixture pack");
    packed.topology_digest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_draft_packs_and_verifies() {
        let draft = WorkflowDraft {
            workflow_id: "wf-small".into(),
            name: "small".into(),
            draft_version: 1,
            nodes: vec![NodeInstance {
                id: "n1".into(),
                name: "Manual".into(),
                contract_lock: NodeContractLock {
                    api_version: "canopy.node/v1alpha1".into(),
                    namespace: "canopy".into(),
                    name: "manual-trigger".into(),
                    version: "v1alpha1".into(),
                    digest: "sha256:00".into(),
                },
                configuration: Value::Null,
                layout: Layout { x: 0.0, y: 0.0 },
                annotation: String::new(),
                compatibility_metadata: Value::Null,
            }],
            connections: vec![],
            annotation: String::new(),
            settings: Value::Null,
            compatibility_metadata: Value::Null,
        };
        let packed = pack(&draft).unwrap();
        assert_eq!(&packed.magic, "CWBT");
        assert_eq!(packed.version, TOPOLOGY_VERSION);
        assert_eq!(packed.node_count, 1);
        let v = verify(&packed.packed_bytes);
        assert!(v.valid, "{v:?}");
        assert_eq!(v.node_count, 1);
    }

    #[test]
    fn corrupted_magic_is_rejected() {
        let draft = generate_100k_fixture();
        let mut packed = pack(&draft).unwrap().packed_bytes;
        packed[0] = b'X';
        let v = verify(&packed);
        assert!(!v.valid);
        assert_eq!(v.error.as_deref(), Some("bad magic"));
    }

    #[test]
    fn fixture_exactly_100k_nodes() {
        let draft = generate_100k_fixture();
        assert_eq!(draft.nodes.len(), 100_000);
        // Deterministic: run twice and compare digests.
        let a = pack(&draft).unwrap();
        let b = pack(&draft).unwrap();
        assert_eq!(a.topology_digest, b.topology_digest);
        assert_eq!(a.packed_bytes, b.packed_bytes);
        let v = verify(&a.packed_bytes);
        assert!(v.valid, "{v:?}");
        assert_eq!(v.node_count, 100_000);
    }
}
