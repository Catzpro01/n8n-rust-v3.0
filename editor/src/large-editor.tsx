// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Ticket 14 — Large Editor (100k-node proof) UI shell.
 *
 * This component wires the packed-topology Web Worker, Structure Deck, 2D
 * viewport canvas, and Command Map search into Preact. It is a self-contained
 * panel mounted from main.tsx when a Workflow id is known; it never renders
 * 100k DOM nodes because:
 *
 *   - Node geometry is drawn on <canvas>, not DOM.
 *   - Structure Deck renders at most PAGE_SIZE (512) rows at a time.
 *   - Search results are capped at 12 visible rows.
 */

import { useEffect, useRef, useState } from "preact/hooks";
import { BoundedResultList, ViewportCanvas, type RenderStats } from "./viewport-canvas";
import { PAGE_SIZE, StructureDeck } from "./structure-deck";
import {
  parseTopology,
  searchNodes,
  type LoadProgress,
  type PackedTopology,
} from "./topology-loader";

type Props = {
  workflowId: string;
};

type State =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "progress"; progress: LoadProgress }
  | { phase: "ready"; topology: PackedTopology }
  | { phase: "error"; message: string };

const WORKER_CHUNK = 2_000;

export function LargeEditor({ workflowId }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const deckRef = useRef<HTMLDivElement | null>(null);
  const searchRef = useRef<HTMLDivElement | null>(null);
  const viewportRef = useRef<ViewportCanvas | null>(null);
  const deckInstRef = useRef<StructureDeck | null>(null);
  const resultsRef = useRef<BoundedResultList | null>(null);
  const workerRef = useRef<Worker | null>(null);
  const requestIdRef = useRef(0);
  const dragStateRef = useRef<{ startX: number; startY: number; ox: number; oy: number } | null>(null);
  const viewTransformRef = useRef({ offsetX: 0, offsetY: 0, zoom: 1 });
  const statsIntervalRef = useRef<number | null>(null);
  const evidenceRef = useRef<LargeEditorEvidence | null>(null);

  const [state, setState] = useState<State>({ phase: "idle" });
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [renderStats, setRenderStats] = useState<RenderStats>({ nodesInViewport: 0, connectionsInViewport: 0, drawCalls: 0 });

  // Mount: create viewport/deck/results instances and the Web Worker.
  useEffect(() => {
    const canvas = canvasRef.current;
    const deckHost = deckRef.current;
    const searchHost = searchRef.current;
    if (!canvas || !deckHost || !searchHost) return;

    const viewport = new ViewportCanvas(canvas);
    viewportRef.current = viewport;
    const deck = new StructureDeck(deckHost);
    deck.onSelectNode = (id) => focusNodeById(id);
    deckInstRef.current = deck;
    const results = new BoundedResultList(searchHost);
    results.onSelect = (id) => focusNodeById(id);
    resultsRef.current = results;

    const onResize = () => viewport.resize();
    window.addEventListener("resize", onResize);

    // Pointer handlers for pan and click selection.
    const onPointerDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      (event.target as Element).setPointerCapture?.(event.pointerId);
      const t = viewTransformRef.current;
      dragStateRef.current = { startX: event.clientX, startY: event.clientY, ox: t.offsetX, oy: t.offsetY };
    };
    const onPointerMove = (event: PointerEvent) => {
      const rect = canvas.getBoundingClientRect();
      const t = viewTransformRef.current;
      if (dragStateRef.current) {
        const dx = event.clientX - dragStateRef.current.startX;
        const dy = event.clientY - dragStateRef.current.startY;
        const next = { offsetX: dragStateRef.current.ox + dx, offsetY: dragStateRef.current.oy + dy, zoom: t.zoom };
        viewTransformRef.current = next;
        viewport.setViewport({ offsetX: next.offsetX, offsetY: next.offsetY });
        return;
      }
      const worldX = (event.clientX - rect.left - t.offsetX) / t.zoom;
      const worldY = (event.clientY - rect.top - t.offsetY) / t.zoom;
      const idx = viewport.setHover(worldX, worldY);
      canvas.style.cursor = idx >= 0 ? "pointer" : "grab";
    };
    const onPointerUp = (event: PointerEvent) => {
      const drag = dragStateRef.current;
      dragStateRef.current = null;
      if (!drag) return;
      const moved = Math.abs(event.clientX - drag.startX) + Math.abs(event.clientY - drag.startY);
      if (moved < 4) {
        const rect = canvas.getBoundingClientRect();
        const t = viewTransformRef.current;
        const worldX = (event.clientX - rect.left - t.offsetX) / t.zoom;
        const worldY = (event.clientY - rect.top - t.offsetY) / t.zoom;
        const idx = viewport.setHover(worldX, worldY);
        if (idx >= 0) {
          viewport.toggleSelected(idx, event.shiftKey);
          const ids = viewport.selectedIds();
          setSelectedNodeId(ids[ids.length - 1] ?? null);
          const latest = ids[ids.length - 1];
          if (latest) deck.selectOnlyNode(latest);
        } else if (!event.shiftKey) {
          viewport.selectOnly([]);
          setSelectedNodeId(null);
        }
      }
    };
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const t = viewTransformRef.current;
      const mx = event.clientX - rect.left;
      const my = event.clientY - rect.top;
      const factor = event.deltaY < 0 ? 1.15 : 1 / 1.15;
      const nextZoom = Math.max(0.08, Math.min(2.0, t.zoom * factor));
      // Zoom around cursor.
      const wx = (mx - t.offsetX) / t.zoom;
      const wy = (my - t.offsetY) / t.zoom;
      const next = {
        offsetX: mx - wx * nextZoom,
        offsetY: my - wy * nextZoom,
        zoom: nextZoom,
      };
      viewTransformRef.current = next;
      viewport.setViewport(next);
    };
    canvas.addEventListener("pointerdown", onPointerDown);
    canvas.addEventListener("pointermove", onPointerMove);
    canvas.addEventListener("pointerup", onPointerUp);
    canvas.addEventListener("pointercancel", onPointerUp);
    canvas.addEventListener("wheel", onWheel, { passive: false });

    return () => {
      window.removeEventListener("resize", onResize);
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", onPointerUp);
      canvas.removeEventListener("wheel", onWheel);
    };
  }, []);

  // Fetch + parse when workflowId changes. We use the Web Worker path but
  // fall back to main-thread parse for environments without Worker support
  // (the node-driven loader tests).
  useEffect(() => {
    if (!workflowId) {
      setState({ phase: "idle" });
      return;
    }
    let cancelled = false;
    const reqId = ++requestIdRef.current;
    setState({ phase: "loading" });
    const startedAt = performance.now();
    let fetchedBytes = 0;
    const initial: LargeEditorEvidence = {
      workflowId,
      fetchedBytes: 0,
      loadStartedAt: startedAt,
    };
    evidenceRef.current = initial;
    window.__canopyLargeEditorEvidence = initial;

    const applyTopology = (topology: PackedTopology, parseMs?: number) => {
      const evidence = evidenceRef.current;
      if (!evidence || cancelled || reqId !== requestIdRef.current) return;
      if (cancelled || reqId !== requestIdRef.current) return;
      const t0 = performance.now();
      viewportRef.current?.setTopology(topology);
      deckInstRef.current?.setData(topology.sections.groups, topology.sections.nodes);
      if (searchQuery.trim()) {
        const results = searchNodes(topology.sections.nodes, searchQuery);
        resultsRef.current?.setItems(results);
      } else {
        resultsRef.current?.setItems([]);
      }
      viewportRef.current?.setViewport({});
      const readyAt = performance.now();
      evidenceRef.current!.readyAt = readyAt;
      evidenceRef.current!.totalLoadMs = Number((readyAt - startedAt).toFixed(3));
      evidenceRef.current!.parseMs = parseMs;
      evidenceRef.current!.nodeCount = topology.sections.nodes.length;
      evidenceRef.current!.connectionCount = topology.sections.connections.length;
      evidenceRef.current!.groupCount = topology.sections.groups.length;
      setState({ phase: "ready", topology });
    };

    const fallbackParse = (buffer: ArrayBuffer, digestHeader: string | null) => {
      // Synchronous incremental progress shim (same chunking as the worker).
      setState({
        phase: "progress",
        progress: { phase: "verify", bytesLoaded: 0, totalBytes: buffer.byteLength, nodesIndexed: 0, connectionsIndexed: 0 },
      });
      const parseStart = performance.now();
      const parsed = parseTopology(buffer, digestHeader);
      const parseMs = performance.now() - parseStart;
      for (let i = 0; i < parsed.sections.nodes.length; i += WORKER_CHUNK) {
        setState({
          phase: "progress",
          progress: {
            phase: "index",
            bytesLoaded: buffer.byteLength,
            totalBytes: buffer.byteLength,
            nodesIndexed: Math.min(i + WORKER_CHUNK, parsed.sections.nodes.length),
            connectionsIndexed: 0,
          },
        });
      }
      applyTopology(parsed, parseMs);
    };

    const runWithWorker = (buffer: ArrayBuffer, digestHeader: string | null) => {
      // Terminate any prior worker.
      workerRef.current?.terminate();
      const worker = new Worker(new URL("./topology-worker.ts", import.meta.url), { type: "module" });
      workerRef.current = worker;
      worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
        if (cancelled || event.data.id !== `req-${reqId}`) return;
        if (event.data.type === "progress") {
          setState({ phase: "progress", progress: event.data });
        } else if (event.data.type === "complete") {
          applyTopology(event.data.topology, undefined);
          worker.terminate();
          workerRef.current = null;
        } else if (event.data.type === "error") {
          setState({ phase: "error", message: event.data.message });
          worker.terminate();
          workerRef.current = null;
        } else if (event.data.type === "cancelled") {
          worker.terminate();
          workerRef.current = null;
        }
      };
      worker.postMessage(
        { id: `req-${reqId}`, type: "parse", buffer, digest: digestHeader },
        [buffer],
      );
    };

    (async () => {
      try {
        const response = await fetch(`/api/v1/workflows/${encodeURIComponent(workflowId)}/topology`);
        if (!response.ok) {
          const problem = await response.json().catch(() => ({})) as { code?: string };
          throw new Error(problem.code ?? `HTTP ${response.status}`);
        }
        const digestHeader = response.headers.get("x-canopy-topology-digest");
        const buffer = await response.arrayBuffer();
        fetchedBytes = buffer.byteLength;
        if (evidenceRef.current) evidenceRef.current.fetchedBytes = fetchedBytes;
        if (typeof Worker !== "undefined") {
          runWithWorker(buffer, digestHeader);
        } else {
          fallbackParse(buffer, digestHeader);
        }
      } catch (error) {
        if (cancelled) return;
        setState({ phase: "error", message: error instanceof Error ? error.message : String(error) });
      }
    })();

    return () => {
      cancelled = true;
      workerRef.current?.postMessage({ type: "cancel" });
      workerRef.current?.terminate();
      workerRef.current = null;
    };
  }, [workflowId]);

  // Re-run search when the query changes.
  useEffect(() => {
    if (state.phase !== "ready") return;
    const results = searchNodes(state.topology.sections.nodes, searchQuery);
    resultsRef.current?.setItems(results);
  }, [searchQuery, state]);

  // Periodic render stats tick: after each animation-frame draw, sample the
  // counters and refresh the HUD.
  useEffect(() => {
    if (state.phase !== "ready") return;
    if (statsIntervalRef.current) window.clearInterval(statsIntervalRef.current);
    const tick = () => {
      const evidence = evidenceRef.current;
      if (!evidence) return;
      viewportRef.current?.setViewport({});
      const stats = viewportRef.current?.lastStats() ?? { nodesInViewport: 0, connectionsInViewport: 0, drawCalls: 0 };
      setRenderStats(stats);
      const deckRows = deckRef.current?.querySelectorAll("li").length ?? 0;
      const searchRows = searchRef.current?.querySelectorAll("li").length ?? 0;
      const canvases = canvasRef.current ? 1 : 0;
      evidence.lastRender = stats;
      evidence.domCounts = {
        structureDeckRows: deckRows,
        searchResultRows: searchRows,
        canvasElements: canvases,
      };
      window.__canopyLargeEditorEvidence = evidence;
    };
    tick();
    statsIntervalRef.current = window.setInterval(tick, 400);
    return () => {
      if (statsIntervalRef.current) window.clearInterval(statsIntervalRef.current);
      statsIntervalRef.current = null;
    };
  }, [state.phase]);

  const focusNodeById = (nodeId: string) => {
    if (state.phase !== "ready") return;
    const idx = state.topology.nodeIdToIndex.get(nodeId);
    if (idx === undefined) return;
    viewportRef.current?.selectOnly([idx]);
    const node = state.topology.sections.nodes[idx];
    const zoom = Math.max(0.6, viewTransformRef.current.zoom);
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;
    viewTransformRef.current = {
      zoom,
      offsetX: rect.width / 2 - node.x * zoom,
      offsetY: rect.height / 2 - node.y * zoom,
    };
    viewportRef.current?.setViewport(viewTransformRef.current);
    deckInstRef.current?.selectOnlyNode(nodeId);
    setSelectedNodeId(nodeId);
  };

  const onSearchKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") setSearchQuery("");
  };

  const totalNodes = state.phase === "ready" ? state.topology.sections.nodes.length : 0;
  const totalConnections = state.phase === "ready" ? state.topology.sections.connections.length : 0;
  const totalGroups = state.phase === "ready" ? state.topology.sections.groups.length : 0;

  return (
    <section class="large-editor" data-testid="large-editor" aria-labelledby="large-editor-title">
      <div class="large-editor-head">
        <div>
          <p class="eyebrow">Bounded canvas (Ticket 14)</p>
          <h3 id="large-editor-title">Workflow topology</h3>
        </div>
        <div class="viewport-hud" data-testid="viewport-hud" aria-live="polite">
          {state.phase === "progress" && (
            <span>Indexing… {state.progress.nodesIndexed.toLocaleString()} / {totalNodes || "…"} nodes</span>
          )}
          {state.phase === "ready" && (
            <>
              <span>{totalNodes.toLocaleString()} nodes</span>
              <span>{totalConnections.toLocaleString()} edges</span>
              <span>{totalGroups.toLocaleString()} groups</span>
              <span data-testid="viewport-cull">{renderStats.nodesInViewport.toLocaleString()} drawn</span>
              <span data-testid="dom-note">DOM rows ≤ {PAGE_SIZE}</span>
            </>
          )}
          {state.phase === "error" && <span class="error">Topology load failed: {state.message}</span>}
        </div>
      </div>
      <div class="large-editor-grid">
        <aside class="structure-deck-host" data-testid="structure-deck" aria-label="Structure deck">
          <p class="deck-eyebrow">Structure deck</p>
          <div ref={deckRef} class="deck-list" />
          {selectedNodeId && <p class="selected-note">Selected: <code>{selectedNodeId}</code></p>}
        </aside>
        <div class="canvas-host">
          <div class="command-map">
            <label>Command map
              <input
                data-testid="command-map-input"
                type="search"
                placeholder="Jump to node (name or id)…"
                value={searchQuery}
                onInput={(event) => setSearchQuery(event.currentTarget.value)}
                onKeyDown={onSearchKey}
              />
            </label>
            <div ref={searchRef} class="search-results" />
          </div>
          <canvas
            ref={canvasRef}
            class="viewport-canvas"
            data-testid="viewport-canvas"
            tabIndex={0}
            aria-label="Workflow viewport canvas (pan with drag, zoom with wheel)"
          />
          {state.phase === "ready" && (
            <p class="viewport-help">Drag to pan · wheel to zoom · click to select · shift+click to multi-select</p>
          )}
        </div>
      </div>
    </section>
  );
}

type WorkerMessage =
  | { id: string; type: "progress" } & LoadProgress
  | { id: string; type: "complete"; topology: PackedTopology }
  | { id: string; type: "error"; message: string }
  | { id: string; type: "cancelled" };

declare global {
  interface Window {
    __canopyLargeEditorEvidence?: LargeEditorEvidence;
  }
}

type LargeEditorEvidence = {
  workflowId: string;
  fetchedBytes: number;
  loadStartedAt: number;
  readyAt?: number;
  totalLoadMs?: number;
  parseMs?: number;
  nodeCount?: number;
  connectionCount?: number;
  groupCount?: number;
  domCounts?: {
    structureDeckRows: number;
    searchResultRows: number;
    canvasElements: number;
  };
  lastRender?: RenderStats;
};
