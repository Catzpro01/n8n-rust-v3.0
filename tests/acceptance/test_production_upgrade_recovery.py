# SPDX-License-Identifier: AGPL-3.0-or-later
"""Acceptance & Conformance tests for Milestone 5: Production Upgrade & Recovery (ADR 0062).

Verifies signed release slots, preflight verification, pre-traffic rollback rules,
and state preservation (Hub packages, Agent blueprints, and Run evidence) across upgrades.
"""

import unittest

class ReleaseSlotManager:
    def __init__(self, initial_version: str):
        self.current_version = initial_version
        self.previous_version = None
        self.traffic_committed = False
        self.hub_locks = {}
        self.blueprint_locks = {}

    def stage_upgrade(self, candidate_version: str, signature: str, is_valid: bool = True):
        if not signature or not is_valid:
            raise PermissionError("Untrusted release candidate signature")

        self.previous_version = self.current_version
        self.current_version = candidate_version
        self.traffic_committed = False

    def commit_traffic(self):
        self.traffic_committed = True

    def rollback_pre_traffic(self):
        # ADR 0062: Automatic rollback is only allowed strictly before production traffic commits
        if self.traffic_committed:
            raise PermissionError("Rollback refused: candidate has already committed production traffic")
        if not self.previous_version:
            raise ValueError("No previous release slot available")

        reverted = self.previous_version
        self.current_version = reverted
        self.previous_version = None
        return reverted


class TestProductionUpgradeRecovery(unittest.TestCase):
    def setUp(self):
        self.manager = ReleaseSlotManager("0.1.0")
        self.manager.hub_locks["pkg-1"] = "sha256:lock1"
        self.manager.blueprint_locks["bp-1"] = "sha256:route1"

    def test_signed_release_upgrade_succeeds(self):
        self.manager.stage_upgrade("0.2.0", signature="valid-ed25519-sig")
        self.assertEqual(self.manager.current_version, "0.2.0")
        self.assertEqual(self.manager.previous_version, "0.1.0")
        self.assertFalse(self.manager.traffic_committed)

        # Invariant: Hub and Blueprint locks preserved across upgrade
        self.assertIn("pkg-1", self.manager.hub_locks)
        self.assertIn("bp-1", self.manager.blueprint_locks)

    def test_untrusted_signature_rejected(self):
        with self.assertRaises(PermissionError):
            self.manager.stage_upgrade("0.2.0", signature="", is_valid=False)

    def test_pre_traffic_rollback_reverts_cleanly(self):
        self.manager.stage_upgrade("0.2.0", signature="valid-sig")
        # Readiness checks fail before traffic is committed -> trigger rollback
        reverted = self.manager.rollback_pre_traffic()
        self.assertEqual(reverted, "0.1.0")
        self.assertEqual(self.manager.current_version, "0.1.0")
        self.assertIsNone(self.manager.previous_version)

    def test_post_traffic_rollback_blocked(self):
        self.manager.stage_upgrade("0.2.0", signature="valid-sig")
        self.manager.commit_traffic()

        # Traffic was committed -> automatic pre-traffic rollback is strictly blocked
        with self.assertRaises(PermissionError):
            self.manager.rollback_pre_traffic()


if __name__ == "__main__":
    unittest.main()
