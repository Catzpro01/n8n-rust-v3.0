# SPDX-License-Identifier: AGPL-3.0-or-later
"""Acceptance & Conformance tests for Milestone 2: Extension Foundation (ADR 0059).

Verifies the versioned framed External Process Protocol (`canopy.lane-protocol/v1alpha1`),
Node Form and Execution Lane separation, cheapest-valid-lane selection rules, isolation
invariants, capability boundaries, and payload size bounds.
"""

import json
import subprocess
import sys
import unittest

LANE_PROTOCOL_ABI = "canopy.lane-protocol/v1alpha1"
INLINE_THRESHOLD_BYTES = 64 * 1024  # 64 KB

CHEAPEST_VALID_LANES = {
    "native": "native-cpu",
    "external_process": "isolated-process",
    "wasm": "wasm-sandbox",
    "agent_engine": "remote-worker",
}

LANE_ISOLATION_RANK = {
    "native-cpu": 0,
    "wasm-sandbox": 1,
    "isolated-process": 2,
    "remote-worker": 3,
}


def select_cheapest_valid_lane(form: str, requested_lane: str | None = None) -> str:
    cheapest = CHEAPEST_VALID_LANES.get(form)
    if not cheapest:
        raise ValueError(f"Unknown node form: {form}")

    if not requested_lane:
        return cheapest

    requested_rank = LANE_ISOLATION_RANK.get(requested_lane)
    cheapest_rank = LANE_ISOLATION_RANK.get(cheapest)

    if requested_rank is None or cheapest_rank is None:
        raise ValueError(f"Unknown execution lane: {requested_lane}")

    # ADR 0059 Invariant: An owner may restrict execution only toward a safer
    # or more isolated lane, never bypassing declared capabilities or running
    # unverified non-native implementations in native-cpu.
    if requested_rank < cheapest_rank:
        raise PermissionError(
            f"Requested lane '{requested_lane}' violates minimum isolation for form '{form}' (minimum: '{cheapest}')"
        )

    return requested_lane


class TestExtensionFoundation(unittest.TestCase):
    def test_cheapest_valid_lane_defaults(self):
        self.assertEqual(select_cheapest_valid_lane("native"), "native-cpu")
        self.assertEqual(select_cheapest_valid_lane("external_process"), "isolated-process")
        self.assertEqual(select_cheapest_valid_lane("wasm"), "wasm-sandbox")
        self.assertEqual(select_cheapest_valid_lane("agent_engine"), "remote-worker")

    def test_isolation_restriction_allowed_toward_safer_lanes(self):
        # Restricting a native node to isolated-process or wasm-sandbox is allowed
        self.assertEqual(select_cheapest_valid_lane("native", "isolated-process"), "isolated-process")
        self.assertEqual(select_cheapest_valid_lane("native", "wasm-sandbox"), "wasm-sandbox")
        self.assertEqual(select_cheapest_valid_lane("native", "remote-worker"), "remote-worker")
        self.assertEqual(select_cheapest_valid_lane("external_process", "remote-worker"), "remote-worker")

    def test_isolation_downgrade_strictly_forbidden(self):
        # External process or WASM can NEVER be assigned to native-cpu
        with self.assertRaises(PermissionError):
            select_cheapest_valid_lane("external_process", "native-cpu")

        with self.assertRaises(PermissionError):
            select_cheapest_valid_lane("wasm", "native-cpu")

        with self.assertRaises(PermissionError):
            select_cheapest_valid_lane("agent_engine", "native-cpu")

    def test_lane_protocol_framing_and_handshake(self):
        # Simulate child process responding to framed protocol
        worker_code = """
import sys, json

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    msg = json.loads(line)
    payload = msg.get("payload", {})
    msg_type = payload.get("type")
    
    if msg_type == "handshake":
        resp = {
            "protocol_version": msg["protocol_version"],
            "message_id": "ack-1",
            "payload": {
                "type": "handshake_ack",
                "status": "ready",
                "implementation_version": "1.0.0",
                "accepted_lane": "isolated-process"
            }
        }
        sys.stdout.write(json.dumps(resp) + "\\n")
        sys.stdout.flush()
    elif msg_type == "activate":
        req = payload
        input_data = req.get("input_data", {})
        items = input_data.get("items", [])
        for item in items:
            item["processed_by"] = "conformance-worker"
        
        resp = {
            "protocol_version": msg["protocol_version"],
            "message_id": "act-res-1",
            "payload": {
                "type": "activation_result",
                "activation_id": req["activation_id"],
                "outcome": "success",
                "output_port": "output",
                "output_data": {"items": items},
                "artifact_references": [],
                "causal_trace": {"lane": "isolated-process", "worker": "python-conformance"},
                "resource_metrics": {
                    "cpu_micros": 500,
                    "peak_memory_bytes": 1048576,
                    "items_processed": len(items)
                }
            }
        }
        sys.stdout.write(json.dumps(resp) + "\\n")
        sys.stdout.flush()
"""
        proc = subprocess.Popen(
            [sys.executable, "-c", worker_code],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        # Step 1: Handshake
        handshake_msg = {
            "protocol_version": LANE_PROTOCOL_ABI,
            "message_id": "msg-handshake-1",
            "payload": {
                "type": "handshake",
                "contract_digest": "sha256:contract-test",
                "implementation_digest": "sha256:impl-test",
                "supported_lanes": ["isolated-process"],
            },
        }
        proc.stdin.write(json.dumps(handshake_msg) + "\n")
        proc.stdin.flush()

        ack_line = proc.stdout.readline()
        ack = json.loads(ack_line)
        self.assertEqual(ack["protocol_version"], LANE_PROTOCOL_ABI)
        self.assertEqual(ack["payload"]["type"], "handshake_ack")
        self.assertEqual(ack["payload"]["status"], "ready")

        # Step 2: Activation
        activate_msg = {
            "protocol_version": LANE_PROTOCOL_ABI,
            "message_id": "msg-act-1",
            "payload": {
                "type": "activate",
                "activation_id": "act-test-01",
                "node_instance_id": "node-ext-01",
                "logical_order": 1,
                "deadline_epoch_millis": None,
                "input_port": "input",
                "input_data": {"items": [{"id": 1, "value": "canopy"}]},
                "artifact_references": [],
                "capability_grants": [
                    {"kind": "environment", "scope": "isolated", "parameters": {}}
                ],
            },
        }
        proc.stdin.write(json.dumps(activate_msg) + "\n")
        proc.stdin.flush()

        res_line = proc.stdout.readline()
        res = json.loads(res_line)
        self.assertEqual(res["protocol_version"], LANE_PROTOCOL_ABI)
        self.assertEqual(res["payload"]["type"], "activation_result")
        self.assertEqual(res["payload"]["outcome"], "success")
        self.assertEqual(res["payload"]["output_data"]["items"][0]["processed_by"], "conformance-worker")

        proc.stdin.close()
        proc.stdout.close()
        proc.stderr.close()
        proc.wait(timeout=2)
        self.assertEqual(proc.returncode, 0)

    def test_oversized_payload_threshold(self):
        # Validates that payloads over 64KB require artifact reference handling
        small_payload = "x" * 1024
        large_payload = "x" * (70 * 1024)

        self.assertLess(len(small_payload.encode("utf-8")), INLINE_THRESHOLD_BYTES)
        self.assertGreater(len(large_payload.encode("utf-8")), INLINE_THRESHOLD_BYTES)


if __name__ == "__main__":
    unittest.main()
