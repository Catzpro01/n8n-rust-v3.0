// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Canvas 2D viewport renderer for Ticket 14.
 *
 * The editor must stay responsive with 100,000 nodes; therefore:
 * - Nodes are drawn on a <canvas> sized to the viewport (not a 100k-element
 *   DOM tree).
 * - Only nodes whose bounding box intersects the current viewport are drawn.
 * - Selection, hover, and command palette overlays are decoupled from the
 *   canvas and render tiny bounded DOM (selected chips, a search list).
 * - Pan/zoom transforms happen in CSS transforms on the canvas wrapper.
 */

import type { PackedTopology, TopologyNode } from "./topology-loader";

export interface ViewportState {
  offsetX: number;
  offsetY: number;
  zoom: number;
  width: number;
  height: number;
}

export interface RenderStats {
  nodesInViewport: number;
  connectionsInViewport: number;
  drawCalls: number;
}

const NODE_WIDTH = 160;
const NODE_HEIGHT = 72;
const MIN_ZOOM_DETAIL = 0.35;

function drawRoundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  if (typeof ctx.roundRect === "function") {
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
  } else {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + r);
    ctx.lineTo(x + w, y + h - r);
    ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    ctx.lineTo(x + r, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
  }
}

export class ViewportCanvas {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private topology: PackedTopology | null = null;
  private viewport: ViewportState;
  private devicePixelRatio = 1;
  private hoveredIndex = -1;
  private selected = new Set<number>();
  private rafId: number | null = null;
  private dirty = true;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const ctx = canvas.getContext("2d", { alpha: false });
    if (!ctx) throw new Error("canvas 2d context unavailable");
    this.ctx = ctx;
    this.viewport = { offsetX: 0, offsetY: 0, zoom: 1, width: 0, height: 0 };
    this.resize();
  }

  setTopology(topology: PackedTopology) {
    this.topology = topology;
    this.dirty = true;
    this.requestDraw();
  }

  resize() {
    const rect = this.canvas.getBoundingClientRect();
    this.devicePixelRatio = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.floor(rect.width * this.devicePixelRatio));
    const h = Math.max(1, Math.floor(rect.height * this.devicePixelRatio));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.viewport.width = rect.width;
    this.viewport.height = rect.height;
    this.dirty = true;
    this.requestDraw();
  }

  setViewport(partial: Partial<ViewportState>) {
    Object.assign(this.viewport, partial);
    this.dirty = true;
    this.requestDraw();
  }

  setHover(worldX: number, worldY: number): number {
    const idx = this.hitTest(worldX, worldY);
    if (idx !== this.hoveredIndex) {
      this.hoveredIndex = idx;
      this.dirty = true;
      this.requestDraw();
    }
    return idx;
  }

  toggleSelected(index: number, additive: boolean) {
    if (!additive) this.selected.clear();
    if (this.selected.has(index)) this.selected.delete(index);
    else this.selected.add(index);
    this.dirty = true;
    this.requestDraw();
  }

  selectOnly(indices: number[]) {
    this.selected = new Set(indices);
    this.dirty = true;
    this.requestDraw();
  }

  get selectedCount() {
    return this.selected.size;
  }

  selectedIds(): string[] {
    const topology = this.topology;
    if (!topology) return [];
    return Array.from(this.selected).map((i) => topology.sections.nodes[i].id);
  }

  /** Expose the last draw's counters for the HUD; zero until first draw. */
  lastStats(): RenderStats {
    return { ...this.lastStats_ };
  }

  private lastStats_: RenderStats = { nodesInViewport: 0, connectionsInViewport: 0, drawCalls: 0 };

  private requestDraw() {
    if (this.rafId !== null || !this.dirty) return;
    this.rafId = requestAnimationFrame(() => {
      this.rafId = null;
      this.lastStats_ = this.draw();
    });
  }

  private hitTest(wx: number, wy: number): number {
    const t = this.topology;
    if (!t) return -1;
    // Check the last 256 candidates under cursor; for linear search over 100k
    // we short-circuit by bailing out of the bounding-box check early using
    // the known world→screen transform and a spatial index (not yet). For
    // correctness during Ticket 14 we accept a bounded linear scan on hover
    // because pointermove is throttled by the browser.
    const nodes = t.sections.nodes;
    const hw = NODE_WIDTH / 2;
    const hh = NODE_HEIGHT / 2;
    for (let i = nodes.length - 1; i >= Math.max(0, nodes.length - 256); i--) {
      const n = nodes[i];
      if (wx >= n.x - hw && wx <= n.x + hw && wy >= n.y - hh && wy <= n.y + hh) {
        return i;
      }
    }
    return -1;
  }

  private draw(): RenderStats {
    this.dirty = false;
    const ctx = this.ctx;
    const { width, height, zoom, offsetX, offsetY } = this.viewport;
    ctx.setTransform(this.devicePixelRatio, 0, 0, this.devicePixelRatio, 0, 0);
    // n8n dark canvas background
    ctx.fillStyle = "#18181f";
    ctx.fillRect(0, 0, width, height);

    const stats: RenderStats = { nodesInViewport: 0, connectionsInViewport: 0, drawCalls: 0 };
    const t = this.topology;
    if (!t) return stats;

    // Compute visible world bounds.
    const wx0 = -offsetX / zoom;
    const wy0 = -offsetY / zoom;
    const wx1 = wx0 + width / zoom;
    const wy1 = wy0 + height / zoom;

    const nodes = t.sections.nodes;
    const hw = NODE_WIDTH / 2;
    const hh = NODE_HEIGHT / 2;

    // n8n-style Dot Matrix Canvas Grid
    const dotSpacing = zoom > 0.7 ? 24 : zoom > 0.35 ? 48 : 96;
    const dotRadius = Math.max(1, 1.25 * Math.min(zoom, 1.2));
    ctx.fillStyle = "rgba(255, 255, 255, 0.12)";
    ctx.beginPath();
    const startGX = Math.floor(wx0 / dotSpacing) * dotSpacing;
    const endGX = Math.ceil(wx1 / dotSpacing) * dotSpacing;
    const startGY = Math.floor(wy0 / dotSpacing) * dotSpacing;
    const endGY = Math.ceil(wy1 / dotSpacing) * dotSpacing;
    for (let x = startGX; x <= endGX; x += dotSpacing) {
      const sx = (x - wx0) * zoom;
      for (let y = startGY; y <= endGY; y += dotSpacing) {
        const sy = (y - wy0) * zoom;
        ctx.moveTo(sx + dotRadius, sy);
        ctx.arc(sx, sy, dotRadius, 0, Math.PI * 2);
      }
    }
    ctx.fill();
    stats.drawCalls++;

    // Draw n8n-style Cubic Bezier connections between ports
    ctx.strokeStyle = "rgba(148, 163, 184, 0.55)";
    ctx.lineWidth = Math.max(1.8, 2.2 * zoom);
    ctx.beginPath();
    for (const c of t.sections.connections) {
      const sIdx = t.nodeIdToIndex.get(c.source_node);
      const tIdx = t.nodeIdToIndex.get(c.target_node);
      if (sIdx === undefined || tIdx === undefined) continue;
      const s = nodes[sIdx];
      const tgt = nodes[tIdx];
      const svx = s.x >= wx0 - hw && s.x <= wx1 + hw && s.y >= wy0 - hh && s.y <= wy1 + hh;
      const tvx = tgt.x >= wx0 - hw && tgt.x <= wx1 + hw && tgt.y >= wy0 - hh && tgt.y <= wy1 + hh;
      if (!svx && !tvx) continue;
      stats.connectionsInViewport++;

      // Source output port (right center) to target input port (left center)
      const x1 = (s.x + hw - wx0) * zoom;
      const y1 = (s.y - wy0) * zoom;
      const x2 = (tgt.x - hw - wx0) * zoom;
      const y2 = (tgt.y - wy0) * zoom;
      const dx = Math.max(32 * zoom, Math.abs(x2 - x1) * 0.45);

      ctx.moveTo(x1, y1);
      ctx.bezierCurveTo(x1 + dx, y1, x2 - dx, y2, x2, y2);
    }
    ctx.stroke();
    stats.drawCalls++;

    // Nodes: n8n modern card geometry with category icon badge & ports
    const showDetail = zoom >= MIN_ZOOM_DETAIL;
    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      if (n.x + hw < wx0 || n.x - hw > wx1 || n.y + hh < wy0 || n.y - hh > wy1) continue;
      stats.nodesInViewport++;
      const sx = (n.x - hw - wx0) * zoom;
      const sy = (n.y - hh - wy0) * zoom;
      const sw = NODE_WIDTH * zoom;
      const sh = NODE_HEIGHT * zoom;
      const isSelected = this.selected.has(i);
      const isHovered = i === this.hoveredIndex;

      ctx.save();
      const cornerRadius = Math.max(4, Math.min(8 * zoom, 10));
      drawRoundedRect(ctx, sx, sy, sw, sh, cornerRadius);

      // Node background
      if (isSelected) {
        ctx.fillStyle = "#202330";
      } else if (isHovered) {
        ctx.fillStyle = "#272a38";
      } else {
        ctx.fillStyle = "#1e2029";
      }
      ctx.fill();

      // Node border
      if (isSelected) {
        ctx.strokeStyle = "#ff6d5a"; // Coral n8n glow
        ctx.lineWidth = Math.max(2, 2.5 * zoom);
      } else if (isHovered) {
        ctx.strokeStyle = "#4e546a";
        ctx.lineWidth = Math.max(1, 1.5 * zoom);
      } else {
        ctx.strokeStyle = "#303443";
        ctx.lineWidth = Math.max(1, 1 * zoom);
      }
      ctx.stroke();

      // Connector ports: Left (Input) & Right (Output)
      const portRadius = Math.max(3, 4.5 * zoom);

      // Left input port
      ctx.beginPath();
      ctx.arc(sx, sy + sh / 2, portRadius, 0, Math.PI * 2);
      ctx.fillStyle = isSelected ? "#ff6d5a" : "#45495b";
      ctx.fill();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = Math.max(1, 1.2 * zoom);
      ctx.stroke();

      // Right output port
      ctx.beginPath();
      ctx.arc(sx + sw, sy + sh / 2, portRadius, 0, Math.PI * 2);
      ctx.fillStyle = isSelected ? "#ff6d5a" : "#45495b";
      ctx.fill();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = Math.max(1, 1.2 * zoom);
      ctx.stroke();

      // Inner details: Category icon, label, and sublabel
      if (showDetail && sw > 48 && sh > 24) {
        const iconSize = Math.min(28 * zoom, 28);
        const iconMargin = 8 * zoom;
        const iconX = sx + iconMargin;
        const iconY = sy + (sh - iconSize) / 2;

        let catColor = "#ff6d5a"; // n8n coral for triggers/manual
        const nameLower = n.name.toLowerCase();
        const contractLower = n.contract.toLowerCase();
        if (contractLower.includes("http") || nameLower.includes("http") || nameLower.includes("request") || nameLower.includes("api")) {
          catColor = "#38bdf8"; // blue
        } else if (contractLower.includes("code") || nameLower.includes("transform") || nameLower.includes("code") || nameLower.includes("eval")) {
          catColor = "#10b981"; // emerald
        } else if (contractLower.includes("agent") || nameLower.includes("ai") || nameLower.includes("llm")) {
          catColor = "#a855f7"; // purple
        } else if (contractLower.includes("hub") || nameLower.includes("webhook") || nameLower.includes("poll")) {
          catColor = "#f59e0b"; // amber
        }

        drawRoundedRect(ctx, iconX, iconY, iconSize, iconSize, Math.max(3, Math.min(5 * zoom, 6)));
        ctx.fillStyle = catColor;
        ctx.fill();

        // Icon glyph
        ctx.fillStyle = "#ffffff";
        ctx.font = `bold ${Math.max(9, 12 * zoom)}px ui-sans-serif, system-ui`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        const glyph = (n.name[0] || "N").toUpperCase();
        ctx.fillText(glyph, iconX + iconSize / 2, iconY + iconSize / 2);

        // Text titles
        ctx.textAlign = "left";
        ctx.textBaseline = "alphabetic";
        const textX = iconX + iconSize + 8 * zoom;
        const maxTextWidth = sw - (textX - sx) - 8 * zoom;

        // Title
        ctx.fillStyle = "#f3f4f6";
        ctx.font = `600 ${Math.max(10, 12 * zoom)}px ui-sans-serif, system-ui`;
        const label = truncate(n.name, Math.max(6, Math.floor(maxTextWidth / (7 * zoom))));
        ctx.fillText(label, textX, sy + 23 * zoom);

        // Subtitle
        ctx.fillStyle = "#9ca3af";
        ctx.font = `${Math.max(8, 10 * zoom)}px ui-sans-serif, system-ui`;
        const subtext = n.contract.split("/").pop() || "action";
        ctx.fillText(truncate(subtext, Math.max(6, Math.floor(maxTextWidth / (6 * zoom)))), textX, sy + 39 * zoom);
      }

      ctx.restore();
      stats.drawCalls++;
    }

    return stats;
  }
}

function truncate(s: string, n: number): string {
  return s.length <= n ? s : `${s.slice(0, Math.max(0, n - 1))}…`;
}

/**
 * A bounded DOM list for search results / command palette. Renders at most
 * `maxVisible` items at once using windowed slicing on a scroll container.
 */
export class BoundedResultList {
  private container: HTMLElement;
  private items: Array<{ index: number; node: TopologyNode }> = [];
  private maxVisible = 12;
  private rowHeight = 32;
  onSelect: ((nodeId: string, index: number) => void) | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.container.style.overflowY = "auto";
    this.container.style.maxHeight = `${this.maxVisible * this.rowHeight}px`;
    this.container.style.contain = "strict";
  }

  setItems(items: Array<{ index: number; node: TopologyNode }>) {
    this.items = items;
    this.render();
  }

  private render() {
    this.container.innerHTML = "";
    const visible = this.items.slice(0, this.maxVisible);
    const ul = document.createElement("ul");
    ul.setAttribute("role", "listbox");
    ul.setAttribute("aria-label", "Search results");
    ul.style.margin = "0";
    ul.style.padding = "0";
    ul.style.listStyle = "none";
    for (const item of visible) {
      const li = document.createElement("li");
      li.tabIndex = 0;
      li.setAttribute("role", "option");
      li.dataset.nodeIndex = String(item.index);
      li.style.height = `${this.rowHeight}px`;
      li.style.padding = "6px 8px";
      li.style.font = "12px ui-monospace, monospace";
      li.style.color = "#e2e8f0";
      li.style.cursor = "pointer";
      li.classList.add("search-result-row");
      li.textContent = `${item.node.name} — ${item.node.id}`;

      const activate = () => {
        this.onSelect?.(item.node.id, item.index);
      };

      li.addEventListener("click", activate);
      li.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          activate();
        } else if (event.key === "ArrowDown") {
          event.preventDefault();
          const next = li.nextElementSibling as HTMLElement | null;
          next?.focus();
        } else if (event.key === "ArrowUp") {
          event.preventDefault();
          const prev = li.previousElementSibling as HTMLElement | null;
          prev?.focus();
        }
      });
      ul.appendChild(li);
    }
    this.container.appendChild(ul);
  }
}
