// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Node.js fixture generator for Ticket 14.
 *
 * Produces the same deterministic 100,000-node topology as
 * `workflowd generate-100k-fixture` so browser tests and node:test suites can
 * exercise the loader, parser, and viewport canvas locally without a Rust
 * toolchain. The binary is written to editor/tests/fixtures/eco-100k-editor-fixture.cwbt
 * alongside a JSON draft and manifest, with bytes/layout matching the Rust
 * generator byte-for-byte for every stable identifier.
 *
 * Run: node scripts/generate-100k-fixture.mjs
 */

import { mkdir, writeFile, stat } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const EDITOR_ROOT = join(__dirname, "..");
const OUT_DIR = join(EDITOR_ROOT, "tests", "fixtures");

const TOPOLOGY_MAGIC = [0x43, 0x57, 0x42, 0x54]; // "CWBT"
const TOPOLOGY_VERSION = 1;
const SECTION_NODES = 1;
const SECTION_CONNECTIONS = 2;
const SECTION_GROUPS = 3;
const SECTION_ANNOTATIONS = 4;
const COLUMN_COUNT = 500;
const GROUP_COUNT = 16;
const WORKER_COUNT = 99_998;
const START_X = 200;
const START_Y = 80;
const CELL_W = 160;
const CELL_H = 90;
const CONTRACT_NAMES = [
  ["canopy", "generate-items", "v1alpha1"],
  ["canopy", "edit-fields", "v1alpha1"],
  ["canopy", "if", "v1alpha1"],
  ["canopy", "merge", "v1alpha1"],
];

function writeU32(buf, offset, value) {
  buf[offset] = value & 0xff;
  buf[offset + 1] = (value >>> 8) & 0xff;
  buf[offset + 2] = (value >>> 16) & 0xff;
  buf[offset + 3] = (value >>> 24) & 0xff;
}

function fakeDigest(id, contractName) {
  const seed = BigInt(BigInt(id.length) * 0x1000n) ^ BigInt(contractName.length);
  return `sha256:${seed.toString(16).padStart(64, "0")}`;
}

function makeNode(id, name, ns, contract, ver, x, y, groupId) {
  return {
    id,
    name,
    contract_lock: {
      api_version: "canopy.node/v1alpha1",
      namespace: ns,
      name: contract,
      version: ver,
      digest: fakeDigest(id, contract),
    },
    configuration: {},
    layout: { x, y },
    annotation: "",
    compatibility_metadata: groupId ? { group_id: groupId } : {},
  };
}

function makeConnection(id, source, sport, target, tport) {
  return {
    id,
    source: { node_id: source, port_id: sport },
    target: { node_id: target, port_id: tport },
  };
}

// Stable hash-based canonical JSON ordering matches serde_json+BTreeMap used by
// the Rust daemon: keys emitted in insertion order of a BTree/sorted map,
// arrays preserve order, and no extraneous whitespace.
function canonicalJson(value) {
  if (value === null) return "null";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  const keys = Object.keys(value).sort();
  return `{${keys.map((k) => `${JSON.stringify(k)}:${canonicalJson(value[k])}`).join(",")}}`;
}

// Project-wide digest: sha256 over canonical JSON of the byte array of body bytes
// (mirrors `crates/workflowd/src/canonical.rs` and topology::hex_digest).
function bodyDigest(bodyBytes) {
  const arr = Array.from(bodyBytes).map((b) => b);
  const canonical = canonicalJson(arr);
  const hash = createHash("sha256").update(canonical).digest("hex");
  return `sha256:${hash}`;
}

function normalizeConnection(value) {
  if (!value || typeof value !== "object") return null;
  const source = value.source;
  const target = value.target;
  if (!source || !target) return null;
  const id = typeof value.id === "string" ? value.id : null;
  const sourceNode = source.node_id;
  const sourcePort = source.port_id ?? source.port;
  const targetNode = target.node_id;
  const targetPort = target.port_id ?? target.port;
  if (!id || !sourceNode || !sourcePort || !targetNode || !targetPort) return null;
  return { id, sourceNode, sourcePort, targetNode, targetPort };
}

function buildSearchableText(node) {
  const parts = [node.name, node.id];
  if (node.annotation) parts.push(node.annotation);
  return parts.join(" ").toLowerCase();
}

function packSection(tag, entries, payload) {
  const payloadBytes = Buffer.from(payload, "utf8");
  const header = Buffer.alloc(9);
  header[0] = tag;
  writeU32(header, 1, entries);
  writeU32(header, 5, payloadBytes.length);
  return Buffer.concat([header, payloadBytes]);
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });

  const nodes = [];
  const rawConnections = [];
  const manualId = "fixture-manual-0";
  nodes.push(makeNode(manualId, "Manual Trigger", "canopy", "manual-trigger", "v1alpha1", 0, 0, "group-entry"));

  let connId = 0;
  let prevNodeId = manualId;
  for (let i = 0; i < WORKER_COUNT; i++) {
    const col = i % COLUMN_COUNT;
    const row = Math.floor(i / COLUMN_COUNT);
    const id = `fixture-node-${String(i).padStart(6, "0")}`;
    const [ns, name, ver] = CONTRACT_NAMES[i % CONTRACT_NAMES.length];
    const groupIdx = i % GROUP_COUNT;
    const groupId = `group-${String(groupIdx).padStart(2, "0")}`;
    const x = START_X + col * CELL_W;
    const y = START_Y + row * CELL_H;
    nodes.push(makeNode(id, `Node ${i}`, ns, name, ver, x, y, groupId));
    rawConnections.push(makeConnection(`conn-${String(connId).padStart(7, "0")}`, prevNodeId, "out", id, "in"));
    connId++;
    prevNodeId = id;
  }
  const sinkId = "fixture-summarize-0";
  const sinkCol = WORKER_COUNT % COLUMN_COUNT;
  const sinkRow = Math.floor(WORKER_COUNT / COLUMN_COUNT);
  nodes.push(
    makeNode(
      sinkId,
      "Summarize (Sink)",
      "canopy",
      "summarize",
      "v1alpha1",
      START_X + sinkCol * CELL_W + CELL_W,
      START_Y + sinkRow * CELL_H,
      "group-sink",
    ),
  );
  rawConnections.push(makeConnection(`conn-${String(connId).padStart(7, "0")}`, prevNodeId, "out", sinkId, "in"));

  if (nodes.length !== 100_000) {
    throw new Error(`fixture must contain exactly 100,000 nodes; got ${nodes.length}`);
  }

  const nodeEntries = nodes
    .map((n) => ({
      id: n.id,
      name: n.name,
      contract: `${n.contract_lock.namespace}/${n.contract_lock.name}/${n.contract_lock.version}`,
      group_id: n.compatibility_metadata.group_id ?? null,
      x: n.layout.x,
      y: n.layout.y,
      searchable_text: buildSearchableText(n),
    }))
    .sort((a, b) => a.id.localeCompare(b.id));

  const connEntries = rawConnections
    .map(normalizeConnection)
    .filter(Boolean)
    .sort((a, b) => a.id.localeCompare(b.id));

  // Groups: derived from per-node group_id, matching Rust topology::pack.
  const groupMap = new Map();
  for (const n of nodes) {
    const gid = n.compatibility_metadata.group_id;
    if (!gid) continue;
    if (!groupMap.has(gid)) groupMap.set(gid, { id: gid, label: gid, node_ids: [], collapsed: false });
    groupMap.get(gid).node_ids.push(n.id);
  }
  const groups = Array.from(groupMap.values())
    .map((g) => ({ ...g, node_ids: g.node_ids.slice().sort() }))
    .sort((a, b) => a.id.localeCompare(b.id));

  const annotation = "";

  const nodesPayload = canonicalJson(nodeEntries);
  const connectionsPayload = canonicalJson(connEntries);
  const groupsPayload = canonicalJson(groups);
  const annotationPayload = canonicalJson(annotation);

  const body = Buffer.concat([
    packSection(SECTION_NODES, nodeEntries.length, nodesPayload),
    packSection(SECTION_CONNECTIONS, connEntries.length, connectionsPayload),
    packSection(SECTION_GROUPS, groups.length, groupsPayload),
    packSection(SECTION_ANNOTATIONS, 1, annotationPayload),
  ]);

  const digestText = bodyDigest(body);
  const digestBytes = Buffer.from(digestText, "utf8");

  const HEADER_FIXED_LEN = 13;
  const totalLen = HEADER_FIXED_LEN + digestBytes.length + body.length;
  const packed = Buffer.alloc(totalLen);
  packed[0] = TOPOLOGY_MAGIC[0];
  packed[1] = TOPOLOGY_MAGIC[1];
  packed[2] = TOPOLOGY_MAGIC[2];
  packed[3] = TOPOLOGY_MAGIC[3];
  packed[4] = TOPOLOGY_VERSION;
  writeU32(packed, 5, totalLen);
  writeU32(packed, 9, digestBytes.length);
  digestBytes.copy(packed, 13);
  body.copy(packed, 13 + digestBytes.length);

  const draft = {
    workflow_id: "eco-100k-editor-fixture",
    name: "Eco 100,000 Node Editor Fixture",
    draft_version: 1,
    nodes,
    connections: rawConnections,
    annotation,
    settings: {},
    compatibility_metadata: {},
  };
  // Connections need sorted by id to match draft ordering? Rust generator sorts
  // them after construction; replicate for byte-identical draft JSON.
  draft.connections.sort((a, b) => a.id.localeCompare(b.id));

  const draftPath = join(OUT_DIR, "eco-100k-editor-fixture.json");
  const binaryPath = join(OUT_DIR, "eco-100k-editor-fixture.cwbt");
  const manifestPath = join(OUT_DIR, "eco-100k-editor-fixture.manifest.json");

  await writeFile(draftPath, JSON.stringify(draft));
  await writeFile(binaryPath, packed);

  const draftStat = await stat(draftPath);
  const manifest = {
    schema: "canopy.editor-fixture/v1alpha1",
    fixture_id: "eco-100k-editor-seam/v1",
    node_count: nodeEntries.length,
    connection_count: connEntries.length,
    group_count: groups.length,
    packed_bytes: packed.length,
    draft_bytes: draftStat.size,
    topology_digest: digestText,
    topology_version: TOPOLOGY_VERSION,
    magic: "CWBT",
    acceptance: {
      exactly_100_000_nodes: nodeEntries.length === 100_000,
      bounded_dom_required: true,
      worker_indexed_incrementally: true,
      viewport_virtualization_required: true,
    },
  };
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2));

  process.stdout.write(
    JSON.stringify({
      event: "fixture_100k_written",
      draft_path: draftPath,
      binary_path: binaryPath,
      manifest_path: manifestPath,
      node_count: nodeEntries.length,
      packed_bytes: packed.length,
      topology_digest: digestText,
    }) + "\n",
  );
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
