# 03: Decide the AI Agent Node and Agent Engine contract

Type: wayfinder-decision
Status: blocked
Blocked by: 01 — Decide the Node Form and Execution Lane extension contract
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/11

## Question

What is the production contract for the AI Agent Node and its six typed subports: Agent Engine, Memory, Skill, MCP/Tool, Policy, and Output?

Settle the turn/session protocol, built-in and external adapter lifecycle, Model Route and ordered fallback, Memory Stack reads/writes/provenance, Skill/MCP capability grants, hard budgets, Output Contract validation and bounded repair, typed suspension/failure/uncertain outcomes, redacted trace, deterministic/replay boundaries, and remote-first versus local-opt-in execution. Include Agent Blueprint scope and lock semantics without making the default Rust installation depend on heavy runtimes.

## Resolution

Pending the GitHub decision ticket. The result is an accepted AI Agent domain/API/adapter decision, glossary updates, and a vertical implementation boundary for the first Agent Node slice.
