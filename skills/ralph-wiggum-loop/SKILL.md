---
name: ralph-wiggum-loop
description: The Atomic Fresh-Context Autonomous Loop by Geoffrey Huntley. Executes backlogs from TODO.md/PRD.json with fresh context per task, atomic git commits, and auto-termination. Trigger with /ralph-loop.
---

# Ralph Wiggum Autonomous Loop (/ralph-loop)

## Trigger
- **Active Trigger**: Use command /ralph-loop or prompt 'jalankan ralph loop'.
- **Passive Condition**: Automatically adopted when executing large multi-step task backlogs, batch refactors, or overnight AFK sprints.

## Core Architecture
1. **Filesystem as External Memory**:
   - Maintains state in TODO.md or prd.json.
   - Checks off completed items (- [x]) and records the next task (- [ ]).
2. **Fresh Context Principle**:
   - Operates on a single atomic task per iteration to prevent context pollution and token burn.
3. **Deterministic Verification Gate**:
   - Runs tests/build before declaring an item complete.
4. **Atomic Git Commit**:
   - Commits every verified task (eat(task-name): implement and verify ...).
5. **Auto-Termination**:
   - Stops cleanly when all tasks in TODO.md are marked complete.

## Slash Command /ralph-loop Execution Workflow
1. Read or generate TODO.md with explicit checkbox items.
2. Select the top uncompleted task (- [ ]).
3. Apply 	dd-auto-loop (write test -> implement -> pass test).
4. If errors arise, engage self-healing-debug-loop.
5. Check off task (- [x]) and commit to Git.
6. Advance to the next task until 100% complete.
