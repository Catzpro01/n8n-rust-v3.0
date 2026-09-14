// SPDX-License-Identifier: AGPL-3.0-or-later
// Bounded visual-baseline comparison for the browser acceptance tests.
//
// The buffers handed in here are rendered PNGs of roughly 70-110 KiB. They must
// never reach `assert.deepEqual`: Node renders a byte-by-byte diff for unequal
// Buffers, and that diff is built synchronously on the main thread. Measured on
// Node 22 the message grows to ~57 KiB and ~570 MB RSS at 4 KiB of input, and
// the process is OOM-killed (exit 137, event loop frozen, so heartbeats stop)
// from ~16 KiB of input upwards. A visual regression therefore has to be
// reported as a bounded, actionable message instead.

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

/**
 * Compare `actual` against the committed baseline `name` inside `baselineDir`.
 *
 * With `update` the baseline is rewritten instead of compared, which is the
 * only supported way to approve an intentional visual change. On mismatch the
 * rendered image is written next to the baseline as `<name>.actual.<ext>` so it
 * can be reviewed (and uploaded by CI) without re-running the browser.
 *
 * @param {string} name baseline file name, e.g. "generate-progress.desktop.png"
 * @param {Buffer} actual rendered bytes
 * @param {{ baselineDir: string, update?: boolean }} options
 * @returns {Promise<{ status: "matched" | "updated", baselinePath: string }>}
 */
export async function compareOrWrite(name, actual, { baselineDir, update = false }) {
  assert.equal(typeof name, "string", "baseline name is required");
  assert.ok(Buffer.isBuffer(actual), "rendered baseline bytes are required");
  await mkdir(baselineDir, { recursive: true });
  const baselinePath = join(baselineDir, name);
  if (update) {
    await writeFile(baselinePath, actual);
    return { status: "updated", baselinePath };
  }
  const expected = await readFile(baselinePath);
  if (actual.equals(expected)) return { status: "matched", baselinePath };
  const actualPath = join(baselineDir, actualName(name));
  await writeFile(actualPath, actual);
  assert.fail(mismatchMessage(name, expected, actual, actualPath));
}

/** `generate-progress.desktop.png` -> `generate-progress.desktop.actual.png` */
export function actualName(name) {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? `${name.slice(0, dot)}.actual${name.slice(dot)}` : `${name}.actual`;
}

function mismatchMessage(name, expected, actual, actualPath) {
  const { firstDifference, differingBytes } = compareBytes(expected, actual);
  return [
    `${name} differs from its committed visual baseline.`,
    `baseline: ${expected.length} bytes sha256:${sha256(expected)}`,
    `actual:   ${actual.length} bytes sha256:${sha256(actual)}`,
    `first differing byte at offset ${firstDifference}; ${differingBytes} differing byte(s) in total`,
    `rendered image written to ${actualPath}`,
    "Review it, then approve the change deliberately with UPDATE_VISUAL_BASELINE=1.",
  ].join("\n");
}

function compareBytes(expected, actual) {
  const shared = Math.min(expected.length, actual.length);
  let firstDifference = -1;
  let differingBytes = 0;
  for (let index = 0; index < shared; index += 1) {
    if (expected[index] === actual[index]) continue;
    if (firstDifference === -1) firstDifference = index;
    differingBytes += 1;
  }
  if (firstDifference === -1 && expected.length !== actual.length) firstDifference = shared;
  differingBytes += Math.abs(expected.length - actual.length);
  return { firstDifference, differingBytes };
}

function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex").slice(0, 16);
}
