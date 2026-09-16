// SPDX-License-Identifier: AGPL-3.0-or-later
import test from "node:test";
import assert from "node:assert/strict";

const CHEAPEST_VALID_LANES = {
  native: "native-cpu",
  external_process: "isolated-process",
  wasm: "wasm-sandbox",
  agent_engine: "remote-worker",
};

const LANE_ISOLATION_RANK = {
  "native-cpu": 0,
  "wasm-sandbox": 1,
  "isolated-process": 2,
  "remote-worker": 3,
};

function selectCheapestValidLane(form, requestedLane = null) {
  const cheapest = CHEAPEST_VALID_LANES[form];
  if (!cheapest) {
    throw new Error(`Unknown node form: ${form}`);
  }

  if (!requestedLane) {
    return cheapest;
  }

  const requestedRank = LANE_ISOLATION_RANK[requestedLane];
  const cheapestRank = LANE_ISOLATION_RANK[cheapest];

  if (requestedRank === undefined) {
    throw new Error(`Unknown execution lane: ${requestedLane}`);
  }

  // Invariant (ADR 0059): An owner may restrict execution only toward a safer
  // or more isolated lane, never bypassing declared capabilities or running
  // unverified non-native implementations in native-cpu.
  if (requestedRank < cheapestRank) {
    throw new Error(
      `Requested lane '${requestedLane}' violates minimum isolation for form '${form}' (minimum: '${cheapest}')`
    );
  }

  return requestedLane;
}

test("selectCheapestValidLane picks default cheapest lane for each form", () => {
  assert.equal(selectCheapestValidLane("native"), "native-cpu");
  assert.equal(selectCheapestValidLane("external_process"), "isolated-process");
  assert.equal(selectCheapestValidLane("wasm"), "wasm-sandbox");
  assert.equal(selectCheapestValidLane("agent_engine"), "remote-worker");
});

test("selectCheapestValidLane allows restricting native node to isolated-process", () => {
  const selected = selectCheapestValidLane("native", "isolated-process");
  assert.equal(selected, "isolated-process");
});

test("selectCheapestValidLane allows restricting native node to wasm-sandbox", () => {
  const selected = selectCheapestValidLane("native", "wasm-sandbox");
  assert.equal(selected, "wasm-sandbox");
});

test("selectCheapestValidLane strictly forbids external_process running in native-cpu", () => {
  assert.throws(
    () => selectCheapestValidLane("external_process", "native-cpu"),
    /violates minimum isolation/
  );
});

test("selectCheapestValidLane strictly forbids wasm running in native-cpu", () => {
  assert.throws(
    () => selectCheapestValidLane("wasm", "native-cpu"),
    /violates minimum isolation/
  );
});

test("selectCheapestValidLane allows external_process in remote-worker", () => {
  const selected = selectCheapestValidLane("external_process", "remote-worker");
  assert.equal(selected, "remote-worker");
});
