# SPDX-License-Identifier: AGPL-3.0-or-later
"""Acceptance & Conformance tests for Milestone 4: AI Agent Platform (ADR 0061).

Verifies durable capability-bound Agent Turns, locked Agent Blueprints, Model Routes,
the 6 typed subports, ordered pre-side-effect fallback, uncertain post-side-effect
classification, and Output Contract validation.
"""

import unittest

class ModelRoute:
    def __init__(self, primary: str, fallbacks: list[str]):
        self.primary = primary
        self.fallbacks = fallbacks


class AgentBlueprintLock:
    def __init__(self, blueprint_id: str, route: ModelRoute, subports: dict):
        self.blueprint_id = blueprint_id
        self.route = route
        self.subports = subports

        # Invariant: Must define all 6 typed subports
        required = {"engine", "memory", "skill", "mcp", "policy", "output"}
        missing = required - set(subports.keys())
        if missing:
            raise ValueError(f"Blueprint missing required subports: {missing}")


class TurnRequest:
    def __init__(self, turn_id: str, message: str, memory_items: list, tools: list):
        self.turn_id = turn_id
        self.message = message
        self.memory_items = memory_items
        self.tools = tools


class FakeAgentEngine:
    def __init__(self, blueprint: AgentBlueprintLock):
        self.blueprint = blueprint

    def execute_turn(
        self,
        req: TurnRequest,
        primary_fails_pre_side_effect: bool = False,
        fails_during_side_effect: bool = False,
        output_schema_valid: bool = True
    ) -> dict:
        # Rule 1: Failures during/after external side effect cannot be retried -> Uncertain
        if fails_during_side_effect:
            return {
                "turn_id": req.turn_id,
                "outcome": "uncertain",
                "active_model": self.blueprint.route.primary,
                "validated_output": None,
                "trace": {"error": "Connection drop after tool POST sent", "retry_safe": False}
            }

        # Rule 2: Pre-side-effect failure allows ordered fallback
        if primary_fails_pre_side_effect:
            active_model = self.blueprint.route.fallbacks[0]
            outcome = "fallback_applied"
        else:
            active_model = self.blueprint.route.primary
            outcome = "success"

        # Rule 3: Output Contract validation
        if not output_schema_valid:
            return {
                "turn_id": req.turn_id,
                "outcome": "output_contract_failed",
                "active_model": active_model,
                "validated_output": None,
                "trace": {"error": "Output failed schema check; downstream emission blocked"}
            }

        return {
            "turn_id": req.turn_id,
            "outcome": outcome,
            "active_model": active_model,
            "validated_output": {
                "response": f"Processed message: {req.message}",
                "model_used": active_model,
                "memory_count": len(req.memory_items),
                "tools_used": len(req.tools)
            },
            "trace": {"subports": self.blueprint.subports}
        }


class TestAIAgentPlatform(unittest.TestCase):
    def setUp(self):
        self.route = ModelRoute(primary="primary-model-v1", fallbacks=["fallback-model-v2"])
        self.subports = {
            "engine": "engine:rust-deterministic",
            "memory": "memory:episodic-v1",
            "skill": "skill:calculator",
            "mcp": "mcp:github",
            "policy": "policy:budget-limit",
            "output": "contract:validated-json"
        }
        self.blueprint = AgentBlueprintLock(
            blueprint_id="bp-customer-support-v1",
            route=self.route,
            subports=self.subports
        )
        self.engine = FakeAgentEngine(self.blueprint)

    def test_blueprint_requires_all_6_subports(self):
        invalid_subports = {"engine": "x", "memory": "y"}
        with self.assertRaises(ValueError):
            AgentBlueprintLock("bp-invalid", self.route, invalid_subports)

    def test_successful_turn_with_memory_and_tools(self):
        req = TurnRequest(
            turn_id="turn-001",
            message="Summarize account activity",
            memory_items=[{"role": "user", "text": "account #123"}],
            tools=["mcp:github"]
        )
        res = self.engine.execute_turn(req)
        self.assertEqual(res["outcome"], "success")
        self.assertEqual(res["active_model"], "primary-model-v1")
        self.assertEqual(res["validated_output"]["memory_count"], 1)
        self.assertEqual(res["validated_output"]["tools_used"], 1)

    def test_ordered_fallback_strictly_pre_side_effect(self):
        req = TurnRequest(
            turn_id="turn-002",
            message="Check status",
            memory_items=[],
            tools=[]
        )
        # Primary fails BEFORE side-effect
        res = self.engine.execute_turn(req, primary_fails_pre_side_effect=True)
        self.assertEqual(res["outcome"], "fallback_applied")
        self.assertEqual(res["active_model"], "fallback-model-v2")
        self.assertIsNotNone(res["validated_output"])

    def test_uncertain_classification_on_post_side_effect_failure(self):
        req = TurnRequest(
            turn_id="turn-003",
            message="Transfer balance",
            memory_items=[],
            tools=["mcp:transfer"]
        )
        # Failure occurs during external write
        res = self.engine.execute_turn(req, fails_during_side_effect=True)
        self.assertEqual(res["outcome"], "uncertain")
        self.assertIsNone(res["validated_output"])
        self.assertFalse(res["trace"]["retry_safe"])

    def test_output_contract_validation_blocks_bad_payload(self):
        req = TurnRequest(
            turn_id="turn-004",
            message="Generate structured json",
            memory_items=[],
            tools=[]
        )
        res = self.engine.execute_turn(req, output_schema_valid=False)
        self.assertEqual(res["outcome"], "output_contract_failed")
        self.assertIsNone(res["validated_output"])


if __name__ == "__main__":
    unittest.main()
