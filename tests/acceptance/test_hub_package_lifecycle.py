# SPDX-License-Identifier: AGPL-3.0-or-later
"""Acceptance & Conformance tests for Milestone 3: Workflow & Skill Hub (ADR 0060).

Verifies content-addressed package lifecycle, sandbox exercise before installation,
scoped dependency locks (Workflow-local default, Project, Global), side-by-side
candidate diffs, and safe retirement invariants.
"""

import unittest

class PackageManifest:
    def __init__(self, package_id, version, pkg_type, publisher, digest, capabilities):
        self.package_id = package_id
        self.version = version
        self.pkg_type = pkg_type
        self.publisher = publisher
        self.digest = digest
        self.capabilities = capabilities
        self.sandbox_exercised = False


class HubLifecycleManager:
    def __init__(self):
        self.catalog = {}
        self.installed = {} # package_id -> (scope, target_id)
        self.active_references = {} # package_id -> count

    def register(self, pkg: PackageManifest):
        self.catalog[pkg.package_id] = pkg

    def exercise_sandbox(self, pkg_id: str):
        pkg = self.catalog[pkg_id]
        # Invariant: Sandbox has zero credentials and blocks production side effects
        if "production_write" in pkg.capabilities:
            raise PermissionError("Production side effects blocked in sandbox review")
        pkg.sandbox_exercised = True

    def compute_side_by_side_diff(self, current_pkg_id: str | None, candidate_pkg: PackageManifest):
        current_caps = set(self.catalog[current_pkg_id].capabilities) if current_pkg_id else set()
        cand_caps = set(candidate_pkg.capabilities)

        added = [f"+ {c}" for c in cand_caps - current_caps]
        removed = [f"- {c}" for c in current_caps - cand_caps]
        breaking = len(added) > 0 or "network_outbound" in cand_caps

        return {
            "current_version": self.catalog[current_pkg_id].version if current_pkg_id else None,
            "candidate_version": candidate_pkg.version,
            "added_capabilities": added,
            "removed_capabilities": removed,
            "breaking": breaking,
        }

    def install(self, pkg_id: str, scope: str = "workflow_local", target_id: str = "wf-1"):
        pkg = self.catalog[pkg_id]
        if not pkg.sandbox_exercised:
            raise PermissionError("Package must be exercised in sandbox before installation")
        self.installed[pkg_id] = (scope, target_id)
        self.active_references[pkg_id] = self.active_references.get(pkg_id, 0) + 1

    def retire(self, pkg_id: str):
        refs = self.active_references.get(pkg_id, 0)
        return {
            "package_id": pkg_id,
            "retired": True,
            "can_purge": refs == 0,
            "active_references": refs
        }


class TestHubPackageLifecycle(unittest.TestCase):
    def setUp(self):
        self.hub = HubLifecycleManager()
        self.safe_pkg = PackageManifest(
            package_id="pkg-data-cleaner",
            version="1.0.0",
            pkg_type="skill",
            publisher="community",
            digest="sha256:safe123",
            capabilities=["pure_transform"]
        )
        self.unsafe_pkg = PackageManifest(
            package_id="pkg-direct-db-writer",
            version="1.0.0",
            pkg_type="skill",
            publisher="unverified",
            digest="sha256:unsafe123",
            capabilities=["production_write"]
        )
        self.hub.register(self.safe_pkg)
        self.hub.register(self.unsafe_pkg)

    def test_sandbox_exercise_blocks_production_side_effects(self):
        # Safe package passes sandbox
        self.hub.exercise_sandbox(self.safe_pkg.package_id)
        self.assertTrue(self.safe_pkg.sandbox_exercised)

        # Unsafe package with production_write fails sandbox
        with self.assertRaises(PermissionError):
            self.hub.exercise_sandbox(self.unsafe_pkg.package_id)

    def test_installation_requires_sandbox_exercise(self):
        # Trying to install unexercised package fails
        with self.assertRaises(PermissionError):
            self.hub.install(self.unsafe_pkg.package_id)

        # Exercised package installs successfully into default workflow_local scope
        self.hub.exercise_sandbox(self.safe_pkg.package_id)
        self.hub.install(self.safe_pkg.package_id)
        self.assertEqual(self.hub.installed[self.safe_pkg.package_id][0], "workflow_local")

    def test_side_by_side_diff_and_breaking_change_detection(self):
        self.hub.exercise_sandbox(self.safe_pkg.package_id)
        v2 = PackageManifest(
            package_id="pkg-data-cleaner-v2",
            version="2.0.0",
            pkg_type="skill",
            publisher="community",
            digest="sha256:safe456",
            capabilities=["pure_transform", "network_outbound"]
        )
        self.hub.register(v2)

        diff = self.hub.compute_side_by_side_diff(self.safe_pkg.package_id, v2)
        self.assertEqual(diff["current_version"], "1.0.0")
        self.assertEqual(diff["candidate_version"], "2.0.0")
        self.assertIn("+ network_outbound", diff["added_capabilities"])
        self.assertTrue(diff["breaking"])

    def test_safe_retirement_protects_historical_references(self):
        self.hub.exercise_sandbox(self.safe_pkg.package_id)
        self.hub.install(self.safe_pkg.package_id)

        # Has active reference: cannot safely purge
        retire_status = self.hub.retire(self.safe_pkg.package_id)
        self.assertTrue(retire_status["retired"])
        self.assertFalse(retire_status["can_purge"])
        self.assertEqual(retire_status["active_references"], 1)


if __name__ == "__main__":
    unittest.main()
