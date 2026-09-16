// SPDX-License-Identifier: AGPL-3.0-or-later

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const STYLES_PATH = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "src", "styles.css");
const STRUCTURE_DECK_PATH = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "src", "structure-deck.ts");
const VIEWPORT_CANVAS_PATH = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "src", "viewport-canvas.ts");

test("styles.css enforces WCAG 2.2 AA visible focus indicators", async () => {
  const css = await readFile(STYLES_PATH, "utf-8");
  assert.ok(css.includes(":focus-visible"), "must include :focus-visible rules");
  assert.ok(css.includes(".deck-row:focus-visible"), "must style deck rows on focus");
  assert.ok(css.includes(".search-result-row:focus-visible"), "must style search rows on focus");
  assert.ok(css.includes("prefers-reduced-motion"), "must respect prefers-reduced-motion");
  assert.ok(css.includes(".sr-only"), "must define sr-only utility class");
});

test("structure-deck.ts implements ARIA tree and keyboard navigation", async () => {
  const code = await readFile(STRUCTURE_DECK_PATH, "utf-8");
  assert.ok(code.includes('role", "tree"'), "StructureDeck list must have role=tree");
  assert.ok(code.includes('role", "treeitem"'), "StructureDeck rows must have role=treeitem");
  assert.ok(code.includes("aria-expanded"), "StructureDeck groups must set aria-expanded");
  assert.ok(code.includes("aria-selected"), "StructureDeck items must set aria-selected");
  assert.ok(code.includes("ArrowDown"), "StructureDeck must support ArrowDown navigation");
  assert.ok(code.includes("ArrowUp"), "StructureDeck must support ArrowUp navigation");
  assert.ok(code.includes("Enter"), "StructureDeck must support Enter activation");
});

test("viewport-canvas.ts BoundedResultList implements ARIA listbox and keyboard navigation", async () => {
  const code = await readFile(VIEWPORT_CANVAS_PATH, "utf-8");
  assert.ok(code.includes('role", "listbox"'), "BoundedResultList must have role=listbox");
  assert.ok(code.includes('role", "option"'), "BoundedResultList items must have role=option");
  assert.ok(code.includes("ArrowDown"), "BoundedResultList must support ArrowDown navigation");
  assert.ok(code.includes("ArrowUp"), "BoundedResultList must support ArrowUp navigation");
  assert.ok(code.includes("Enter"), "BoundedResultList must support Enter activation");
});
