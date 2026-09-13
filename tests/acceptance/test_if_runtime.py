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

    def publish_workflow(self) -> dict:
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
                        "count": 12,
                        "start": 0,
                        "step": 1,
                        "data": None,
                        "storage_mode": "auto",
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
                        "assignments": [
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
                        ],
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
                        "logic": "all",
                        "conditions": [{"expression": '$json.parity === "even"'}],
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


if __name__ == "__main__":
    unittest.main()
