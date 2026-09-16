// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Packed binary topology loader for Ticket 14.
 *
 * The daemon serves /api/v1/workflows/{id}/topology as
 * `application/vnd.canopy.topology+v1` with a stable magic prefix, version
 * byte, total length, section bounds, and a content digest. The browser runs
 * parsing inside a Web Worker so a 100,000-node document never blocks the
 * main thread. This file exposes the types and the wire-format helpers used
 * by the worker; the renderer lives in viewport-canvas.ts.
 */

export const TOPOLOGY_MAGIC = [0x43, 0x57, 0x42, 0x54]; // "CWBT"
export const TOPOLOGY_VERSION = 1;

export const SECTION_NODES = 1;
export const SECTION_CONNECTIONS = 2;
export const SECTION_GROUPS = 3;
export const SECTION_ANNOTATIONS = 4;

export interface TopologyNode {
  id: string;
  name: string;
  contract: string;
  group_id: string | null;
  x: number;
  y: number;
  searchable_text: string;
}

export interface TopologyConnection {
  id: string;
  source_node: string;
  source_port: string;
  target_node: string;
  target_port: string;
}

export interface TopologyGroup {
  id: string;
  label: string;
  node_ids: string[];
  collapsed: boolean;
}

export interface TopologySections {
  nodes: TopologyNode[];
  connections: TopologyConnection[];
  groups: TopologyGroup[];
  annotation: string;
}

export interface TopologyVerify {
  valid: boolean;
  version: number;
  nodeCount: number;
  connectionCount: number;
  groupCount: number;
  error?: string;
}

export interface LoadProgress {
  /** Opaque request id for correlating progress to a caller. */
  id?: string;
  phase: "fetch" | "verify" | "index" | "complete";
  bytesLoaded: number;
  totalBytes: number;
  nodesIndexed: number;
  connectionsIndexed: number;
}

export interface PackedTopology {
  version: number;
  totalBytes: number;
  topologyDigest: string;
  sections: TopologySections;
  nodeIdToIndex: Map<string, number>;
}

/**
 * Verify a packed buffer header and section bounds without parsing JSON. This
 * is a cheap linear scan so callers can reject corrupt/truncated payloads
 * before handing work to a Worker.
 */
export function verifyTopology(buffer: ArrayBuffer): TopologyVerify {
  const data = new Uint8Array(buffer);
  if (data.byteLength < 13) {
    return { valid: false, version: 0, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "header too short" };
  }
  for (let i = 0; i < 4; i++) {
    if (data[i] !== TOPOLOGY_MAGIC[i]) {
      return { valid: false, version: 0, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "bad magic" };
    }
  }
  const version = data[4];
  if (version !== TOPOLOGY_VERSION) {
    return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "unsupported version" };
  }
  const totalBytes = readU32(data, 5);
  if (totalBytes !== data.byteLength) {
    return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "declared length mismatch" };
  }
  const digestLen = readU32(data, 9);
  let offset = 13 + digestLen;
  if (offset > data.byteLength) {
    return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "digest exceeds blob" };
  }
  let nodeCount = 0;
  let connectionCount = 0;
  let groupCount = 0;
  const seen = new Set<number>();
  while (offset < data.byteLength) {
    if (offset + 9 > data.byteLength) {
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "section header truncated" };
    }
    const tag = data[offset];
    const count = readU32(data, offset + 1);
    const bytes = readU32(data, offset + 5);
    offset += 9 + bytes;
    if (offset > data.byteLength) {
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "section payload exceeds blob" };
    }
    if (seen.has(tag)) {
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "duplicate section" };
    }
    seen.add(tag);
    if (tag === SECTION_NODES) nodeCount = count;
    else if (tag === SECTION_CONNECTIONS) connectionCount = count;
    else if (tag === SECTION_GROUPS) groupCount = count;
    else if (tag !== SECTION_ANNOTATIONS) {
      return { valid: false, version, nodeCount: 0, connectionCount: 0, groupCount: 0, error: "unknown section tag" };
    }
  }
  return { valid: true, version, nodeCount, connectionCount, groupCount };
}

/**
 * Parse section JSON payloads out of a verified buffer. This is what the
 * Web Worker runs. Returns the full section arrays and a nodeId→index map
 * used by the canvas for O(1) lookup while panning.
 */
export function parseTopology(buffer: ArrayBuffer, digestHeader: string | null): PackedTopology {
  const data = new Uint8Array(buffer);
  const v = verifyTopology(buffer);
  if (!v.valid) {
    throw new Error(`topology verification failed: ${v.error}`);
  }
  const totalBytes = readU32(data, 5);
  const digestLen = readU32(data, 9);
  const digestBytes = utf8(data, 13, 13 + digestLen);
  if (digestHeader && digestBytes !== digestHeader) {
    throw new Error("topology digest header mismatch");
  }
  const decoder = new TextDecoder("utf-8");
  let offset = 13 + digestLen;
  let sections: TopologySections = {
    nodes: [],
    connections: [],
    groups: [],
    annotation: "",
  };
  while (offset < data.byteLength) {
    const tag = data[offset];
    const count = readU32(data, offset + 1);
    const bytes = readU32(data, offset + 5);
    const start = offset + 9;
    const end = start + bytes;
    const payload = JSON.parse(decoder.decode(data.subarray(start, end))) as unknown;
    switch (tag) {
      case SECTION_NODES:
        sections.nodes = payload as TopologyNode[];
        if (sections.nodes.length !== count) throw new Error("node count mismatch");
        break;
      case SECTION_CONNECTIONS:
        sections.connections = payload as TopologyConnection[];
        if (sections.connections.length !== count) throw new Error("connection count mismatch");
        break;
      case SECTION_GROUPS:
        sections.groups = payload as TopologyGroup[];
        if (sections.groups.length !== count) throw new Error("group count mismatch");
        break;
      case SECTION_ANNOTATIONS: {
        const arr = payload as [string] | string;
        sections.annotation = typeof arr === "string" ? arr : arr[0] ?? "";
        break;
      }
      default:
        throw new Error(`unknown section tag ${tag}`);
    }
    offset = end;
  }
  const nodeIdToIndex = new Map<string, number>();
  for (let i = 0; i < sections.nodes.length; i++) {
    nodeIdToIndex.set(sections.nodes[i].id, i);
  }
  return {
    version: v.version,
    totalBytes,
    topologyDigest: digestBytes,
    sections,
    nodeIdToIndex,
  };
}

export function buildSpatialIndex(
  nodes: TopologyNode[],
  onProgress?: (indexed: number, total: number) => void,
): Float64Array {
  // Flat buffer: x,y pairs in node order. Used by the viewport canvas to
  // cull off-screen nodes without touching the JS heap.
  const coords = new Float64Array(nodes.length * 2);
  const chunk = 2_000;
  for (let i = 0; i < nodes.length; i += chunk) {
    const end = Math.min(i + chunk, nodes.length);
    for (let j = i; j < end; j++) {
      coords[j * 2] = nodes[j].x;
      coords[j * 2 + 1] = nodes[j].y;
    }
    if (onProgress) onProgress(end, nodes.length);
  }
  return coords;
}

export function searchNodes(
  nodes: TopologyNode[],
  query: string,
  limit = 48,
): Array<{ index: number; node: TopologyNode }> {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  const results: Array<{ index: number; node: TopologyNode }> = [];
  for (let i = 0; i < nodes.length && results.length < limit; i++) {
    if (nodes[i].searchable_text.includes(needle) || nodes[i].id.toLowerCase().includes(needle)) {
      results.push({ index: i, node: nodes[i] });
    }
  }
  return results;
}

function readU32(data: Uint8Array, offset: number): number {
  return (
    (data[offset] |
      (data[offset + 1] << 8) |
      (data[offset + 2] << 16) |
      (data[offset + 3] << 24)) >>>
    0
  );
}

function utf8(data: Uint8Array, start: number, end: number): string {
  return new TextDecoder("utf-8").decode(data.subarray(start, end));
}
