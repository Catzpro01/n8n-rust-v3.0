// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Structure Deck (Ticket 14).
 *
 * A DOM-bounded hierarchical panel that shows groups first, then nodes inside
 * the selected group. Even for 100,000 nodes, the panel renders at most
 * `PAGE_SIZE` (512) rows at a time with next/previous paging, so the DOM
 * never has a 100k-element tree.
 */

import type { TopologyGroup, TopologyNode } from "./topology-loader";

export const PAGE_SIZE = 512;

export interface StructureDeckState {
  selectedGroupId: string | null;
  collapsed: Set<string>;
  page: number;
  selectedNodeIds: Set<string>;
}

export interface StructureDeckRow {
  kind: "group" | "node";
  id: string;
  label: string;
  depth: number;
  selected: boolean;
  groupCollapsed?: boolean;
  nodeCount?: number;
}

export class StructureDeck {
  private container: HTMLElement;
  private state: StructureDeckState;
  private groups: TopologyGroup[] = [];
  private nodes: TopologyNode[] = [];
  private groupNodes = new Map<string, TopologyNode[]>();
  onSelectNode: ((id: string) => void) | null = null;
  onToggleGroup: ((id: string) => void) | null = null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.state = {
      selectedGroupId: null,
      collapsed: new Set(),
      page: 0,
      selectedNodeIds: new Set(),
    };
    this.container.style.overflowY = "auto";
    this.container.style.contain = "strict";
  }

  setData(groups: TopologyGroup[], nodes: TopologyNode[]) {
    this.groups = groups.slice().sort((a, b) => a.label.localeCompare(b.label));
    this.nodes = nodes;
    this.groupNodes.clear();
    const ungrouped: TopologyNode[] = [];
    for (const n of nodes) {
      if (n.group_id) {
        let bucket = this.groupNodes.get(n.group_id);
        if (!bucket) {
          bucket = [];
          this.groupNodes.set(n.group_id, bucket);
        }
        bucket.push(n);
      } else {
        ungrouped.push(n);
      }
    }
    if (ungrouped.length) {
      this.groupNodes.set("__ungrouped__", ungrouped);
      this.groups = [
        { id: "__ungrouped__", label: "Ungrouped", node_ids: ungrouped.map((n) => n.id), collapsed: false },
        ...this.groups,
      ];
    }
    this.state.page = 0;
    this.render();
  }

  toggleGroup(groupId: string) {
    if (this.state.collapsed.has(groupId)) this.state.collapsed.delete(groupId);
    else this.state.collapsed.add(groupId);
    if (this.state.selectedGroupId !== groupId) this.state.selectedGroupId = groupId;
    this.render();
  }

  selectOnlyNode(id: string) {
    this.state.selectedNodeIds = new Set([id]);
    this.render();
  }

  moveSelected(dx: number, dy: number) {
    // Bulk move is applied to the model by callers; this method only ensures
    // the deck re-renders after the model updates.
    void dx;
    void dy;
    this.render();
  }

  groupSelection(newGroupId: string) {
    if (this.state.selectedNodeIds.size === 0 || this.state.selectedNodeIds.size > PAGE_SIZE) return;
    // Mutate groups in place: remove selected nodes from previous groups and
    // insert them into the new group. Callers persist via draft commands.
    for (const node of this.nodes) {
      if (this.state.selectedNodeIds.has(node.id)) {
        if (node.group_id) {
          this.groupNodes
            .get(node.group_id)
            ?.splice(this.groupNodes.get(node.group_id)!.findIndex((n) => n.id === node.id), 1);
        }
        node.group_id = newGroupId;
      }
    }
    let bucket = this.groupNodes.get(newGroupId);
    if (!bucket) {
      bucket = [];
      this.groupNodes.set(newGroupId, bucket);
      this.groups.push({ id: newGroupId, label: newGroupId, node_ids: [], collapsed: false });
    }
    for (const id of this.state.selectedNodeIds) {
      const node = this.nodes.find((n) => n.id === id);
      if (node && !bucket.find((n) => n.id === id)) bucket.push(node);
    }
    this.render();
  }

  private render() {
    this.container.innerHTML = "";
    const list = document.createElement("ul");
    list.setAttribute("role", "tree");
    list.setAttribute("aria-label", "Workflow node structure");
    list.style.listStyle = "none";
    list.style.margin = "0";
    list.style.padding = "0";
    const rows = this.visibleRows();
    for (const row of rows) {
      const li = document.createElement("li");
      li.tabIndex = 0;
      li.setAttribute("role", "treeitem");
      li.setAttribute("aria-level", String(row.depth + 1));
      li.setAttribute("aria-selected", row.selected ? "true" : "false");
      li.dataset.rowKind = row.kind;
      li.dataset.rowId = row.id;
      li.style.paddingLeft = `${8 + row.depth * 12}px`;
      li.style.height = "28px";
      li.style.display = "flex";
      li.style.alignItems = "center";
      li.style.font = "12px ui-sans-serif, system-ui";
      li.style.color = row.selected ? "#fbbf24" : "#cbd5e1";
      li.style.cursor = "pointer";
      li.classList.add("deck-row");

      const activate = () => {
        if (row.kind === "group") {
          this.toggleGroup(row.id);
          this.onToggleGroup?.(row.id);
        } else {
          this.selectOnlyNode(row.id);
          this.onSelectNode?.(row.id);
        }
      };

      li.addEventListener("click", activate);
      li.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          activate();
        } else if (event.key === "ArrowRight" && row.kind === "group" && row.groupCollapsed) {
          event.preventDefault();
          this.toggleGroup(row.id);
          this.onToggleGroup?.(row.id);
        } else if (event.key === "ArrowLeft" && row.kind === "group" && !row.groupCollapsed) {
          event.preventDefault();
          this.toggleGroup(row.id);
          this.onToggleGroup?.(row.id);
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

      if (row.kind === "group") {
        li.setAttribute("aria-expanded", row.groupCollapsed ? "false" : "true");
        const marker = document.createElement("span");
        marker.textContent = row.groupCollapsed ? "▸" : "▾";
        marker.style.width = "16px";
        marker.setAttribute("aria-hidden", "true");
        li.appendChild(marker);
        const label = document.createElement("span");
        label.textContent = `${row.label} (${row.nodeCount ?? 0})`;
        li.appendChild(label);
      } else {
        li.textContent = row.label;
      }
      list.appendChild(li);
    }
    if (this.pageCount() > 1) {
      const nav = document.createElement("div");
      const prev = document.createElement("button");
      prev.textContent = "◀";
      prev.disabled = this.state.page === 0;
      prev.addEventListener("click", () => {
        if (this.state.page > 0) {
          this.state.page--;
          this.render();
        }
      });
      const next = document.createElement("button");
      next.textContent = "▶";
      next.disabled = this.state.page >= this.pageCount() - 1;
      next.addEventListener("click", () => {
        if (this.state.page < this.pageCount() - 1) {
          this.state.page++;
          this.render();
        }
      });
      nav.appendChild(prev);
      nav.appendChild(document.createTextNode(` page ${this.state.page + 1}/${this.pageCount()} `));
      nav.appendChild(next);
      this.container.appendChild(nav);
    }
    this.container.appendChild(list);
  }

  private visibleRows(): StructureDeckRow[] {
    const rows: StructureDeckRow[] = [];
    const selectedGroup =
      this.state.selectedGroupId ??
      (this.groups.length === 1 ? this.groups[0].id : null);
    if (!selectedGroup) {
      for (const g of this.groups.slice(0, PAGE_SIZE)) {
        rows.push({
          kind: "group",
          id: g.id,
          label: g.label,
          depth: 0,
          selected: false,
          groupCollapsed: this.state.collapsed.has(g.id),
          nodeCount: g.node_ids.length,
        });
      }
      return rows;
    }
    const selectedBucket = this.groupNodes.get(selectedGroup) ?? [];
    const start = this.state.page * PAGE_SIZE;
    const end = Math.min(selectedBucket.length, start + PAGE_SIZE);
    // Always show the active group row.
    const groupEntry = this.groups.find((g) => g.id === selectedGroup);
    if (groupEntry) {
      rows.push({
        kind: "group",
        id: groupEntry.id,
        label: groupEntry.label,
        depth: 0,
        selected: true,
        groupCollapsed: this.state.collapsed.has(groupEntry.id),
        nodeCount: groupEntry.node_ids.length,
      });
      if (!this.state.collapsed.has(groupEntry.id)) {
        for (let i = start; i < end; i++) {
          const n = selectedBucket[i];
          rows.push({
            kind: "node",
            id: n.id,
            label: `${n.name} — ${n.contract.split("/").pop() ?? ""}`,
            depth: 1,
            selected: this.state.selectedNodeIds.has(n.id),
          });
        }
      }
    }
    return rows;
  }

  private pageCount(): number {
    if (!this.state.selectedGroupId) return 1;
    const bucket = this.groupNodes.get(this.state.selectedGroupId);
    if (!bucket) return 1;
    return Math.max(1, Math.ceil(bucket.length / PAGE_SIZE));
  }
}
