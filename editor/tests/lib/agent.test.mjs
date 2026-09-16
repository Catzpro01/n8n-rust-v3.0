// SPDX-License-Identifier: AGPL-3.0-or-later
import test from "node:test";
import assert from "node:assert/strict";

const REQUIRED_SUBPORTS = [
  "engine_ref",
  "memory_refs",
  "skill_refs",
  "mcp_tool_refs",
  "policy_ref",
  "output_contract_ref",
];

function validateAgentBlueprint(blueprint) {
  if (!blueprint.blueprint_id || !blueprint.model_route) {
    throw new Error("Missing required blueprint identity or model route");
  }

  for (const subport of REQUIRED_SUBPORTS) {
    if (blueprint.subports?.[subport] === undefined) {
      throw new Error(`Missing required subport: ${subport}`);
    }
  }

  return true;
}

function classifyTurnFailure(hasSideEffectOccurred, primaryError) {
  // ADR 0061:
  // - Classified failure BEFORE external side-effects allows ordered fallback.
  // - Failure during or AFTER unknown external side-effects must become UNCERTAIN.
  if (!hasSideEffectOccurred) {
    return {
      canFallback: true,
      outcome: "fallback_applied",
    };
  }
  return {
    canFallback: false,
    outcome: "uncertain",
    reason: "Cannot safely retry after external side effect initiated",
  };
}

test("validateAgentBlueprint validates presence of all 6 typed subports", () => {
  const validBlueprint = {
    blueprint_id: "agent-researcher-v1",
    model_route: {
      primary_model_id: "claude-3-5-sonnet",
      fallback_model_ids: ["gemini-1-5-pro"],
    },
    subports: {
      engine_ref: "engine:rust-deterministic",
      memory_refs: ["memory:episodic-v1"],
      skill_refs: ["skill:web-search"],
      mcp_tool_refs: ["mcp:github"],
      policy_ref: "policy:strict-budget",
      output_contract_ref: "contract:json-report",
    },
  };

  assert.equal(validateAgentBlueprint(validBlueprint), true);

  const missingSubport = {
    ...validBlueprint,
    subports: { ...validBlueprint.subports, output_contract_ref: undefined },
  };
  assert.throws(() => validateAgentBlueprint(missingSubport), /Missing required subport: output_contract_ref/);
});

test("classifyTurnFailure allows fallback only strictly before side effects", () => {
  const preFailure = classifyTurnFailure(false, "503 Service Unavailable");
  assert.equal(preFailure.canFallback, true);
  assert.equal(preFailure.outcome, "fallback_applied");

  const postFailure = classifyTurnFailure(true, "Socket reset after tool POST");
  assert.equal(postFailure.canFallback, false);
  assert.equal(postFailure.outcome, "uncertain");
});
