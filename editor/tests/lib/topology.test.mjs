// SPDX-License-Identifier: AGPL-3.0-or-later

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import {
  SECTION_NODES,
  parseTopology,
  verifyTopology,
} from "./topology-loader.mjs";

const FIXTURE_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "fixtures",
);

test("verifyTopology rejects an empty buffer", () => {
  const v = verifyTopology(new ArrayBuffer(0));
  assert.equal(v.valid, false);
  assert.equal(v.error, "header too short");
});

test("verifyTopology rejects bad magic", () => {
  const buf = new Uint8Array(13);
  buf.set([0x58, 0x57, 0x42, 0x54]);
  const v = verifyTopology(buf.buffer);
  assert.equal(v.valid, false);
  assert.equal(v.error, "bad magic");
});

test("packs a minimal synthetic topology in-memory", () => {
  const encoder = new TextEncoder();
  const nodesPayload = JSON.stringify([
    {
      id: "n1",
      name: "Manual",
      contract: "canopy/manual-trigger/v1alpha1",
      groupId: null,
      x: 0,
      y: 0,
      searchableText: "manual n1",
    },
  ]);
  const nodesBytes = encoder.encode(nodesPayload);
  const digestBody = "canopy.topology-digest/v1:1.0.0." + "0".repeat(64);
  const digestBytes = encoder.encode(digestBody);
  const total = 13 + digestBytes.length + 9 + nodesBytes.length;
  const buf = new Uint8Array(total);
  buf.set([0x43, 0x57, 0x42, 0x54, 0x01]);
  const view = new DataView(buf.buffer);
  view.setUint32(5, total, true);
  view.setUint32(9, digestBytes.length, true);
  buf.set(digestBytes, 13);
  const offset = 13 + digestBytes.length;
  buf[offset] = SECTION_NODES;
  view.setUint32(offset + 1, 1, true);
  view.setUint32(offset + 5, nodesBytes.length, true);
  buf.set(nodesBytes, offset + 9);
  const v = verifyTopology(buf.buffer);
  assert.equal(v.valid, true);
  assert.equal(v.nodeCount, 1);
});

test("parses a generated 100k fixture if present", async (t) => {
  const fixturePath = join(FIXTURE_DIR, "eco-100k-editor-fixture.cwbt");
  let exists = false;
  try {
    exists = (await stat(fixturePath)).isFile();
  } catch {
    exists = false;
  }
  if (!exists) {
    t.skip("run `workflowd generate-100k-fixture` first");
    return;
  }
  const binary = await readFile(fixturePath);
  const buffer = binary.buffer.slice(binary.byteOffset, binary.byteOffset + binary.byteLength);
  const v = verifyTopology(buffer);
  assert.equal(v.valid, true, `verification failed: ${v.error}`);
  assert.equal(v.nodeCount, 100_000);
  assert.ok(v.connectionCount >= 99_999);
  const start = performance.now();
  const parsed = parseTopology(buffer);
  const elapsed = performance.now() - start;
  assert.equal(parsed.sections.nodes.length, 100_000);
  assert.equal(parsed.nodeIdToIndex.size, 100_000);
  assert.ok(elapsed < 1_500, `parse took ${elapsed}ms`);
});
