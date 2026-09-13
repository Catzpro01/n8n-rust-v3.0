---
name: test-time-compute
description: >-
  Test-Time Compute & Inference-Time Search using Monte Carlo Tree Search (MCTS) and Process Reward Models (PRM).
---

# Test-Time Compute & Inference Search

## Methodology
- Generate multiple reasoning trajectories in parallel.
- Evaluate step-level correctness using Process Reward Models (PRMs).
- Roll back failed reasoning branches using MCTS backtracking.
