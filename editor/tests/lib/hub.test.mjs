// SPDX-License-Identifier: AGPL-3.0-or-later
import test from "node:test";
import assert from "node:assert/strict";

function computeSideBySideDiff(current, candidate) {
  const currentCaps = new Set(current?.capabilities || []);
  const candidateCaps = new Set(candidate.capabilities || []);

  const capabilityChanges = [];
  for (const cap of candidateCaps) {
    if (!currentCaps.has(cap)) {
      capabilityChanges.push(`+ added capability: ${cap}`);
    }
  }
  for (const cap of currentCaps) {
    if (!candidateCaps.has(cap)) {
      capabilityChanges.push(`- removed capability: ${cap}`);
    }
  }

  const breakingChanges =
    capabilityChanges.length > 0 || candidateCaps.has("network_outbound");

  return {
    candidatePackageId: candidate.packageId,
    currentVersion: current?.version || null,
    candidateVersion: candidate.version,
    capabilityChanges,
    compatibilityStatus: "verified_clean_room",
    breakingChanges,
  };
}

function evaluatePackageRetirement(pkgId, activeReferences) {
  return {
    packageId: pkgId,
    activeReferences,
    isRetired: true,
    canSafelyPurge: activeReferences === 0,
  };
}

test("computeSideBySideDiff detects added capabilities and breaking changes", () => {
  const current = {
    packageId: "pkg-slack-v1",
    version: "1.0.0",
    capabilities: ["read_only"],
  };
  const candidate = {
    packageId: "pkg-slack-v2",
    version: "2.0.0",
    capabilities: ["read_only", "network_outbound"],
  };

  const diff = computeSideBySideDiff(current, candidate);
  assert.equal(diff.candidateVersion, "2.0.0");
  assert.equal(diff.currentVersion, "1.0.0");
  assert.equal(diff.capabilityChanges.length, 1);
  assert.match(diff.capabilityChanges[0], /\+ added capability: network_outbound/);
  assert.equal(diff.breakingChanges, true);
});

test("computeSideBySideDiff marks identical capabilities as non-breaking", () => {
  const current = {
    packageId: "pkg-text-helper",
    version: "1.0.0",
    capabilities: ["pure_transform"],
  };
  const candidate = {
    packageId: "pkg-text-helper",
    version: "1.0.1",
    capabilities: ["pure_transform"],
  };

  const diff = computeSideBySideDiff(current, candidate);
  assert.equal(diff.capabilityChanges.length, 0);
  assert.equal(diff.breakingChanges, false);
});

test("evaluatePackageRetirement protects packages with active durable references", () => {
  // If historical revisions or active runs reference the package, purge is blocked
  const statusActive = evaluatePackageRetirement("pkg-slack-v1", 3);
  assert.equal(statusActive.isRetired, true);
  assert.equal(statusActive.canSafelyPurge, false);

  // If 0 active references, purge is safe
  const statusUnused = evaluatePackageRetirement("pkg-slack-v0", 0);
  assert.equal(statusUnused.isRetired, true);
  assert.equal(statusUnused.canSafelyPurge, true);
});
