// SPDX-License-Identifier: AGPL-3.0-or-later

import test from "node:test";
import assert from "node:assert/strict";

import { import_n8n_v2, MAX_IMPORT_BYTES } from "./n8n-import.mjs";
import {
  HELLO_WORLD,
  SECRETFUL,
  UNSAFE_EXEC_COMMAND,
  MALFORMED_NODES_STRING,
} from "./n8n-fixtures.mjs";

function bytes(s) {
  return Buffer.from(s);
}

test("n8n 2.39.0 first-subset fixture imports into a draft", () => {
  const { draft, report } = import_n8n_v2("wf-n8n-hello", bytes(HELLO_WORLD));
  assert.equal(draft.workflow_id, "wf-n8n-hello");
  assert.equal(draft.name, "n8n 2.39.0 hello");
  assert.equal(draft.nodes.length, 2);
  assert.equal(report.classifications.native_equivalent, 1);
  assert.equal(report.classifications.preserved_opaque, 1);
  assert.equal(report.blocked, false);
  assert.deepEqual(draft.nodes.map(n => n.id), ["manual-1", "set-1"]);
  for (const n of draft.nodes) {
    assert.ok(n.compatibility_metadata.n8n_type);
    assert.equal(typeof n.compatibility_metadata.n8n_type_version, "number");
    assert.equal(n.compatibility_metadata.import_source, "n8n.workflow-json/v2");
  }
  assert.equal(draft.connections.length, 1);
  assert.equal(draft.connections[0].source.node_id, "manual-1");
  assert.equal(draft.connections[0].target.node_id, "set-1");
  assert.equal(draft.compatibility_metadata.n8n_version_target, "2.39.0");
});

test("credentials block and secret-shaped parameter keys are redacted fail-closed", () => {
  const { draft, report } = import_n8n_v2("wf", bytes(SECRETFUL));
  assert.equal(draft.nodes[0].configuration.apiKey, "[REDACTED]");
  assert.equal(draft.nodes[0].configuration.headers.Authorization, "[REDACTED]");
  assert.ok(report.redactions.some(r => r.path === "credentials"));
  assert.ok(report.redactions.some(r => r.path.endsWith(".apiKey")));
});

test("malformed nodes field (string instead of array) is treated as zero nodes, not a crash", () => {
  const { draft } = import_n8n_v2("wf", bytes(MALFORMED_NODES_STRING));
  assert.equal(draft.nodes.length, 0);
  assert.equal(draft.name, "malformed (nodes not an array)");
});

test("large base64 blobs are redacted; oversized strings are rejected", () => {
  const big = "A".repeat(9000);
  const doc = { name: "blob", nodes: [{ id: "1", name: "n", type: "t", typeVersion: 1, parameters: { data: big } }], connections: {} };
  assert.throws(() => import_n8n_v2("wf", bytes(JSON.stringify(doc))), /exceeds/);

  const b64 = "Q".repeat(1500);
  const doc2 = { name: "x", nodes: [{ id: "1", name: "n", type: "t", typeVersion: 1, parameters: { blob: b64 } }], connections: {} };
  const { draft } = import_n8n_v2("wf", bytes(JSON.stringify(doc2)));
  assert.equal(draft.nodes[0].configuration.blob, "[REDACTED]");
});

test("unsafe node (executeCommand) blocks the import before a draft is returned", () => {
  assert.throws(() => import_n8n_v2("wf", bytes(UNSAFE_EXEC_COMMAND)), /executeCommand/);
});

test("dangling connection is dropped and reported as adapted", () => {
  const doc = {
    name: "dangle",
    nodes: [{ id: "m", name: "Manual", type: "n8n-nodes-base.manualTrigger", typeVersion: 1, position: [0,0], parameters: {} }],
    connections: { m: { main: [[{ node: "missing", type: "main", index: 0 }]] } },
  };
  const { draft, report } = import_n8n_v2("wf", bytes(JSON.stringify(doc)));
  assert.equal(draft.connections.length, 0);
  assert.ok(report.findings.some(f => f.code === "dangling_connection"));
  assert.equal(report.classifications.adapted, 1);
});

test("size cap is enforced", () => {
  const pad = "x".repeat(MAX_IMPORT_BYTES);
  assert.throws(() => import_n8n_v2("wf", bytes(`{"name":"x","nodes":[],"connections":{},"pad":"${pad}"}`)), /exceeds/);
});

test("canonical comparison preserves node/connection order (not sorted away)", () => {
  const doc = {
    name: "ordering",
    nodes: [
      { id: "b", name: "B", type: "t", typeVersion: 1, position: [0,0], parameters: { n: 2 } },
      { id: "a", name: "A", type: "t", typeVersion: 1, position: [100,0], parameters: { n: 1 } },
    ],
    connections: {},
  };
  const { draft } = import_n8n_v2("wf", bytes(JSON.stringify(doc)));
  assert.deepEqual(draft.nodes.map(n => n.id), ["b", "a"]);
});
