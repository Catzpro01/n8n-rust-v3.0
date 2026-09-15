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
//
// Byte equality is not the gate, and deliberately so. Chromium's subpixel font
// rasterization is not reproducible: run 34917759414 and run 34922145595 both
// executed the `validate` job on vps-fern-worker-3 against the same committed
// baselines, and only one of them matched. Antialiasing shifts a channel by one
// and the sha256 changes. A byte-exact gate on that is a lottery, so the hard
// gate is the rendered geometry - width and height from IHDR - which is what a
// layout regression actually changes. Byte drift is reported as an annotated
// warning so it stays visible without deciding the pipeline.

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/**
 * Read width and height out of a PNG IHDR chunk. No decoder dependency: the
 * geometry sits at a fixed offset, and geometry is all the gate needs.
 *
 * @param {Buffer} buffer
 * @returns {{ width: number, height: number }}
 */
export function pngSize(buffer) {
  assert.ok(Buffer.isBuffer(buffer), "pngSize needs a Buffer");
  assert.ok(buffer.length >= 24, "buffer is too short to be a PNG");
  assert.ok(
    buffer.subarray(0, 8).equals(PNG_SIGNATURE),
    "buffer does not start with the PNG signature",
  );
  assert.equal(buffer.toString("latin1", 12, 16), "IHDR", "first chunk is not IHDR");
  const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
  return { width: view.getUint32(16), height: view.getUint32(20) };
}

/**
 * Compare `actual` against the committed baseline `name` inside `baselineDir`.
 *
 * With `update` the baseline is rewritten instead of compared, which is the
 * only supported way to approve an intentional visual change.
 *
 * A geometry difference fails hard. Equal geometry with different bytes is
 * rendering drift: the rendered image is still written next to the baseline as
 * `<name>.actual.<ext>` for review and the detail is emitted as a GitHub
 * annotation, but the comparison passes.
 *
 * @param {string} name baseline file name, e.g. "generate-progress.desktop.png"
 * @param {Buffer} actual rendered bytes
 * @param {{ baselineDir: string, update?: boolean }} options
 * @returns {Promise<{ status: "matched" | "updated" | "drift", baselinePath: string }>}
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

  // Decode before deciding. A baseline or render that is not a readable PNG
  // cannot have its geometry checked, and that is a hard failure - silently
  // passing an undecodable image would hide exactly the regression we guard.
  const expectedSize = pngSize(expected);
  const actualSize = pngSize(actual);
  if (expectedSize.width !== actualSize.width || expectedSize.height !== actualSize.height) {
    assert.fail(layoutMessage(name, expectedSize, actualSize, expected, actual, actualPath));
  }
  warn(name, expected, actual, actualPath);
  return { status: "drift", baselinePath };
}

/**
 * Compare several baselines and report every layout regression in one failure.
 *
 * Comparing one at a time makes CI discover mismatches serially: the run dies
 * on the first one, the next run dies on the second. Rendering a page once and
 * reporting all differences together costs nothing and saves a whole pipeline
 * round trip per additional baseline.
 *
 * @param {Array<[string, Buffer]>} entries baseline name and rendered bytes
 * @param {{ baselineDir: string, update?: boolean }} options
 * @returns {Promise<Array<{ status: string, baselinePath: string }>>}
 */
export async function compareAll(entries, { baselineDir, update = false }) {
  const results = [];
  const regressions = [];
  for (const [name, actual] of entries) {
    try {
      results.push(await compareOrWrite(name, actual, { baselineDir, update }));
    } catch (error) {
      // compareOrWrite already wrote <name>.actual.png before throwing.
      regressions.push(error.message);
    }
  }
  if (regressions.length > 0) {
    assert.fail(
      `${regressions.length} visual baseline(s) changed geometry.\n\n` +
      regressions.join("\n\n"),
    );
  }
  return results;
}

/** `generate-progress.desktop.png` -> `generate-progress.desktop.actual.png` */
export function actualName(name) {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? `${name.slice(0, dot)}.actual${name.slice(dot)}` : `${name}.actual`;
}

function layoutMessage(name, expectedSize, actualSize, expected, actual, actualPath) {
  return [
    `${name} changed geometry - this is a layout regression, not font rendering.`,
    `baseline: ${expectedSize.width}x${expectedSize.height} ${expected.length} bytes sha256:${sha256(expected)}`,
    `actual:   ${actualSize.width}x${actualSize.height} ${actual.length} bytes sha256:${sha256(actual)}`,
    `rendered image written to ${actualPath}`,
    "Review it, then approve the change deliberately with UPDATE_VISUAL_BASELINE=1.",
  ].join("\n");
}

/**
 * Rendering drift at identical geometry. Emitted as a GitHub check-run
 * annotation so it stays visible in the UI without failing the job; `%` and
 * `\r` are escaped because that is the annotation wire format.
 */
function warn(name, expected, actual, actualPath) {
  const { firstDifference, differingBytes } = compareBytes(expected, actual);
  const shared = Math.max(expected.length, actual.length);
  const percent = ((differingBytes / shared) * 100).toFixed(1);
  const lines = [
    `${name}: same geometry, different pixels - font antialiasing drift, not a layout change.`,
    `baseline: ${expected.length} bytes sha256:${sha256(expected)}`,
    `actual:   ${actual.length} bytes sha256:${sha256(actual)}`,
    `first differing byte at offset ${firstDifference}; ${differingBytes} differing byte(s), ${percent}% of the image`,
    `rendered image written to ${actualPath}`,
  ];
  for (const line of lines) {
    process.stdout.write(`::warning::${escapeAnnotation(line)}\n`);
  }
}

function escapeAnnotation(line) {
  return line.replace(/%/g, "%25").replace(/\r/g, "%0D").replace(/\n/g, "%0A");
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
