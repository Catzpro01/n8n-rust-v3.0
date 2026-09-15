// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Ticket 14 — standalone browser benchmark entry.
 *
 * Fetches the 100k-node packed topology (from the same origin, which in the
 * VPS perf profile is served by workflowd at `/api/v1/workflows/.../topology`,
 * or from a relative `/fixtures/...cwbt` file when opened off a static host
 * for headless capture), runs the same Web Worker parse+index path as the
 * production editor, then animates the viewport while sampling fps, draw
 * counts, memory (where available), and DOM counts.
 *
 * Results are written to the page DOM and to `window.__benchmarkResult` so
 * Playwright can assert on them after the run completes.
 */

import "./styles.css";
import { BoundedResultList, ViewportCanvas } from "./viewport-canvas";
import { StructureDeck } from "./structure-deck";
import {
  parseTopology,
  searchNodes,
  verifyTopology,
  type LoadProgress,
  type PackedTopology,
} from "./topology-loader";
import type { RenderStats } from "./viewport-canvas";

const params = new URLSearchParams(location.search);
const workflowId = params.get("workflow") ?? "eco-100k-editor-fixture";
const source = params.get("source") ?? "api"; // "api" | "fixture"
const FRAME_SAMPLES = 240;
const ANIMATE_SECONDS = 8;

type BenchResult = {
  workflowId: string;
  source: string;
  userAgent: string;
  screen: { width: number; height: number; dpr: number };
  startedAt: number;
  loadedAt?: number;
  fetchMs?: number;
  verifyMs?: number;
  parseMs?: number;
  indexMs?: number;
  totalReadyMs?: number;
  nodeCount?: number;
  connectionCount?: number;
  groupCount?: number;
  packedBytes?: number;
  fps?: { avg: number; p50: number; p95: number; min: number; max: number };
  drawnCounts?: { avg: number; p50: number; p95: number; max: number };
  edgeCounts?: { avg: number; p50: number; max: number };
  memory?: { initial?: number; afterLoad?: number; afterAnim?: number };
  domCounts?: { structureRows: number; searchRows: number; liElements: number; totalElements: number };
  error?: string;
};

const result: BenchResult = {
  workflowId,
  source,
  userAgent: navigator.userAgent,
  screen: { width: window.innerWidth, height: window.innerHeight, dpr: window.devicePixelRatio },
  startedAt: performance.now(),
};
(window as unknown as { __benchmarkResult: BenchResult }).__benchmarkResult = result;

function el<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing element #${id}`);
  return node as T;
}

async function run() {
  const statusEl = el<HTMLElement>("bench-status");
  const metricsEl = el<HTMLPreElement>("bench-metrics");
  const canvas = el<HTMLCanvasElement>("bench-canvas");
  const deckHost = el<HTMLElement>("bench-deck");
  const searchHost = el<HTMLElement>("bench-search");

  statusEl.textContent = "Loading packed topology…";

  const memoryInitial = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize;
  result.memory = { initial: memoryInitial };

  let url: string;
  if (source === "fixture") {
    url = "/tests/fixtures/eco-100k-editor-fixture.cwbt";
  } else {
    url = `/api/v1/workflows/${encodeURIComponent(workflowId)}/topology`;
  }

  const tFetchStart = performance.now();
  let response: Response;
  try {
    response = await fetch(url);
  } catch (err) {
    // Fallback to static fixture endpoint if API is unavailable (offline mode).
    if (source === "api") {
      url = "/tests/fixtures/eco-100k-editor-fixture.cwbt";
      response = await fetch(url);
      result.source = "fixture-fallback";
    } else {
      throw err;
    }
  }
  if (!response.ok) {
    throw new Error(`fetch failed: HTTP ${response.status}`);
  }
  const digestHeader = response.headers.get("x-canopy-topology-digest");
  const buffer = await response.arrayBuffer();
  result.fetchMs = Number((performance.now() - tFetchStart).toFixed(3));
  result.packedBytes = buffer.byteLength;
  statusEl.textContent = "Verifying header…";

  const t0 = performance.now();
  const verify = verifyTopology(buffer);
  result.verifyMs = Number((performance.now() - t0).toFixed(3));
  if (!verify.valid) throw new Error(`verify failed: ${verify.error}`);

  statusEl.textContent = "Parsing sections on Worker…";

  let topology: PackedTopology;
  let parseMs: number | undefined;
  let indexMs: number | undefined;
  if (typeof Worker !== "undefined") {
    const workerResult = await parseOnWorker(buffer, digestHeader, (progress) => {
      statusEl.textContent = `Indexing… ${progress.nodesIndexed.toLocaleString()} / ${verify.nodeCount.toLocaleString()} nodes`;
    });
    topology = workerResult.topology;
    parseMs = workerResult.parseMs;
    indexMs = workerResult.indexMs;
  } else {
    const t1 = performance.now();
    topology = parseTopology(buffer, digestHeader);
    parseMs = Number((performance.now() - t1).toFixed(3));
    indexMs = 0;
  }

  const viewport = new ViewportCanvas(canvas);
  const deck = new StructureDeck(deckHost);
  const results = new BoundedResultList(searchHost);
  viewport.setTopology(topology);
  deck.setData(topology.sections.groups, topology.sections.nodes);
  const needle = "node 99";
  results.setItems(searchNodes(topology.sections.nodes, needle, 12));
  result.parseMs = parseMs;
  result.indexMs = indexMs;
  viewport.resize();

  result.nodeCount = topology.sections.nodes.length;
  result.connectionCount = topology.sections.connections.length;
  result.groupCount = topology.sections.groups.length;
  result.loadedAt = performance.now();
  result.totalReadyMs = Number((result.loadedAt - result.startedAt).toFixed(3));
  result.memory!.afterLoad = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize;

  statusEl.textContent = `Animating viewport for ${ANIMATE_SECONDS}s…`;

  const frames: number[] = [];
  const drawnCounts: number[] = [];
  const edgeCounts: number[] = [];
  let lastFrame = performance.now();
  const vx0 = 0;
  const vy0 = 0;
  const vx1 = topology.sections.nodes.reduce((m, n) => Math.max(m, n.x), 0);
  const vy1 = topology.sections.nodes.reduce((m, n) => Math.max(m, n.y), 0);
  const start = performance.now();

  await new Promise<void>((resolve) => {
    function frame(now: number) {
      const dt = now - lastFrame;
      lastFrame = now;
      if (dt > 0 && dt < 250) frames.push(1000 / dt);
      const t = Math.min(1, (now - start) / (ANIMATE_SECONDS * 1000));
      // Pan across the workflow over time, zoom oscillates gently.
      const zoom = 0.25 + 0.75 * (0.5 - 0.5 * Math.cos(t * Math.PI * 2));
      const ox = -(vx0 + (vx1 - vx0) * t) * zoom + window.innerWidth / 2;
      const oy = -(vy0 + (vy1 - vy0) * t * 0.5) * zoom + window.innerHeight / 2;
      viewport.setViewport({ offsetX: ox, offsetY: oy, zoom });
      const stats = viewport.lastStats();
      drawnCounts.push(stats.nodesInViewport);
      edgeCounts.push(stats.connectionsInViewport);
      if (frames.length > FRAME_SAMPLES) frames.shift();
      if (now - start < ANIMATE_SECONDS * 1000) {
        requestAnimationFrame(frame);
      } else {
        resolve();
      }
    }
    requestAnimationFrame(frame);
  });

  result.fps = summarize(frames);
  result.drawnCounts = summarize(drawnCounts);
  result.edgeCounts = summarize(edgeCounts);
  result.memory!.afterAnim = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize;
  const lis = document.querySelectorAll("li").length;
  result.domCounts = {
    structureRows: deckHost.querySelectorAll("li").length,
    searchRows: searchHost.querySelectorAll("li").length,
    liElements: lis,
    totalElements: document.querySelectorAll("*").length,
  };

  statusEl.textContent = "Complete — see metrics below.";
  metricsEl.textContent = JSON.stringify(result, null, 2);
  el<HTMLElement>("bench-pass").textContent = result.fps.avg >= 30 && result.drawnCounts.p95 < 5000 ? "PASS" : "REVIEW";
  el<HTMLElement>("bench-pass").dataset.status = result.fps.avg >= 30 && result.drawnCounts.p95 < 5000 ? "pass" : "review";
}

function summarize(values: number[]): { avg: number; p50: number; p95: number; min: number; max: number } {
  if (!values.length) return { avg: 0, p50: 0, p95: 0, min: 0, max: 0 };
  const sorted = values.slice().sort((a, b) => a - b);
  const sum = sorted.reduce((s, v) => s + v, 0);
  const p = (pct: number) => sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * pct))];
  return {
    avg: Number((sum / sorted.length).toFixed(2)),
    p50: Number(p(0.5).toFixed(2)),
    p95: Number(p(0.95).toFixed(2)),
    min: Number(sorted[0].toFixed(2)),
    max: Number(sorted[sorted.length - 1].toFixed(2)),
  };
}

function parseOnWorker(
  buffer: ArrayBuffer,
  digestHeader: string | null,
  onProgress: (p: LoadProgress) => void,
): Promise<{ topology: PackedTopology; parseMs?: number; indexMs?: number }> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./topology-worker.ts", import.meta.url), { type: "module" });
    const id = `bench-${Date.now()}`;
    const tParseStart = performance.now();
    let parseMs: number | undefined;
    let indexMs: number | undefined;
    worker.onmessage = (event: MessageEvent) => {
      const msg = event.data;
      if (msg.id !== id) return;
      if (msg.type === "progress") {
        onProgress(msg);
        if (msg.phase === "index" && msg.nodesIndexed === 0 && parseMs === undefined) {
          parseMs = Number((performance.now() - tParseStart).toFixed(3));
        }
      } else if (msg.type === "complete") {
        indexMs = Number((performance.now() - tParseStart - (parseMs ?? 0)).toFixed(3));
        worker.terminate();
        resolve({ ...msg.topology, parseMs, indexMs });
      } else if (msg.type === "error") {
        worker.terminate();
        reject(new Error(msg.message));
      }
    };
    worker.postMessage({ id, type: "parse", buffer, digest: digestHeader }, [buffer]);
  });
}

run().catch((error: Error) => {
  result.error = error.message;
  el<HTMLElement>("bench-status").textContent = `Error: ${error.message}`;
  el<HTMLPreElement>("bench-metrics").textContent = JSON.stringify(result, null, 2);
  el<HTMLElement>("bench-pass").textContent = "FAIL";
  el<HTMLElement>("bench-pass").dataset.status = "fail";
});
