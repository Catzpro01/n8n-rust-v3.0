// SPDX-License-Identifier: AGPL-3.0-or-later
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import { join } from "node:path";
import test from "node:test";

import { actualName, compareAll, compareOrWrite } from "./visual-baseline.mjs";

// The real desktop baseline committed under editor/tests/baselines is 107,596
// bytes. That exact size is what used to be handed to assert.deepEqual, so the
// regression test keeps it.
const BASELINE_BYTES = 107_596;

async function withBaselineDirectory(run) {
  const directory = await mkdtemp(join(os.tmpdir(), "canopy-visual-baseline-"));
  try {
    return await run(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test("a matching baseline passes without building a diff", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    const rendered = Buffer.alloc(BASELINE_BYTES, 0x5a);
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), rendered);
    const result = await compareOrWrite("generate-progress.desktop.png", Buffer.from(rendered), { baselineDir });
    assert.equal(result.status, "matched");
  });
});

test("a differing full-size baseline fails fast with a bounded message", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), Buffer.alloc(BASELINE_BYTES, 0x5a));
    const rendered = Buffer.alloc(BASELINE_BYTES, 0xa5);
    const started = process.hrtime.bigint();
    const error = await compareOrWrite("generate-progress.desktop.png", rendered, { baselineDir }).then(
      () => null,
      (thrown) => thrown,
    );
    const elapsedMilliseconds = Number(process.hrtime.bigint() - started) / 1e6;

    assert.ok(error, "a visual mismatch must fail the acceptance test");
    assert.equal(error.code, "ERR_ASSERTION");
    // assert.deepEqual on these buffers needs minutes and is OOM-killed; the
    // bounded comparison must stay inside one event-loop turn.
    assert.ok(elapsedMilliseconds < 5_000, `comparison took ${elapsedMilliseconds}ms`);
    assert.ok(error.message.length < 4_096, `message was ${error.message.length} characters`);
    assert.match(error.message, /first differing byte at offset 0/);
    assert.match(error.message, new RegExp(`${BASELINE_BYTES} differing byte\\(s\\) in total`));
    assert.match(error.message, /UPDATE_VISUAL_BASELINE=1/);

    const written = await readFile(join(baselineDir, "generate-progress.desktop.actual.png"));
    assert.ok(written.equals(rendered), "the rendered image must be retained for review");
  });
});

test("a length-only difference is reported at the truncation offset", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), Buffer.alloc(1_024, 0x11));
    const error = await compareOrWrite("generate-progress.mobile.png", Buffer.alloc(1_020, 0x11), { baselineDir }).then(
      () => null,
      (thrown) => thrown,
    );
    assert.match(error.message, /first differing byte at offset 1020/);
    assert.match(error.message, /4 differing byte\(s\) in total/);
  });
});

test("update mode rewrites the committed baseline", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    const rendered = Buffer.alloc(64, 0x7f);
    const result = await compareOrWrite("generate-progress.desktop.png", rendered, { baselineDir, update: true });
    assert.equal(result.status, "updated");
    const stored = await readFile(join(baselineDir, "generate-progress.desktop.png"));
    assert.ok(stored.equals(rendered));
  });
});

test("actual file names keep the baseline extension", () => {
  assert.equal(actualName("generate-progress.desktop.png"), "generate-progress.desktop.actual.png");
  assert.equal(actualName("baseline"), "baseline.actual");
});

test("compareAll reports every drifted baseline in a single failure", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), Buffer.alloc(BASELINE_BYTES, 0x5a));
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), Buffer.alloc(74_089, 0x33));

    const error = await compareAll(
      [
        ["generate-progress.desktop.png", Buffer.alloc(BASELINE_BYTES, 0xa5)],
        ["generate-progress.mobile.png", Buffer.alloc(74_089, 0xc3)],
      ],
      { baselineDir },
    ).then(() => null, (thrown) => thrown);

    assert.ok(error, "two drifted baselines must fail the acceptance test");
    assert.match(error.message, /2 visual baseline\(s\) differ/);
    assert.match(error.message, /generate-progress\.desktop\.png differs/);
    assert.match(error.message, /generate-progress\.mobile\.png differs/);
    // Both rendered images must survive, so one run is enough to review both.
    for (const name of ["generate-progress.desktop.actual.png", "generate-progress.mobile.actual.png"]) {
      const written = await readFile(join(baselineDir, name));
      assert.ok(written.length > 0, `${name} must be retained`);
    }
  });
});

test("compareAll passes when every baseline matches and stops at nothing", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), Buffer.alloc(64, 0x11));
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), Buffer.alloc(32, 0x22));
    const results = await compareAll(
      [
        ["generate-progress.desktop.png", Buffer.alloc(64, 0x11)],
        ["generate-progress.mobile.png", Buffer.alloc(32, 0x22)],
      ],
      { baselineDir },
    );
    assert.deepEqual(results.map((item) => item.status), ["matched", "matched"]);
  });
});

test("compareAll names only the baseline that actually drifted", async () => {
  await withBaselineDirectory(async (baselineDir) => {
    await writeFile(join(baselineDir, "generate-progress.desktop.png"), Buffer.alloc(64, 0x11));
    await writeFile(join(baselineDir, "generate-progress.mobile.png"), Buffer.alloc(32, 0x22));
    const error = await compareAll(
      [
        ["generate-progress.desktop.png", Buffer.alloc(64, 0x11)],
        ["generate-progress.mobile.png", Buffer.alloc(32, 0x99)],
      ],
      { baselineDir },
    ).then(() => null, (thrown) => thrown);
    assert.match(error.message, /1 visual baseline\(s\) differ/);
    assert.match(error.message, /generate-progress\.mobile\.png differs/);
    assert.doesNotMatch(error.message, /generate-progress\.desktop\.png differs/);
  });
});
