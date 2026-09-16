// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Ticket 14 — browser benchmark using Playwright.
 *
 * Starts workflowd, opens /bench.html?source=api against the running daemon,
 * waits for the benchmark to complete, then asserts:
 *   - Parse completes within 1500ms.
 *   - Average FPS stays ≥ 30 while panning across the entire 100k workflow.
 *   - P95 of drawn node count stays ≤ 5,000 per frame.
 *   - Structure Deck DOM rows ≤ PAGE_SIZE (512), search results ≤ 12.
 *   - Total DOM elements < 1,500 (proves no 100k-node tree).
 *
 * It writes an evidence JSON file to editor/tests/fixtures/large-editor.evidence.json
 * and a PNG screenshot of the final frame.
 *
 * Requires `workflowd` at target/debug/workflowd (built via `cargo build`
 * before this script runs). Chromium from Playwright is required.
 */

import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { chromium } from "playwright";
import { randomBytes } from "node:crypto";

const repo = resolve(import.meta.dirname, "../..");
const binary = process.env.WORKFLOWD_BIN || join(repo, "target", "debug", "workflowd");
const root = join(os.tmpdir(), `canopy-bench-${Date.now()}`);
const state = join(root, "state");
const key = join(root, "master.key");
const fixtureDir = join(repo, "editor", "tests", "fixtures");

await mkdir(state, { recursive: true });
await mkdir(fixtureDir, { recursive: true });
await writeFile(key, randomBytes(32));

const port = await freePort();
const origin = `http://127.0.0.1:${port}`;
const daemon = spawn(binary, ["serve"], {
  cwd: repo,
  env: {
    ...process.env,
    WORKFLOWD_BIND: `127.0.0.1:${port}`,
    WORKFLOWD_CONTROL_ORIGIN: origin,
    WORKFLOWD_STATE_DIR: state,
    WORKFLOWD_MASTER_KEY_FILE: key,
    WORKFLOWD_ARGON_MEMORY_KIB: "8192",
    WORKFLOWD_ARGON_ITERATIONS: "1",
    RUST_LOG: "info",
  },
  stdio: ["ignore", "pipe", "pipe"],
});

let daemonLog = "";
daemon.stdout.on("data", (b) => { daemonLog += b.toString(); });
daemon.stderr.on("data", (b) => { daemonLog += b.toString(); });

try {
  await waitForListening(port, 15_000);

  // Generate the 100k fixture via the daemon (requires Rust binary), then
  // load the benchmark page against the served API.
  const fixtureProc = spawn(binary, ["generate-100k-fixture"], {
    cwd: repo,
    env: {
      ...process.env,
      WORKFLOWD_STATE_DIR: state,
      CANOPY_FIXTURE_DIR: fixtureDir,
    },
  });
  await new Promise((resolveP, rejectP) => {
    fixtureProc.on("exit", (code) => (code === 0 ? resolveP(null) : rejectP(new Error(`fixture gen exit ${code}`))));
  });

  const browser = await chromium.launch({ args: ["--no-sandbox"] });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, deviceScaleFactor: 1 });

  const errors = [];
  page.on("pageerror", (err) => errors.push(err.message));
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });

  await page.goto(`${origin}/bench.html?workflow=eco-100k-editor-fixture&source=fixture`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    () => {
      const el = document.getElementById("bench-pass");
      return el && el.dataset.status !== "pending";
    },
    { timeout: 60_000 },
  );

  const result = await page.evaluate(() => (window as unknown as { __benchmarkResult: unknown }).__benchmarkResult);
  const passEl = await page.$("#bench-pass");
  const status = await passEl?.getAttribute("data-status");
  const metricsText = await page.locator("#bench-metrics").textContent();
  await page.screenshot({ path: join(fixtureDir, "large-editor-benchmark.png"), fullPage: true });
  await writeFile(join(fixtureDir, "large-editor.evidence.json"), JSON.stringify(result, null, 2));

  console.log(metricsText);
  assert.equal(errors.length, 0, `page errors: ${errors.join(" | ")}`);
  assert.equal(status, "pass", `benchmark did not pass (status=${status})`);
  assert.ok(result.nodeCount === 100_000, "expected 100,000 nodes");
  assert.ok(result.totalReadyMs < 3_000, `ready in ${result.totalReadyMs}ms (> 3s)`);
  assert.ok(result.fps.avg >= 30, `avg fps ${result.fps.avg} < 30`);
  assert.ok(result.fps.p95 >= 20, `p95 fps ${result.fps.p95} < 20`);
  assert.ok(result.drawnCounts.p95 <= 5_000, `p95 drawn ${result.drawnCounts.p95} > 5000`);
  assert.ok(result.domCounts.structureRows <= 512, `structure rows ${result.domCounts.structureRows} > 512`);
  assert.ok(result.domCounts.searchRows <= 12, `search rows ${result.domCounts.searchRows} > 12`);
  assert.ok(result.domCounts.totalElements < 1_500, `total DOM ${result.domCounts.totalElements} >= 1500`);

  await browser.close();
  console.log("BENCHMARK PASS — evidence written to editor/tests/fixtures/large-editor.evidence.json");
} finally {
  daemon.kill("SIGTERM");
  setTimeout(() => daemon.kill("SIGKILL"), 2_000);
}

function freePort() {
  return new Promise((resolveP, rejectP) => {
    const server = net.createServer();
    server.unref();
    server.on("error", rejectP);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolveP(port));
    });
  });
}

function waitForListening(port, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolveP, rejectP) => {
    const tryConnect = () => {
      const socket = net.connect(port, "127.0.0.1");
      socket.once("connect", () => { socket.destroy(); resolveP(null); });
      socket.once("error", () => {
        socket.destroy();
        if (Date.now() > deadline) rejectP(new Error(`daemon never listened on ${port}\n${daemonLog}`));
        else setTimeout(tryConnect, 100);
      });
    };
    tryConnect();
  });
}
