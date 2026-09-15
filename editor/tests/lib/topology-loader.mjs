// SPDX-License-Identifier: AGPL-3.0-or-later

// JS mirror of src/topology-loader.ts for node:test. Keep in sync manually.
// Tests validate the wire format; the TypeScript version is the production
// bundle consumed by the editor.

export const TOPOLOGY_MAGIC = [0x43, 0x57, 0x42, 0x54];
export const TOPOLOGY_VERSION = 1;
export const SECTION_NODES = 1;
export const SECTION_CONNECTIONS = 2;
export const SECTION_GROUPS = 3;
export const SECTION_ANNOTATIONS = 4;

export function verifyTopology(buffer) {
  const data = new Uint8Array(buffer);
  if (data.byteLength < 13)
    return { valid: false, version: 0, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "header too short" };
  for (let i = 0; i < 4; i++) {
    if (data[i] !== TOPOLOGY_MAGIC[i])
      return { valid: false, version: 0, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "bad magic" };
  }
  const version = data[4];
  if (version !== TOPOLOGY_VERSION)
    return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "unsupported version" };
  const totalBytes = readU32(data, 5);
  if (totalBytes !== data.byteLength)
    return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "declared length mismatch" };
  const digestLen = readU32(data, 9);
  let offset = 13 + digestLen;
  if (offset > data.byteLength)
    return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "digest exceeds blob" };
  let nodeCount = 0;
  let connectionCount = 0;
  let groupCount = 0;
  const seen = new Set();
  while (offset < data.byteLength) {
    if (offset + 9 > data.byteLength)
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "section header truncated" };
    const tag = data[offset];
    const count = readU32(data, offset + 1);
    const bytes = readU32(data, offset + 5);
    offset += 9 + bytes;
    if (offset > data.byteLength)
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "section payload exceeds blob" };
    if (seen.has(tag))
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "duplicate section" };
    seen.add(tag);
    if (tag === SECTION_NODES) nodeCount = count;
    else if (tag === SECTION_CONNECTIONS) connectionCount = count;
    else if (tag === SECTION_GROUPS) groupCount = count;
    else if (tag !== SECTION_ANNOTATIONS)
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "unknown section tag" };
  }
  return { valid: true, version, nodeCount, connectionCount, groupCount };
}

export function parseTopology(buffer) {
  const data = new Uint8Array(buffer);
  const v = verifyTopology(buffer);
  if (!v.valid) throw new Error(`topology verification failed: ${v.error}`);
  const digestLen = readU32(data, 9);
  let offset = 13 + digestLen;
  const sections = { nodes: [], connections: [], groups: [], annotation: "" };
  const decoder = new TextDecoder("utf-8");
  while (offset < data.byteLength) {
    const tag = data[offset];
    const count = readU32(data, offset + 1);
    const bytes = readU32(data, offset + 5);
    const start = offset + 9;
    const end = start + bytes;
    const payload = JSON.parse(decoder.decode(data.subarray(start, end)));
    if (tag === SECTION_NODES) sections.nodes = payload;
    else if (tag === SECTION_CONNECTIONS) sections.connections = payload;
    else if (tag === SECTION_GROUPS) sections.groups = payload;
    else if (tag === SECTION_ANNOTATIONS) sections.annotation = Array.isArray(payload) ? payload[0] : payload;
    if (tag <= SECTION_GROUPS && Array.isArray(payload) && payload.length !== count)
      throw new Error("section count mismatch");
    offset = end;
  }
  const nodeIdToIndex = new Map();
  for (let i = 0; i < sections.nodes.length; i++) nodeIdToIndex.set(sections.nodes[i].id, i);
  return { version: v.version, sections, nodeIdToIndex };
}

function readU32(data, offset) {
  return (data[offset] | (data[offset + 1] << 8) | (data[offset + 2] << 16) | (data[offset + 3] << 24)) >>> 0;
}
