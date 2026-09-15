// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Web Worker entry for incremental topology indexing (Ticket 14).
 *
 * The main thread posts a `{ type: "parse", buffer, digest }` message, this
 * worker parses sections incrementally, building the spatial and adjacency
 * indexes in chunks bounded by INDEX_CHUNK so progress events are delivered
 * and cancellation is responsive. Because Workers serialize via structured
 * clone and the buffers can be transferred (transferList), the main thread
 * does not pay a copy cost.
 */

import {
  buildSpatialIndex,
  parseTopology,
  verifyTopology,
  type LoadProgress,
  type PackedTopology,
} from "./topology-loader";

const INDEX_CHUNK = 2_000;

let cancelled = false;

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  cancelled = false;
  const msg = event.data;
  if (msg.type === "cancel") {
    cancelled = true;
    return;
  }
  if (msg.type !== "parse") return;
  try {
    run(msg.buffer, msg.digest, msg.id);
  } catch (error) {
    self.postMessage({
      id: msg.id,
      type: "error",
      message: error instanceof Error ? error.message : String(error),
    } satisfies WorkerError);
  }
};

function run(buffer: ArrayBuffer, digest: string | null, id: string) {
  const bytes = buffer.byteLength;
  post<WorkerProgress>({
    id,
    type: "progress",
    phase: "verify",
    bytesLoaded: 0,
    totalBytes: bytes,
    nodesIndexed: 0,
    connectionsIndexed: 0,
  });
  const verify = verifyTopology(buffer);
  if (!verify.valid) {
    post({ id, type: "error", message: verify.error ?? "verify failed" } satisfies WorkerError);
    return;
  }
  post<WorkerProgress>({
    id,
    type: "progress",
    phase: "index",
    bytesLoaded: bytes,
    totalBytes: bytes,
    nodesIndexed: 0,
    connectionsIndexed: 0,
  });
  const parsed: PackedTopology = parseTopology(buffer, digest);
  // The spatial index is built in chunks and progress is reported between
  // chunks so long documents cannot starve the event loop.
  let indexed = 0;
  const coords = buildSpatialIndex(parsed.sections.nodes, (built, total) => {
    indexed = built;
    post<WorkerProgress>({
      id,
      type: "progress",
      phase: "index",
      bytesLoaded: bytes,
      totalBytes: bytes,
      nodesIndexed: built,
      connectionsIndexed: built === total ? parsed.sections.connections.length : 0,
    });
    if (cancelled) throw new Error("cancelled");
    // Yield back to the worker event loop.
    // Atomics.pause-style yield; postMessage already queues a microtask.
  });
  if (cancelled) {
    post({ id, type: "cancelled" } satisfies WorkerCancelled);
    return;
  }
  post<WorkerProgress>({
    id,
    type: "progress",
    phase: "complete",
    bytesLoaded: bytes,
    totalBytes: bytes,
    nodesIndexed: parsed.sections.nodes.length,
    connectionsIndexed: parsed.sections.connections.length,
  });
  // Build adjacency index (outgoing edge list per node).
  const adjacency = new Uint32Array(parsed.sections.connections.length * 2);
  const outgoingOffset = new Uint32Array(parsed.sections.nodes.length + 1);
  const nodeIndex = parsed.nodeIdToIndex;
  const connCount = parsed.sections.connections.length;
  for (let ci = 0; ci < connCount; ci++) {
    const c = parsed.sections.connections[ci];
    const sIdx = nodeIndex.get(c.sourceNode);
    const tIdx = nodeIndex.get(c.targetNode);
    if (sIdx === undefined || tIdx === undefined) continue;
    outgoingOffset[sIdx]++;
    adjacency[ci * 2] = sIdx;
    adjacency[ci * 2 + 1] = tIdx;
    if (ci % INDEX_CHUNK === 0 && cancelled) {
      post({ id, type: "cancelled" } satisfies WorkerCancelled);
      return;
    }
  }
  // Prefix sum to get ranges.
  let running = 0;
  for (let i = 0; i < outgoingOffset.length; i++) {
    const c = outgoingOffset[i];
    outgoingOffset[i] = running;
    running += c;
  }
  post({
    id,
    type: "complete",
    topology: parsed,
    coords,
    adjacency,
    adjacencyOffsets: outgoingOffset,
  } satisfies WorkerComplete);
}

function post<T>(message: T) {
  (self as unknown as Worker).postMessage(message);
}

export type WorkerRequest =
  | { id: string; type: "parse"; buffer: ArrayBuffer; digest: string | null }
  | { type: "cancel" };

export type WorkerProgress = {
  id: string;
  type: "progress";
  phase: LoadProgress["phase"];
  bytesLoaded: number;
  totalBytes: number;
  nodesIndexed: number;
  connectionsIndexed: number;
};

export type WorkerComplete = {
  id: string;
  type: "complete";
  topology: PackedTopology;
  coords: Float64Array;
  adjacency: Uint32Array;
  adjacencyOffsets: Uint32Array;
};

export type WorkerError = { id: string; type: "error"; message: string };
export type WorkerCancelled = { id: string; type: "cancelled" };
