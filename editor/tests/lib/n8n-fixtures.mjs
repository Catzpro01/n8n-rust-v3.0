// SPDX-License-Identifier: AGPL-3.0-or-later

// Inline, owner-shaped n8n 2.39.0 workflow fixtures used by the import
// compatibility tests. Stored as string literals because the fixtures
// directory is gitignored (generated binary benchmark fixtures live there).
// All fixtures are written exclusively from the documented public n8n v2
// schema (name, nodes[], connections, settings, versionId); no reverse-
// engineered internal fields are included.

export const HELLO_WORLD = JSON.stringify({
  name: "n8n 2.39.0 hello",
  versionId: "e2e-hello-world-v1",
  pinData: {},
  settings: {
    executionOrder: "v1",
    saveManualExecutions: false,
    saveDataErrorExecution: "none",
    saveDataSuccessExecution: "none",
  },
  nodes: [
    {
      id: "manual-1",
      name: "When clicking \"Test workflow\"",
      type: "n8n-nodes-base.manualTrigger",
      typeVersion: 1,
      position: [200, 200],
      parameters: {},
      webhookId: "",
    },
    {
      id: "set-1",
      name: "Set greeting",
      type: "n8n-nodes-base.set",
      typeVersion: 3,
      position: [460, 200],
      parameters: {
        assignments: {
          assignments: [
            { id: "a1", name: "message", value: "=Hello from n8n 2.39.0", type: "string" },
          ],
        },
        includeOtherFields: false,
        options: {},
      },
    },
  ],
  connections: {
    "manual-1": {
      main: [[{ node: "set-1", type: "main", index: 0 }]],
    },
  },
});

export const SECRETFUL = JSON.stringify({
  name: "secretful (must be sanitized)",
  nodes: [
    {
      id: "http-1",
      name: "HTTP Request",
      type: "n8n-nodes-base.httpRequest",
      typeVersion: 4,
      position: [200, 200],
      credentials: { httpBasicAuth: { user: "u", password: "REDACT-ME" } },
      parameters: {
        url: "https://example.com/hook",
        apiKey: "REDACT-ME-API-KEY",
        headers: { Authorization: "Bearer xyz" },
      },
    },
  ],
  connections: {},
});

export const UNSAFE_EXEC_COMMAND = JSON.stringify({
  name: "unsafe (executeCommand)",
  nodes: [
    {
      id: "m",
      name: "Manual",
      type: "n8n-nodes-base.manualTrigger",
      typeVersion: 1,
      position: [0, 0],
      parameters: {},
    },
    {
      id: "sh",
      name: "Run shell",
      type: "n8n-nodes-base.executeCommand",
      typeVersion: 1,
      position: [240, 0],
      parameters: { command: "id" },
    },
  ],
  connections: {
    m: { main: [[{ node: "sh", type: "main", index: 0 }]] },
  },
});

export const MALFORMED_NODES_STRING = JSON.stringify({
  name: "malformed (nodes not an array)",
  nodes: "not-an-array",
  connections: {},
});
