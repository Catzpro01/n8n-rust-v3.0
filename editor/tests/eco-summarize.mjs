// SPDX-License-Identifier: AGPL-3.0-or-later
import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { chromium } from "playwright";

const repo = resolve(import.meta.dirname, "../..");
const binary = process.env.WORKFLOWD_BIN || join(repo, "target/debug/workflowd");
const root = await mkdtemp(join(os.tmpdir(), "canopy-eco-summarize-ui-"));
const state = join(root, "state");
const key = join(root, "master.key");
await writeFile(key, crypto.getRandomValues(new Uint8Array(32)));
const port = await freePort();
const origin = `http://localhost:${port}`;
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
    WORKFLOWD_DRAFT_LEASE_TTL_SECONDS: "30",
  },
  stdio: ["ignore", "pipe", "pipe"],
});
daemon.stdout.resume();
// Keep a bounded tail of the daemon's stderr. Discarding it is what made a
// startup failure report nothing but "daemon did not start" (run 34920215184;
// the same bare message came back in runs 34954379976 and 34955243839).
const daemonStderr = [];
let daemonStderrBytes = 0;
daemon.stderr.on("data", (chunk) => {
  if (daemonStderrBytes >= 65_536) return;
  daemonStderr.push(chunk);
  daemonStderrBytes += chunk.length;
});
let browser;
try {
  await ready(origin, daemon, daemonStderr);
  const setup = await fetch(`${origin}/api/v1/setup`, {
    method: "POST",
    headers: { origin, "content-type": "application/json" },
    body: JSON.stringify({
      email: "owner@example.test",
      password: "correct horse battery staple",
      recovery_passphrase: "separate recovery phrase long",
    }),
  });
  assert.equal(setup.status, 201);

  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 1100 },
    locale: "en-US",
    timezoneId: "UTC",
    reducedMotion: "reduce",
  });
  const page = await context.newPage();
  await page.goto(origin);
  await page.getByTestId("email").fill("owner@example.test");
  await page.getByTestId("password").fill("correct horse battery staple");
  await page.getByTestId("sign-in").click();
  await page.waitForFunction(() => Boolean(localStorage.getItem("canopy-owner-session-v1")));

  const publication = await page.evaluate(async () => {
    const owner = JSON.parse(localStorage.getItem("canopy-owner-session-v1"));
    const editorSessionId = sessionStorage.getItem("canopy-editor-session-v1");
    if (!editorSessionId) throw new Error("editor session was not claimed");
    const csrf = owner.csrf_token;
    const mutate = async (path, body) => {
      const response = await fetch(path, {
        method: "POST",
        headers: { "content-type": "application/json", "x-canopy-csrf": csrf },
        body: JSON.stringify(body),
      });
      const value = await response.json();
      if (!response.ok) throw new Error(`${path}: ${response.status} ${JSON.stringify(value)}`);
      return value;
    };
    const catalog = await (await fetch("/api/v1/catalog")).json();
    const locks = Object.fromEntries(catalog.nodes.map((node) => [node.contract_lock.name, node.contract_lock]));
    const workflowId = "wf-eco-summarize-browser";
    await mutate("/api/v1/workflows", {
      workflow_id: workflowId,
      name: "Eco 100K Summarize Browser Acceptance",
      annotation: "Ticket 11 browser/API acceptance",
      settings: {},
      compatibility_metadata: {},
    });
    const lease = await mutate(`/api/v1/workflows/${workflowId}/editing/open`, {
      editor_session_id: editorSessionId,
      label: "Eco browser tab",
    });
    let version = 0;
    const operations = [
      { kind: "add_node", node_instance: {
        id: "manual-trigger", name: "Manual Trigger", contract_lock: locks["manual-trigger"],
        configuration: { capture_mode: "manual" }, layout: { x: 100, y: 120 }, annotation: "", compatibility_metadata: {},
      } },
      { kind: "add_node", node_instance: {
        id: "generate-items", name: "Generate Items", contract_lock: locks["generate-items"],
        configuration: { count: 49_998, start: 0, step: 1, data: null, storage_mode: "artifact" },
        layout: { x: 360, y: 120 }, annotation: "", compatibility_metadata: {},
      } },
      { kind: "add_node", node_instance: {
        id: "edit-fields", name: "Edit Fields", contract_lock: locks["edit-fields"],
        configuration: { mode: "merge", assignments: [
          { path: ["eco"], kind: "fixed", value: true },
          { path: ["parity"], kind: "expression", source: '$json.value % 2 === 0 ? "even" : "odd"' },
          { path: ["doubled"], kind: "expression", source: "$json.value * 2" },
          { path: ["label"], kind: "expression", source: '"eco-" + $json.index' },
        ] },
        layout: { x: 620, y: 120 }, annotation: "", compatibility_metadata: {},
      } },
      { kind: "add_node", node_instance: {
        id: "if", name: "If", contract_lock: locks["if"],
        configuration: { logic: "all", conditions: [{ expression: '$json.parity === "even"' }] },
        layout: { x: 900, y: 120 }, annotation: "", compatibility_metadata: {},
      } },
      { kind: "add_node", node_instance: {
        id: "merge", name: "Merge", contract_lock: locks.merge,
        configuration: { mode: "true_then_false" }, layout: { x: 1180, y: 120 }, annotation: "", compatibility_metadata: {},
      } },
      { kind: "add_node", node_instance: {
        id: "summarize", name: "Summarize", contract_lock: locks.summarize,
        configuration: { operation: "output_digest" }, layout: { x: 1440, y: 120 }, annotation: "", compatibility_metadata: {},
      } },
      { kind: "connect", connection: { id: "manual-to-generate", source: { node_id: "manual-trigger", port_id: "invocation" }, target: { node_id: "generate-items", port_id: "input" } } },
      { kind: "connect", connection: { id: "generate-to-edit", source: { node_id: "generate-items", port_id: "items" }, target: { node_id: "edit-fields", port_id: "input" } } },
      { kind: "connect", connection: { id: "edit-to-if", source: { node_id: "edit-fields", port_id: "item" }, target: { node_id: "if", port_id: "input" } } },
      { kind: "connect", connection: { id: "if-true-to-merge", source: { node_id: "if", port_id: "true" }, target: { node_id: "merge", port_id: "true" } } },
      { kind: "connect", connection: { id: "if-false-to-merge", source: { node_id: "if", port_id: "false" }, target: { node_id: "merge", port_id: "false" } } },
      { kind: "connect", connection: { id: "merge-to-summarize", source: { node_id: "merge", port_id: "items" }, target: { node_id: "summarize", port_id: "items" } } },
    ];
    for (let index = 0; index < operations.length; index += 1) {
      const accepted = await mutate(`/api/v1/workflows/${workflowId}/draft-commands`, {
        editor_session_id: editorSessionId,
        lease_generation: lease.lease_generation,
        command_id: `eco-browser-${index}`,
        base_draft_version: version,
        operation: operations[index],
      });
      version = accepted.draft_version;
    }
    const previewRequest = {
      editor_session_id: editorSessionId,
      lease_generation: lease.lease_generation,
      draft_version: version,
    };
    const preview = await mutate(`/api/v1/workflows/${workflowId}/compile-preview`, previewRequest);
    if (!preview.can_publish) throw new Error(`Eco compile blocked: ${JSON.stringify(preview)}`);
    const published = await mutate(`/api/v1/workflows/${workflowId}/publish`, {
      publication_id: "publish-eco-browser-one",
      ...previewRequest,
      compile_input_digest: preview.compile_input_digest,
      acknowledged_diagnostics: preview.diagnostics.filter((item) => item.requires_ack).map((item) => item.fingerprint),
    });
    return { workflowId, revisionId: published.revision.revision_id, draftVersion: version };
  });

  await page.goto(`${origin}/?workflow=${publication.workflowId}`);
  await waitText(page.getByTestId("current-publication"), "Revision 1", 600);
  await page.getByTestId("start-run").click();
  await page.getByTestId("generation-progress").waitFor({ timeout: 30_000 });
  await waitAttribute(page.getByTestId("run-durable-state"), "data-state", "succeeded", 6000);
  await waitText(page.getByTestId("generation-progress"), "Generated 49,998 items", 6000);
  await waitText(page.getByTestId("merge-progress"), "Merged 49,998 items", 6000);
  await waitText(page.getByTestId("summarize-progress"), "Summary: 49,998 items", 6000);
  await waitText(page.getByTestId("branch-progress"), "24,999 true", 6000);
  await page.getByTestId("inspect-trace").click();
  await waitText(page.getByTestId("causal-trace"), "Integrity verified", 6000);
  await waitText(page.getByTestId("run-timing"), "Aggregate wall time", 6000);
  await waitText(page.getByTestId("run-timing"), "Aggregate CPU time", 6000);
  await waitCount(page.getByTestId("activation-outcome"), 6, 6000);
  await page.getByTestId("retained-merge-links").waitFor({ timeout: 6000 });
  assert.ok(await page.locator('[data-testid^="retained-merge-segment-"]').count() > 0, "Causal Trace must expose retained Merge segments");
  assert.match(await page.getByTestId("causal-trace").textContent() ?? "", /Output digest/);

  // Browser-authored Draft mutation, second publication, and non-destructive rollback.
  const editor = page.getByTestId("editor");
  const firstDraftVersion = Number(await editor.getAttribute("data-draft-version"));
  await page.getByTestId("annotation").fill("Eco revision two");
  await page.getByTestId("save-annotation").click();
  await waitAttribute(editor, "data-draft-version", String(firstDraftVersion + 1), 600);
  await page.getByTestId("compile-preview").click();
  await page.getByTestId("compile-diagnostics").waitFor({ timeout: 30_000 });
  const acknowledgements = page.locator('input[data-testid^="ack-"]');
  for (const checkbox of await acknowledgements.all()) await checkbox.check();
  await page.getByTestId("publish-revision").click();
  await waitText(page.getByTestId("current-publication"), "Revision 2", 600);
  await page.getByTestId("rollback-revision-1").click();
  await waitText(page.getByTestId("current-publication"), "Revision 1", 600);
  await waitText(page.getByTestId("newer-history"), "1 newer", 600);
  assert.equal(await page.getByTestId("annotation").inputValue(), "Eco revision two");
  await waitText(page.getByTestId("signature-identity"), "Signed rollback", 600);
  await waitText(page.getByTestId("publication-difference"), "Changed", 600);

  const noOverflow = await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth);
  assert.equal(noOverflow, true, "Eco progress must fit the browser viewport");
  console.log("eco-browser-api=passed activations=100000 digest=verified rollback=non-destructive overflow=0");
} finally {
  if (browser) await browser.close();
  daemon.kill("SIGTERM");
  // kill() on an already-exited daemon never emits another "exit", and
  // awaiting one unconditionally turns the failure being reported into an
  // unsettled top-level await (Node exit 13, no message).
  await new Promise((resolve) => {
    if (daemon.exitCode !== null || daemon.signalCode !== null) resolve();
    else daemon.once("exit", resolve);
  });
  await rm(root, { recursive: true, force: true });
}

async function freePort() {
  const server = net.createServer();
  await new Promise((resolvePromise) => server.listen(0, "127.0.0.1", resolvePromise));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 0;
  await new Promise((resolvePromise) => server.close(resolvePromise));
  return port;
}

async function ready(base, daemon, stderrChunks) {
  // 1200 * 50ms = 60s: the same health budget the Python acceptance Daemon
  // allows, sized for a cold debug binary starting behind the other jobs of a
  // shared self-hosted runner.
  for (let attempt = 0; attempt < 1200; attempt += 1) {
    try {
      if ((await fetch(`${base}/health/live`)).ok) return;
    } catch {
      // Startup race.
    }
    // The daemon is already gone: polling out the budget only buries the reason.
    if (daemon.exitCode !== null || daemon.signalCode !== null) break;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  const outcome =
    daemon.exitCode !== null
      ? `exited with code ${daemon.exitCode}`
      : daemon.signalCode !== null
        ? `killed by ${daemon.signalCode}`
        : "still running when the 60s health budget expired";
  const reason = stderrChunks.map((chunk) => chunk.toString()).join("").trim();
  throw new Error(`daemon did not start (${outcome}): ${reason || "no stderr captured"}`);
}

async function waitText(locator, text, attempts) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if ((await locator.textContent())?.includes(text)) return;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 100));
  }
  throw new Error(`expected ${text}: ${await locator.textContent()}`);
}

async function waitAttribute(locator, name, value, attempts) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if ((await locator.getAttribute(name)) === value) return;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 100));
  }
  throw new Error(`expected ${name}=${value}; got ${await locator.getAttribute(name)}`);
}

async function waitCount(locator, expected, attempts) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if (await locator.count() === expected) return;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 100));
  }
  throw new Error(`expected ${expected} matching elements; got ${await locator.count()}`);
}
