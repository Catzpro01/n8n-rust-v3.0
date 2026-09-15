// SPDX-License-Identifier: AGPL-3.0-or-later
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import { join } from "node:path";
import test from "node:test";

import { actualName, compareAll, compareOrWrite, pngSize } from "./visual-baseline.mjs";

const BASELINE_BYTES = 107_596;
const DESKTOP = { width: 1172, height: 264 };
const MOBILE = { width: 340, height: 545 };

/** Build a buffer with a real PNG signature and IHDR, padded to `size`. */
function png({ width, height }, size, fill = 0x5a) {
  const buffer = Buffer.alloc(Math.max(size, 24), fill);
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]).copy(buffer, 0);
  buffer.writeUInt32BE(13, 8);
  buffer.write("IHDR", 12, "latin1");
  buffer.writeUInt32BE(width, 16);
  buffer.writeUInt32BE(height, 20);
  return buffer;
}

async function withBaselineDirectory(run) {
  const baselineDir = await mkdtemp(join(os.tmpdir(), "visual-baseline-"));
  try {
    await run(baselineDir);
  } finally {
    await rm(baselineDir, { recursive: true, force: true });
  }
}

test("a matching baseline passes without building a diff", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    const rendered = png(DESKTOP, BASELINE_BYTES);
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), rendered);
    const result = await compareOrWrite("generate-progress.desktop.png", rendered, { baselineDir });
    assert.equal(result.status, "matched");
  });
});

test("same geometry with different pixels is drift, not a failure", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, BASELINE_BYTES, 0x5a));
    const rendered = png(DESKTOP, BASELINE_BYTES, 0xa5);
    const result = await compareOrWrite("generate-progress.desktop.png", rendered, { baselineDir });
    assert.equal(result.status, "drift");
    // Still written, so the drift is reviewable without re-running the browser.
    const written = await readFile(join(baselineDir, "generate-progress.desktop.actual.png"));
    assert.equal(written.length, BASELINE_BYTES);
  });
});

test("a geometry change fails hard as a layout regression", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, BASELINE_BYTES, 0x5a));
    const taller = png({ width: 1172, height: 265 }, BASELINE_BYTES, 0x5a);
    const error = await compareOrWrite("generate-progress.desktop.png", taller, { baselineDir }).then(
      () => null,
      (thrown) => thrown,
    );
    assert.ok(error, "a one-pixel height change must fail the acceptance test");
    assert.match(error.message, /changed geometry/);
    assert.match(error.message, /1172x264/);
    assert.match(error.message, /1172x265/);
  });
});

test("a differing full-size baseline reports a bounded message", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, BASELINE_BYTES, 0x5a));
    const taller = png({ width: 1200, height: 264 }, BASELINE_BYTES, 0xa5);
    const error = await compareOrWrite("generate-progress.desktop.png", taller, { baselineDir }).then(
      () => null,
      (thrown) => thrown,
    );
    // The point of the bounded message: no byte-by-byte diff of a 100 KiB buffer.
    assert.ok(error.message.length < 2_000, `message must stay bounded, got ${error.message.length}`);
  });
});

test("an undecodable image fails rather than passing silently", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, BASELINE_BYTES, 0x5a));
    const notPng = Buffer.alloc(BASELINE_BYTES, 0x00);
    const error = await compareOrWrite("generate-progress.desktop.png", notPng, { baselineDir }).then(
      () => null,
      (thrown) => thrown,
    );
    assert.ok(error, "an image whose geometry cannot be read must not pass the gate");
    assert.match(error.message, /PNG signature/);
  });
});

test("update mode rewrites the committed baseline", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    const rendered = png(DESKTOP, 64, 0x7f);
    const result = await compareOrWrite("generate-progress.desktop.png", rendered, {
      baselineDir,
      update: true,
    });
    assert.equal(result.status, "updated");
    const written = await readFile(result.baselinePath);
    assert.ok(written.equals(rendered));
  });
});

test("actual file names keep the baseline extension", () => {
  assert.equal(actualName("generate-progress.desktop.png"), "generate-progress.desktop.actual.png");
  assert.equal(actualName("no-extension"), "no-extension.actual");
});

test("pngSize reads the geometry the gate depends on", () => {
  assert.deepEqual(pngSize(png(MOBILE, 64)), MOBILE);
});

test("compareAll reports every geometry change in a single failure", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, BASELINE_BYTES, 0x5a));
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), png(MOBILE, 74_089, 0x33));

    const error = await compareAll(
      [
        ["generate-progress.desktop.png", png({ width: 1172, height: 265 }, BASELINE_BYTES, 0xa5)],
        ["generate-progress.mobile.png", png({ width: 340, height: 546 }, 74_089, 0xc3)],
      ],
      { baselineDir },
    ).then(() => null, (thrown) => thrown);

    assert.ok(error, "two layout regressions must fail the acceptance test");
    assert.match(error.message, /2 visual baseline\(s\) changed geometry/);
    assert.match(error.message, /generate-progress\.desktop\.png/);
    assert.match(error.message, /generate-progress\.mobile\.png/);
    for (const name of ["generate-progress.desktop.actual.png", "generate-progress.mobile.actual.png"]) {
      const written = await readFile(join(baselineDir, name));
      assert.ok(written.length > 0, `${name} must be retained`);
    }
  });
});

test("compareAll passes when geometry matches even if pixels drift", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, 64, 0x11));
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), png(MOBILE, 32, 0x22));
    const results = await compareAll(
      [
        ["generate-progress.desktop.png", png(DESKTOP, 64, 0x99)],
        ["generate-progress.mobile.png", png(MOBILE, 32, 0x22)],
      ],
      { baselineDir },
    );
    assert.deepEqual(results.map((item) => item.status), ["drift", "matched"]);
  });
});

test("compareAll names only the baseline whose geometry changed", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), png(DESKTOP, 64, 0x11));
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), png(MOBILE, 32, 0x22));
    const error = await compareAll(
      [
        ["generate-progress.desktop.png", png(DESKTOP, 64, 0x11)],
        ["generate-progress.mobile.png", png({ width: 341, height: 545 }, 32, 0x22)],
      ],
      { baselineDir },
    ).then(() => null, (thrown) => thrown);
    assert.match(error.message, /1 visual baseline\(s\) changed geometry/);
    assert.match(error.message, /generate-progress\.mobile\.png/);
    assert.doesNotMatch(error.message, /generate-progress\.desktop\.png changed geometry/);
  });
});
