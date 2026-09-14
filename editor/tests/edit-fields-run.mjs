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
const root = await mkdtemp(join(os.tmpdir(), "canopy-edit-fields-ui-"));
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
daemon.stderr.resume();
let browser;
try {
  await ready(origin);
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
  const context = await browser.newContext({ viewport: { width: 1280, height: 1100 }, bypassCSP: true });
  const page = await context.newPage();
  await page.goto(origin);
  await page.getByTestId("email").fill("owner@example.test");
  await page.getByTestId("password").fill("correct horse battery staple");
  await page.getByTestId("sign-in").click();
  await page.waitForFunction(() => Boolean(sessionStorage.getItem("canopy-editor-session-v1")));

  const publication = await page.evaluate(async () => {
    const owner = JSON.parse(localStorage.getItem("canopy-owner-session-v1"));
    const session = sessionStorage.getItem("canopy-editor-session-v1");
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
    if (locks["edit-fields"].api_version !== "v1alpha2") throw new Error("active Edit Fields lock is not v1alpha2");
    const workflowId = "wf-edit-fields-browser";
    await mutate("/api/v1/workflows", {
      workflow_id: workflowId,
      name: "Edit Fields Eco Browser",
      annotation: "Ticket 08 public seam fixture",
      settings: {},
      compatibility_metadata: {},
    });
    const lease = await mutate(`/api/v1/workflows/${workflowId}/editing/open`, {
      editor_session_id: session,
      label: "Edit Fields browser tab",
    });
    let version = 0;
    const eco = {
      mode: "merge",
      assignments: [
        { path: ["eco"], kind: "fixed", value: true },
        { path: ["parity"], kind: "expression", source: "$json.value % 2 === 0 ? \"even\" : \"odd\"" },
        { path: ["doubled"], kind: "expression", source: "$json.value * 2" },
        { path: ["label"], kind: "expression", source: "\"eco-\" + $json.index" },
      ],
    };
    const operations = [
      { kind: "add_node", node_instance: { id: "manual-trigger", name: "Manual Trigger", contract_lock: locks["manual-trigger"], configuration: { capture_mode: "manual" }, layout: { x: 100, y: 120 }, annotation: "", compatibility_metadata: {} } },
      { kind: "add_node", node_instance: { id: "generate-items", name: "Generate Items", contract_lock: locks["generate-items"], configuration: { count: 17, start: 0, step: 1, data: { payload: "eco-artifact" }, storage_mode: "artifact" }, layout: { x: 420, y: 120 }, annotation: "", compatibility_metadata: {} } },
      { kind: "add_node", node_instance: { id: "edit-fields", name: "Edit Fields", contract_lock: locks["edit-fields"], configuration: eco, layout: { x: 760, y: 120 }, annotation: "", compatibility_metadata: {} } },
      { kind: "connect", connection: { id: "manual-to-generate", source: { node_id: "manual-trigger", port_id: "invocation" }, target: { node_id: "generate-items", port_id: "input" } } },
      { kind: "connect", connection: { id: "generate-to-edit", source: { node_id: "generate-items", port_id: "items" }, target: { node_id: "edit-fields", port_id: "input" } } },
    ];
    for (let index = 0; index < operations.length; index += 1) {
      const accepted = await mutate(`/api/v1/workflows/${workflowId}/draft-commands`, {
        editor_session_id: session,
        lease_generation: lease.lease_generation,
        command_id: `browser-edit-fields-${index}`,
        base_draft_version: version,
        operation: operations[index],
      });
      version = accepted.draft_version;
    }
    const previewRequest = { editor_session_id: session, lease_generation: lease.lease_generation, draft_version: version };
    const preview = await mutate(`/api/v1/workflows/${workflowId}/compile-preview`, previewRequest);
    if (!preview.can_publish || preview.diagnostics.some((item) => item.severity === "error")) {
      throw new Error(`valid Edit Fields preview failed: ${JSON.stringify(preview.diagnostics)}`);
    }
    const invalid = await mutate(`/api/v1/workflows/${workflowId}/draft-commands`, {
      editor_session_id: session,
      lease_generation: lease.lease_generation,
      command_id: "browser-edit-fields-invalid",
      base_draft_version: version,
      operation: {
        kind: "configure_node",
        node_instance_id: "edit-fields",
        configuration: {
          mode: "merge",
          assignments: [{ path: ["label"], kind: "expression", source: "process.env.SECRET" }],
        },
      },
    });
    version = invalid.draft_version;
    const invalidPreview = await mutate(`/api/v1/workflows/${workflowId}/compile-preview`, {
      editor_session_id: session,
      lease_generation: lease.lease_generation,
      draft_version: version,
    });
    if (invalidPreview.can_publish || !invalidPreview.diagnostics.some((item) => item.code === "E_EXPRESSION_UNSUPPORTED")) {
      throw new Error(`unsupported expression was not diagnosed: ${JSON.stringify(invalidPreview.diagnostics)}`);
    }
    const restored = await mutate(`/api/v1/workflows/${workflowId}/draft-commands`, {
      editor_session_id: session,
      lease_generation: lease.lease_generation,
      command_id: "browser-edit-fields-restore",
      base_draft_version: version,
      operation: { kind: "configure_node", node_instance_id: "edit-fields", configuration: eco },
    });
    version = restored.draft_version;
    const finalRequest = { editor_session_id: session, lease_generation: lease.lease_generation, draft_version: version };
    const finalPreview = await mutate(`/api/v1/workflows/${workflowId}/compile-preview`, finalRequest);
    const published = await mutate(`/api/v1/workflows/${workflowId}/publish`, {
      publication_id: "publish-edit-fields-browser",
      ...finalRequest,
      compile_input_digest: finalPreview.compile_input_digest,
      acknowledged_diagnostics: finalPreview.diagnostics.filter((item) => item.requires_ack).map((item) => item.fingerprint),
    });
    return { workflowId, published };
  });
  assert.ok(publication.published);

  await page.goto(`${origin}/?workflow=${publication.workflowId}`);
  await page.getByTestId("start-run").click();
  await page.getByTestId("transform-progress").waitFor({ timeout: 30_000 });
  await waitAttribute(page.getByTestId("run-durable-state"), "data-state", "succeeded", 900);
  assert.match((await page.getByTestId("transform-progress").textContent()) ?? "", /Transformed 17 items/);
  assert.match((await page.getByTestId("generation-progress").textContent()) ?? "", /Encrypted spill/);
  await page.getByTestId("inspect-trace").click();
  await page.getByTestId("causal-trace").waitFor();
  assert.equal(await page.getByTestId("activation-outcome").count(), 3);
  assert.equal(await page.getByTestId("checkpoint-count").textContent(), "1");

  const seam = await page.evaluate(async (runId) => {
    const run = await (await fetch(`/api/v1/runs/${runId}`)).json();
    const trace = await (await fetch(`/api/v1/runs/${runId}/trace`)).json();
    const transform = trace.activations.find((activation) => activation.logical_order === 3);
    return {
      state: run.durable.state,
      logicalOrder: run.durable.logical_order,
      outputCount: run.correctness.output_count,
      transformedCount: run.generation?.transform?.transformed_count,
      artifact: Boolean(run.generation?.artifact),
      transformOutcome: transform?.outcome,
      itemLinking: transform?.output?.item_linking,
      label: transform?.output?.stream_digest,
    };
  }, await page.getByTestId("run-id").textContent().then((value) => value.trim()));
  assert.deepEqual({
    state: seam.state,
    logicalOrder: seam.logicalOrder,
    outputCount: seam.outputCount,
    transformedCount: seam.transformedCount,
    artifact: seam.artifact,
    transformOutcome: seam.transformOutcome,
    itemLinking: seam.itemLinking,
  }, {
    state: "succeeded",
    logicalOrder: 3,
    outputCount: 17,
    transformedCount: 17,
    artifact: true,
    transformOutcome: "success",
    itemLinking: "one_to_one",
  });
  assert.match(seam.label, /^sha256:/);

  const mobile = await context.newPage();
  await mobile.setViewportSize({ width: 390, height: 900 });
  await mobile.goto(`${origin}/?workflow=${publication.workflowId}&run=${await page.getByTestId("run-id").textContent().then((value) => value.trim())}`);
  await mobile.getByTestId("transform-progress").waitFor();
  assert.equal(await mobile.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await mobile.close();
  console.log("edit-fields-browser=passed v1alpha2=passed diagnostics=passed transform=17 artifact=preserved trace-logical-order=3 mobile-overflow=0");
} finally {
  if (browser) await browser.close();
  daemon.kill("SIGTERM");
  await new Promise((resolve) => daemon.once("exit", resolve));
  await rm(root, { recursive: true, force: true });
}

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 0;
  await new Promise((resolve) => server.close(resolve));
  return port;
}
async function ready(origin) {
  for (let attempt = 0; attempt < 400; attempt += 1) {
    try {
      if ((await fetch(`${origin}/health/live`)).ok) return;
    } catch {
      // Startup race.
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error("daemon did not start");
}
async function waitAttribute(locator, name, value, attempts = 200) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if ((await locator.getAttribute(name)) === value) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`expected ${name}=${value}; got ${await locator.getAttribute(name)}`);
}
