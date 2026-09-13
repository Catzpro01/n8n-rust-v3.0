# Project context

> Bootstrap context for the Rust workflow-automation project. Items under
> **Open decisions** are not settled architecture decisions.

## Product intent

Build a Rust-based workflow automation product with a browser workflow editor,
node-based execution, persistence, and real-time execution status. The product
may offer interactions familiar to n8n users, but its implementation, branding,
assets, and visual design must be independently authored.

## Domain vocabulary

- **Workflow**: a versioned directed graph that a user can save and execute.
- **Node**: a unit of workflow behavior with typed input/output ports and a
  configuration value.
- **Edge**: a connection from an output port of one node to an input port of
  another node.
- **Execution**: one run of a workflow, with status, timestamps, logs, and
  per-node results.
- **Node adapter**: the integration that implements a node's runtime behavior.
- **Canvas**: the browser surface used to place, connect, select, and inspect
  nodes.
- **Credential**: a secret reference used by a node adapter; raw secret values
  must not be exposed in workflow JSON or logs.

## Working invariants

- Checked-out source and tests are authoritative; `cache.kv` is discovery-only.
- Every behavior change needs a focused test or an explicitly documented reason
  why a test cannot be added yet.
- A reviewer may report findings without silently changing the builder's files.
- Credentials, tokens, and private user data never enter Git or handoff files.

## Wayfinder checkpoint

- **2026-09-13:** The first focused product session should be an architecture
  spike rather than a commitment to a vertical product slice. Compare the
  browser/UI and Rust backend boundaries, keep the spike reversible, and use
  its findings to sharpen the product boundary before implementation.
- The decision map is GitHub issue [#2](https://github.com/Catzpro01/n8n-rust-v3.0/issues/2).
  The initial product-boundary frontier is [#3](https://github.com/Catzpro01/n8n-rust-v3.0/issues/3);
  downstream architecture and execution questions are tracked in #4–#7.

## Open decisions

1. Dioxus fullstack versus Leptos/Axum versus a Rust backend with a separate
   frontend.
2. Clean-room visual language and branding, including the product name.
3. Minimum node set and whether workflow JSON import/export is required.
4. Execution model: in-process MVP, Postgres-backed queue, or Redis workers.
5. SQLite-first MVP versus PostgreSQL from the first persistent slice.
6. GitHub Actions plus Codespaces versus a deployed preview environment.

Resolve these through `/wayfinder` or `/grill-with-docs`; do not silently turn
an open decision into an implementation convention. An architecture spike is
allowed only when it is explicitly reversible and does not pretend to settle
one of the open decisions.
