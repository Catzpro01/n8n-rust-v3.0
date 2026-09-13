# SPDX-License-Identifier: AGPL-3.0-or-later
"""Public-seam acceptance for durable native If routing."""

from __future__ import annotations

import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from tests.acceptance.test_run_manual_trigger import Daemon, api, command, login


class IfRuntimeAcceptance(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.state = root / "state"
        self.key = root / "master.key"
        self.key.write_bytes(os.urandom(32))
        self.daemon = Daemon(self.state, self.key)
        self.addCleanup(self.daemon.stop)
        setup = api(
            self.daemon.origin,
            "/api/v1/setup",
            "POST",
            {
                "email": "owner@example.test",
                "password": "correct horse battery staple",
                "recovery_passphrase": "separate recovery phrase long",
            },
            {"Origin": self.daemon.origin},
        )
        self.assertEqual(setup[0], 201, setup[2])
        self.auth, self.mutation = login(self.daemon.origin)

    def publish_workflow(
        self,
        include_merge: bool = False,
        all_false: bool = False,
        any_logic: bool = False,
        item_count: int = 12,
        include_summary: bool = False,
        eco_fixture: bool = False,
    ) -> dict:
        origin = self.daemon.origin
        workflow_id = "wf-if-runtime"
        catalog = api(origin, "/api/v1/catalog", headers=self.auth)[2]
        locks = {
            node["contract_lock"]["name"]: node["contract_lock"]
            for node in catalog["nodes"]
        }
        self.assertTrue(
            {"manual-trigger", "generate-items", "edit-fields", "if"}.issubset(locks)
        )
        if include_merge:
            self.assertIn("merge", locks)
        created = api(
            origin,
            "/api/v1/workflows",
            "POST",
            {
                "workflow_id": workflow_id,
                "name": "Deterministic If Runtime",
                "annotation": "Ticket 09 public seam",
                "settings": {},
                "compatibility_metadata": {},
            },
            self.mutation,
        )
        self.assertEqual(created[0], 201, created[2])
        lease = api(
            origin,
            f"/api/v1/workflows/{workflow_id}/editing/open",
            "POST",
            {"editor_session_id": "if-tab", "label": "If acceptance"},
            self.mutation,
        )
        self.assertEqual(lease[0], 200, lease[2])
        generation = lease[2]["lease_generation"]
        version = 0
        condition_expression = (
            '$json.parity === "never"'
            if all_false
            else '$json.parity === "even"'
        )
        conditions = [{"expression": condition_expression}]
        if any_logic:
            conditions.append({"expression": "$json.value === 11"})
        assignments = [
            {
                "path": ["parity"],
                "kind": "expression",
                "source": '$json.value % 2 === 0 ? "even" : "odd"',
            },
            {
                "path": ["label"],
                "kind": "expression",
                "source": '"eco-" + $json.index',
            },
        ]
        if eco_fixture:
            assignments = [
                {"path": ["eco"], "kind": "fixed", "value": True},
                *assignments[:1],
                {
                    "path": ["doubled"],
                    "kind": "expression",
                    "source": "$json.value * 2",
                },
                assignments[1],
            ]
        operations = [
            {
                "kind": "add_node",
                "node_instance": {
                    "id": "manual-trigger",
                    "name": "Manual Trigger",
                    "contract_lock": locks["manual-trigger"],
                    "configuration": {"capture_mode": "manual"},
                    "layout": {"x": 100, "y": 120},
                    "annotation": "",
                    "compatibility_metadata": {},
                },
            },
            {
                "kind": "add_node",
                "node_instance": {
                    "id": "generate-items",
                    "name": "Generate Items",
                    "contract_lock": locks["generate-items"],
                    "configuration": {
                        "count": item_count,
                        "start": 0,
                        "step": 1,
                        "data": None,
                        "storage_mode": "artifact" if eco_fixture else "auto",
                    },
                    "layout": {"x": 360, "y": 120},
                    "annotation": "",
                    "compatibility_metadata": {},
                },
            },
            {
                "kind": "add_node",
                "node_instance": {
                    "id": "edit-fields",
                    "name": "Edit Fields",
                    "contract_lock": locks["edit-fields"],
                    "configuration": {
                        "mode": "merge",
                        "assignments": assignments,
                    },
                    "layout": {"x": 620, "y": 120},
                    "annotation": "",
                    "compatibility_metadata": {},
                },
            },
            {
                "kind": "add_node",
                "node_instance": {
                    "id": "if",
                    "name": "If",
                    "contract_lock": locks["if"],
                    "configuration": {
                        "logic": "any" if any_logic else "all",
                        "conditions": conditions,
                    },
                    "layout": {"x": 900, "y": 120},
                    "annotation": "",
                    "compatibility_metadata": {},
                },
            },
            {
                "kind": "connect",
                "connection": {
                    "id": "manual-to-generate",
                    "source": {"node_id": "manual-trigger", "port_id": "invocation"},
                    "target": {"node_id": "generate-items", "port_id": "input"},
                },
            },
            {
                "kind": "connect",
                "connection": {
                    "id": "generate-to-edit",
                    "source": {"node_id": "generate-items", "port_id": "items"},
                    "target": {"node_id": "edit-fields", "port_id": "input"},
                },
            },
            {
                "kind": "connect",
                "connection": {
                    "id": "edit-to-if",
                    "source": {"node_id": "edit-fields", "port_id": "item"},
                    "target": {"node_id": "if", "port_id": "input"},
                },
            },
        ]
        if include_merge:
            operations.extend(
                [
                    {
                        "kind": "add_node",
                        "node_instance": {
                            "id": "merge",
                            "name": "Merge",
                            "contract_lock": locks["merge"],
                            "configuration": {"mode": "true_then_false"},
                            "layout": {"x": 1180, "y": 120},
                            "annotation": "",
                            "compatibility_metadata": {},
                        },
                    },
                    {
                        "kind": "connect",
                        "connection": {
                            "id": "if-true-to-merge",
                            "source": {"node_id": "if", "port_id": "true"},
                            "target": {"node_id": "merge", "port_id": "true"},
                        },
                    },
                    {
                        "kind": "connect",
                        "connection": {
                            "id": "if-false-to-merge",
                            "source": {"node_id": "if", "port_id": "false"},
                            "target": {"node_id": "merge", "port_id": "false"},
                        },
                    },
                ]
            )
        if include_summary:
            self.assertIn("summarize", locks)
            operations.extend(
                [
                    {
                        "kind": "add_node",
                        "node_instance": {
                            "id": "summarize",
                            "name": "Summarize",
                            "contract_lock": locks["summarize"],
                            "configuration": {"operation": "output_digest"},
                            "layout": {"x": 1440, "y": 120},
                            "annotation": "",
                            "compatibility_metadata": {},
                        },
                    },
                    {
                        "kind": "connect",
                        "connection": {
                            "id": "merge-to-summarize",
                            "source": {"node_id": "merge", "port_id": "items"},
                            "target": {"node_id": "summarize", "port_id": "input"},
                        },
                    },
                ]
            )
        for index, operation in enumerate(operations):
            response = api(
                origin,
                f"/api/v1/workflows/{workflow_id}/draft-commands",
                "POST",
                command("if-tab", generation, f"if-command-{index}", version, operation),
                self.mutation,
            )
            self.assertEqual(response[0], 200, response[2])
            version = response[2]["draft_version"]
        preview_request = {
            "editor_session_id": "if-tab",
            "lease_generation": generation,
            "draft_version": version,
        }
        preview = api(
            origin,
            f"/api/v1/workflows/{workflow_id}/compile-preview",
            "POST",
            preview_request,
            self.mutation,
        )
        self.assertEqual(preview[0], 200, preview[2])
        self.assertTrue(preview[2]["can_publish"], preview[2])
        warnings = [
            diagnostic["fingerprint"]
            for diagnostic in preview[2]["diagnostics"]
            if diagnostic["requires_ack"]
        ]
        published = api(
            origin,
            f"/api/v1/workflows/{workflow_id}/publish",
            "POST",
            {
                "publication_id": "publish-if-runtime",
                **preview_request,
                "compile_input_digest": preview[2]["compile_input_digest"],
                "acknowledged_diagnostics": warnings,
            },
            self.mutation,
        )
        self.assertEqual(published[0], 201, published[2])
        return api(origin, f"/api/v1/workflows/{workflow_id}/publication", headers=self.auth)[2]

    def wait_terminal(self, run_id: str) -> dict:
        for _ in range(4_000):
            response = api(self.daemon.origin, f"/api/v1/runs/{run_id}", headers=self.auth)
            self.assertEqual(response[0], 200, response[2])
            if response[2]["durable"]["terminal"]:
                return response[2]
            time.sleep(0.01)
        self.fail("If runtime Run did not become terminal")

    def test_transformed_items_are_routed_and_persisted(self):
        publication = self.publish_workflow()
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        admitted = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/runs",
            "POST",
            {
                "run_request_id": "run-if-runtime",
                "publication_event_id": event["event_id"],
                "revision_id": current["revision_id"],
                "plan_digest": current["plan_digest"],
                "captured_invocation": {"manual": True, "fixture": "if-runtime"},
            },
            self.mutation,
        )
        self.assertEqual(admitted[0], 201, admitted[2])
        run_id = admitted[2]["run"]["run_id"]
        terminal = self.wait_terminal(run_id)

        self.assertEqual(terminal["durable"]["state"], "succeeded")
        self.assertEqual(terminal["durable"]["logical_order"], 4)
        self.assertEqual(terminal["correctness"]["attempted"], 4)
        self.assertEqual(terminal["correctness"]["succeeded"], 4)
        self.assertEqual(terminal["correctness"]["output_count"], 12)
        generation = terminal["generation"]
        self.assertEqual(generation["generated_count"], 12)
        self.assertEqual(generation["transform"]["transformed_count"], 12)
        branch = generation["branch"]
        self.assertEqual(branch["node_instance_id"], "if")
        self.assertEqual(branch["true_count"], 6)
        self.assertEqual(branch["false_count"], 6)
        self.assertRegex(branch["stream_digest"], r"^sha256:[0-9a-f]{64}$")

        trace_response = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}/trace",
            headers=self.auth,
        )
        self.assertEqual(trace_response[0], 200, trace_response[2])
        trace = trace_response[2]
        self.assertTrue(trace["integrity_verified"])
        self.assertEqual(
            [activation["logical_order"] for activation in trace["activations"]],
            [1, 2, 3, 4],
        )
        branch_activation = trace["activations"][3]
        self.assertEqual(branch_activation["node_instance_id"], "if")
        self.assertEqual(branch_activation["output"]["true_count"], 6)
        self.assertEqual(branch_activation["output"]["false_count"], 6)
        self.assertEqual(branch_activation["provenance"]["output_ports"], ["true", "false"])
        self.assertTrue(branch_activation["provenance"]["item_linking"] == "one_to_one")

        branch_events = [
            event
            for event in trace["events"]
            if event["event_type"] == "activation_outcome"
            and event["payload"].get("node_instance_id") == "if"
        ]
        self.assertEqual(len(branch_events), 1)
        self.assertEqual(branch_events[0]["payload"]["true_count"], 6)
        self.assertEqual(branch_events[0]["payload"]["false_count"], 6)
        self.assertTrue(branch_events[0]["payload"]["exactly_one_output_per_item"])

    def test_any_logic_routes_composed_predicate(self):
        publication = self.publish_workflow(any_logic=True)
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        admitted = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/runs",
            "POST",
            {
                "run_request_id": "run-if-any-logic",
                "publication_event_id": event["event_id"],
                "revision_id": current["revision_id"],
                "plan_digest": current["plan_digest"],
                "captured_invocation": {"manual": True, "fixture": "if-any-logic"},
            },
            self.mutation,
        )
        self.assertEqual(admitted[0], 201, admitted[2])
        terminal = self.wait_terminal(admitted[2]["run"]["run_id"])

        self.assertEqual(terminal["durable"]["state"], "succeeded")
        self.assertEqual(terminal["durable"]["logical_order"], 4)
        self.assertEqual(terminal["correctness"]["output_count"], 12)
        branch = terminal["generation"]["branch"]
        self.assertEqual(branch["true_count"], 7)
        self.assertEqual(branch["false_count"], 5)
        self.assertRegex(branch["stream_digest"], r"^sha256:[0-9a-f]{64}$")

    def test_bounded_queue_preserves_every_if_route_under_backpressure(self):
        publication = self.publish_workflow(item_count=1024)
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        admitted = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/runs",
            "POST",
            {
                "run_request_id": "run-if-bounded-queue",
                "publication_event_id": event["event_id"],
                "revision_id": current["revision_id"],
                "plan_digest": current["plan_digest"],
                "captured_invocation": {"manual": True, "fixture": "if-bounded-queue"},
            },
            self.mutation,
        )
        self.assertEqual(admitted[0], 201, admitted[2])
        terminal = self.wait_terminal(admitted[2]["run"]["run_id"])

        self.assertEqual(terminal["durable"]["state"], "succeeded")
        self.assertEqual(terminal["correctness"]["output_count"], 1024)
        generation = terminal["generation"]
        self.assertEqual(generation["generated_count"], 1024)
        self.assertEqual(generation["transform"]["transformed_count"], 1024)
        branch = generation["branch"]
        self.assertEqual(branch["true_count"], 512)
        self.assertEqual(branch["false_count"], 512)
        self.assertGreaterEqual(generation["backpressure_events"], 1)
        self.assertRegex(branch["stream_digest"], r"^sha256:[0-9a-f]{64}$")

    def test_cancellation_closes_if_without_routing_and_does_not_block_next_run(self):
        publication = self.publish_workflow()
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        path = "/api/v1/workflows/wf-if-runtime/runs"
        request = {
            "run_request_id": "run-if-cancel-before-route",
            "publication_event_id": event["event_id"],
            "revision_id": current["revision_id"],
            "plan_digest": current["plan_digest"],
            "captured_invocation": {"manual": True, "fixture": "if-cancel-before-route"},
        }
        admitted = api(self.daemon.origin, path, "POST", request, self.mutation)
        self.assertEqual(admitted[0], 201, admitted[2])
        run_id = admitted[2]["run"]["run_id"]
        cancelled = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}/cancel",
            "POST",
            {"cancellation_request_id": "cancel-if-before-route"},
            self.mutation,
        )
        self.assertEqual(cancelled[0], 200, cancelled[2])
        self.assertTrue(cancelled[2]["accepted"])
        terminal = self.wait_terminal(run_id)
        self.assertEqual(terminal["durable"]["state"], "cancelled")
        self.assertEqual(terminal["correctness"]["output_count"], 0)
        self.assertFalse(terminal["correctness"]["complete"])

        trace = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}/trace",
            headers=self.auth,
        )
        self.assertEqual(trace[0], 200, trace[2])
        evidence = trace[2]
        self.assertTrue(evidence["integrity_verified"])
        self.assertEqual(evidence["activations"], [])
        self.assertIn(
            "run_cancelled_before_activation",
            [event["event_type"] for event in evidence["events"]],
        )
        self.assertFalse(
            any(
                activation["node_instance_id"] == "if"
                for activation in evidence["activations"]
            )
        )

        next_request = {
            **request,
            "run_request_id": "run-if-after-cancel",
            "captured_invocation": {"manual": True, "fixture": "if-after-cancel"},
        }
        next_admission = api(self.daemon.origin, path, "POST", next_request, self.mutation)
        self.assertEqual(next_admission[0], 201, next_admission[2])
        next_terminal = self.wait_terminal(next_admission[2]["run"]["run_id"])
        self.assertEqual(next_terminal["durable"]["state"], "succeeded")
        self.assertEqual(next_terminal["generation"]["branch"]["true_count"], 6)
        self.assertEqual(next_terminal["generation"]["branch"]["false_count"], 6)

    def test_closed_branches_are_reduced_and_persisted(self):
        publication = self.publish_workflow(include_merge=True)
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        admitted = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/runs",
            "POST",
            {
                "run_request_id": "run-merge-runtime",
                "publication_event_id": event["event_id"],
                "revision_id": current["revision_id"],
                "plan_digest": current["plan_digest"],
                "captured_invocation": {"manual": True, "fixture": "merge-runtime"},
            },
            self.mutation,
        )
        self.assertEqual(admitted[0], 201, admitted[2])
        run_id = admitted[2]["run"]["run_id"]
        terminal = self.wait_terminal(run_id)

        self.assertEqual(terminal["durable"]["state"], "succeeded")
        self.assertEqual(terminal["durable"]["logical_order"], 5)
        self.assertEqual(terminal["correctness"]["attempted"], 5)
        self.assertEqual(terminal["correctness"]["succeeded"], 5)
        self.assertEqual(terminal["correctness"]["output_count"], 12)
        merge = terminal["generation"]["merge"]
        self.assertEqual(merge["node_instance_id"], "merge")
        self.assertEqual(merge["mode"], "true_then_false")
        self.assertEqual(merge["true_count"], 6)
        self.assertEqual(merge["false_count"], 6)
        self.assertEqual(merge["output_count"], 12)
        self.assertRegex(merge["stream_digest"], r"^sha256:[0-9a-f]{64}$")
        self.assertGreater(merge["physical_spool_bytes"], 0)
        self.assertEqual(len(merge["true_segments"]), 1)
        self.assertEqual(len(merge["false_segments"]), 1)
        self.assertEqual(len(merge["output_segments"]), 1)
        self.assertEqual(
            merge["true_segments"][0]["media_type"], "application/x-ndjson"
        )

        trace_response = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}/trace",
            headers=self.auth,
        )
        self.assertEqual(trace_response[0], 200, trace_response[2])
        trace = trace_response[2]
        self.assertTrue(trace["integrity_verified"])
        self.assertEqual(
            [activation["logical_order"] for activation in trace["activations"]],
            [1, 2, 3, 4, 5],
        )
        merge_activation = trace["activations"][4]
        self.assertEqual(merge_activation["node_instance_id"], "merge")
        self.assertEqual(merge_activation["output"]["output_count"], 12)
        self.assertEqual(
            merge_activation["provenance"]["input_ports"], ["true", "false"]
        )
        self.assertEqual(
            merge_activation["provenance"]["spooling"], "artifact-backed-segments"
        )

        merge_events = [
            event
            for event in trace["events"]
            if event["event_type"] == "activation_outcome"
            and event["payload"].get("node_instance_id") == "merge"
        ]
        self.assertEqual(len(merge_events), 1)
        self.assertEqual(merge_events[0]["payload"]["output_count"], 12)
        self.assertEqual(
            merge_events[0]["payload"]["spooling"], "artifact-backed-segments"
        )

        # The reducer evidence is durable, not only live in the worker.
        old_daemon = self.daemon
        old_daemon.stop()
        self.daemon = Daemon(self.state, self.key)
        self.addCleanup(self.daemon.stop)
        self.auth, self.mutation = login(self.daemon.origin)
        reread = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}",
            headers=self.auth,
        )
        self.assertEqual(reread[0], 200, reread[2])
        self.assertEqual(reread[2]["durable"]["state"], "succeeded")
        self.assertEqual(reread[2]["generation"]["merge"]["output_count"], 12)

    def test_eco_100k_summary_is_durable_and_rollback_is_non_destructive(self):
        publication = self.publish_workflow(
            include_merge=True,
            include_summary=True,
            eco_fixture=True,
            item_count=49_998,
        )
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        admitted = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/runs",
            "POST",
            {
                "run_request_id": "run-eco-100k-summary",
                "publication_event_id": event["event_id"],
                "revision_id": current["revision_id"],
                "plan_digest": current["plan_digest"],
                "captured_invocation": {"manual": True, "fixture": "eco-100k-summary"},
            },
            self.mutation,
        )
        self.assertEqual(admitted[0], 201, admitted[2])
        run_id = admitted[2]["run"]["run_id"]
        terminal = self.wait_terminal(run_id)

        self.assertEqual(terminal["durable"]["state"], "succeeded")
        self.assertEqual(terminal["durable"]["logical_order"], 6)
        self.assertEqual(terminal["correctness"]["attempted"], 6)
        self.assertEqual(terminal["correctness"]["succeeded"], 6)
        self.assertEqual(terminal["correctness"]["output_count"], 49_998)
        generation = terminal["generation"]
        self.assertEqual(generation["generated_count"], 49_998)
        self.assertEqual(generation["transform"]["transformed_count"], 49_998)
        self.assertEqual(generation["branch"]["true_count"], 24_999)
        self.assertEqual(generation["branch"]["false_count"], 24_999)
        self.assertEqual(generation["merge"]["output_count"], 49_998)
        self.assertEqual(generation["merge"]["true_count"], 24_999)
        self.assertEqual(generation["merge"]["false_count"], 24_999)
        self.assertGreater(generation["merge"]["physical_spool_bytes"], 0)
        summary = generation["summary"]
        self.assertEqual(summary["node_instance_id"], "summarize")
        self.assertEqual(summary["operation"], "output_digest")
        self.assertEqual(summary["total_count"], 49_998)
        self.assertEqual(summary["true_count"], 24_999)
        self.assertEqual(summary["false_count"], 24_999)
        self.assertEqual(summary["logical_bytes"], 3_791_517)
        self.assertEqual(summary["first_ordinal"], 0)
        self.assertEqual(summary["last_ordinal"], 49_997)
        self.assertEqual(
            summary["output_digest"],
            "sha256:1caaeb3901bd0a17d8875a65c3363c8fbbc8c8a6695519ccbad0005e57ae9fdd",
        )

        trace_response = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}/trace",
            headers=self.auth,
        )
        self.assertEqual(trace_response[0], 200, trace_response[2])
        trace = trace_response[2]
        self.assertTrue(trace["integrity_verified"])
        self.assertEqual(
            [activation["logical_order"] for activation in trace["activations"]],
            [1, 2, 3, 4, 5, 6],
        )
        summary_activation = trace["activations"][5]
        self.assertEqual(summary_activation["node_instance_id"], "summarize")
        self.assertEqual(summary_activation["output"]["total_count"], 49_998)
        self.assertEqual(summary_activation["output"]["output_digest"], summary["output_digest"])
        self.assertEqual(
            summary_activation["provenance"]["causal_trace_links"]["retained_ordinal_range"],
            [0, 49_997],
        )
        summary_events = [
            item
            for item in trace["events"]
            if item["event_type"] == "activation_outcome"
            and item["payload"].get("node_instance_id") == "summarize"
        ]
        self.assertEqual(len(summary_events), 1)
        self.assertEqual(summary_events[0]["payload"]["output_digest"], summary["output_digest"])
        self.assertEqual(
            summary_events[0]["payload"]["causal_trace_links"]["source_merge_output_digest"],
            generation["merge"]["stream_digest"],
        )

        # Restart proves the bounded reducer result and trace are durable before rollback.
        self.daemon.stop()
        self.daemon = Daemon(self.state, self.key)
        self.addCleanup(self.daemon.stop)
        self.auth, self.mutation = login(self.daemon.origin)
        reread = api(self.daemon.origin, f"/api/v1/runs/{run_id}", headers=self.auth)
        self.assertEqual(reread[0], 200, reread[2])
        self.assertEqual(reread[2]["generation"]["summary"], summary)
        reread_trace = api(
            self.daemon.origin,
            f"/api/v1/runs/{run_id}/trace",
            headers=self.auth,
        )
        self.assertTrue(reread_trace[2]["integrity_verified"])
        self.assertEqual(reread_trace[2]["activations"][5]["output"], summary_activation["output"])

        # Publish a changed immutable revision, then roll back only the pointer.
        draft = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime",
            headers=self.auth,
        )[2]
        lease = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/editing/open",
            "POST",
            {"editor_session_id": "if-tab", "label": "Eco acceptance"},
            self.mutation,
        )
        self.assertEqual(lease[0], 200, lease[2])
        changed = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/draft-commands",
            "POST",
            command(
                "if-tab",
                lease[2]["lease_generation"],
                "eco-second-revision",
                draft["draft_version"],
                {"kind": "set_workflow_annotation", "annotation": "Eco revision two"},
            ),
            self.mutation,
        )
        self.assertEqual(changed[0], 200, changed[2])
        preview_request = {
            "editor_session_id": "if-tab",
            "lease_generation": lease[2]["lease_generation"],
            "draft_version": changed[2]["draft_version"],
        }
        preview = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/compile-preview",
            "POST",
            preview_request,
            self.mutation,
        )
        self.assertEqual(preview[0], 200, preview[2])
        second = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/publish",
            "POST",
            {
                "publication_id": "publish-eco-revision-two",
                **preview_request,
                "compile_input_digest": preview[2]["compile_input_digest"],
                "acknowledged_diagnostics": [
                    item["fingerprint"]
                    for item in preview[2]["diagnostics"]
                    if item["requires_ack"]
                ],
            },
            self.mutation,
        )
        self.assertEqual(second[0], 201, second[2])
        revision_two = second[2]["revision"]["revision_id"]
        self.assertNotEqual(revision_two, current["revision_id"])
        rolled = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/rollback",
            "POST",
            {
                "rollback_id": "rollback-eco-to-one",
                "editor_session_id": "if-tab",
                "lease_generation": lease[2]["lease_generation"],
                "draft_version": changed[2]["draft_version"],
                "target_revision_id": current["revision_id"],
            },
            self.mutation,
        )
        self.assertEqual(rolled[0], 200, rolled[2])
        status = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/publication",
            headers=self.auth,
        )
        self.assertEqual(status[2]["current_published"]["revision_id"], current["revision_id"])
        self.assertEqual(status[2]["latest_published"]["revision_id"], revision_two)
        self.assertEqual(status[2]["mutable_draft"]["draft_version"], changed[2]["draft_version"])
        self.assertEqual(status[2]["difference"]["state"], "changed")
        self.assertEqual(
            api(self.daemon.origin, "/api/v1/workflows/wf-if-runtime", headers=self.auth)[2]["annotation"],
            "Eco revision two",
        )

    def test_empty_true_branch_is_reduced_without_padding(self):
        publication = self.publish_workflow(include_merge=True, all_false=True)
        current = publication["current_published"]
        event = publication["current_event"]["envelope"]
        admitted = api(
            self.daemon.origin,
            "/api/v1/workflows/wf-if-runtime/runs",
            "POST",
            {
                "run_request_id": "run-merge-empty-true",
                "publication_event_id": event["event_id"],
                "revision_id": current["revision_id"],
                "plan_digest": current["plan_digest"],
                "captured_invocation": {"manual": True, "fixture": "merge-empty-true"},
            },
            self.mutation,
        )
        self.assertEqual(admitted[0], 201, admitted[2])
        terminal = self.wait_terminal(admitted[2]["run"]["run_id"])

        self.assertEqual(terminal["durable"]["state"], "succeeded")
        self.assertEqual(terminal["correctness"]["output_count"], 12)
        merge = terminal["generation"]["merge"]
        self.assertEqual(merge["true_count"], 0)
        self.assertEqual(merge["false_count"], 12)
        self.assertEqual(merge["output_count"], 12)
        self.assertEqual(merge["true_segments"], [])
        self.assertEqual(len(merge["false_segments"]), 1)
        self.assertEqual(len(merge["output_segments"]), 1)


if __name__ == "__main__":
    unittest.main()
