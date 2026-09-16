// SPDX-License-Identifier: AGPL-3.0-or-later

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { import_n8n_v2, MAX_IMPORT_BYTES } from "./n8n-import.mjs";

const FIXTURES = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "fixtures",
);

function loadFixture(name) {
  return readFileSync(join(FIXTURES, name));
}

test("n8n 2.39.0 first-subset fixture imports into a draft", () => {
  const bytes = loadFixture("n8n-hello-world.v2.json");
  const { draft, report } = import_n8n_v2("wf-n8n-hello", bytes);
  assert.equal(draft.workflow_id, "wf-n8n-hello");
  assert.equal(draft.name, "n8n 2.39.0 hello");
  assert.equal(draft.nodes.length, 2);
  // Manual trigger native, set preserved opaque.
  assert.equal(report.classifications.native_equivalent, 1);
  assert.equal(report.classifications.preserved_opaque, 1);
  assert.equal(report.blocked, false);
  // Exact external ids preserved.
  assert.deepEqual(draft.nodes.map(n => n.id), ["manual-1", "set-1"]);
  // n8n_type / n8n_type_version retained for round-trip.
  for (const n of draft.nodes) {
    assert.ok(n.compatibility_metadata.n8n_type);
    assert.equal(typeof n.compatibility_metadata.n8n_type_version, "number");
  }
  // Connections normalized.
  assert.equal(draft.connections.length, 1);
  assert.equal(draft.connections[0].source.node_id, "manual-1");
  assert.equal(draft.connections[0].target.node_id, "set-1");
});

test("credentials block and secret-shaped parameter keys are redacted fail-closed", () => {
  const doc = {
    name: "secretful",
    nodes: [{
      id: "http-1",
      name: "HTTP Request",
      type: "n8n-nodes-base.httpRequest",
      typeVersion: 4,
      position: [200, 200],
      credentials: { httpBasicAuth: { user: "u", password: "LEAK" } },
      parameters: { url: "https://example.com", apiKey: "S3CR3T", body: {} },
    }],
    connections: {},
    settings: { saveManualExecutions: true },
  };
  const { draft, report } = import_n8n_v2("wf", Buffer.from(JSON.stringify(doc)));
  assert.equal(draft.nodes[0].configuration.apiKey, "[REDACTED]");
  assert.ok(report.redactions.some(r => r.path === "credentials"));
  assert.ok(report.redactions.some(r => r.path.endsWith(".apiKey")));
});

test("large base64 blobs are redacted; oversized strings are rejected", () => {
  const big = Buffer.alloc(9000, "A").toString();
  const doc = {
    name: "blob",
    nodes: [{ id: "1", name: "n", type: "t", typeVersion: 1, parameters: { data: big } }],
    connections: {},
  };
  assert.throws(() => import_n8n_v2("wf", Buffer.from(JSON.stringify(doc))), /exceeds/);

  // Just over 1024 chars of pure base64 alphabet -> redacted, not thrown.
  const b64 = Buffer.alloc(1500, "Q").toString();
  const doc2 = { name: "x", nodes: [{ id: "1", name: "n", type: "t", typeVersion: 1, parameters: { blob: b64 } }], connections: {} };
  const { draft } = import_n8n_v2("wf", Buffer.from(JSON.stringify(doc2)));
  assert.equal(draft.nodes[0].configuration.blob, "[REDACTED]");
});

test("unsafe node (executeCommand) blocks the import before a draft is returned", () => {
  const doc = {
    name: "evil",
    nodes: [
      { id: "m", name: "Manual", type: "n8n-nodes-base.manualTrigger", typeVersion: 1, position: [0,0], parameters: {} },
      { id: "sh", name: "Shell", type: "n8n-nodes-base.executeCommand", typeVersion: 1, position: [200,0], parameters: { command: "id" } },
    ],
    connections: { "m": { main: [[{ node: "sh", type: "main", index: 0 }]] } },
  };
  assert.throws(() => import_n8n_v2("wf", Buffer.from(JSON.stringify(doc))), /executeCommand/);
});

test("dangling connection is dropped and reported as adapted", () => {
  const doc = {
    name: "dangle",
    nodes: [{ id: "m", name: "Manual", type: "n8n-nodes-base.manualTrigger", typeVersion: 1, position: [0,0], parameters: {} }],
    connections: { "m": { main: [[{ node: "missing", type: "main", index: 0 }]] } },
  };
  const { draft, report } = import_n8n_v2("wf", Buffer.from(JSON.stringify(doc)));
  assert.equal(draft.connections.length, 0);
  assert.ok(report.findings.some(f => f.code === "dangling_connection"));
});

test("size cap is enforced", () => {
  const doc = { name: "huge", nodes: [], connections: {} };
  const base = JSON.stringify(doc);
  const pad = "x".repeat(MAX_IMPORT_BYTES);
  assert.throws(() => import_n8n_v2("wf", Buffer.from(base.slice(0, -2) + `,"pad":"${pad}"}`)), /exceeds/);
});
