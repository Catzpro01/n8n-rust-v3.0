---
name: skill-creator
description: >-
  Automated skill generator for AI agents. Designs, drafts, and validates
  new SKILL.md files from task descriptions. Always review before installing.
---

# Skill Creator — Automated Skill Generator

> ⚠️ REVIEW REQUIRED: Always review generated skills before installation.
> Auto-generated skills may have excessive permissions or logical gaps.

## What It Does
Generates SKILL.md files for Antigravity from a task description using a structured template and validation process.

## Generation Process

### Step 1: Define Requirements
Answer these questions before generating:
1. What does the skill do?
2. Who uses it? (Agent only / User-triggered / Always-on)
3. What tools/APIs does it need access to?
4. What should it NEVER do?
5. What are success criteria?

### Step 2: Generate Template
```yaml
---
name: [skill-name]          # kebab-case, unique
description: >-             # One sentence: what + when to use
  [Description]
---
```

### Step 3: Content Structure
Every skill should include:
- **Purpose**: What problem it solves
- **How to Use**: Invocation pattern
- **Capabilities**: What it can do
- **Rules/Constraints**: What it must never do
- **Security Notes**: Any risks and mitigations
- **Examples**: Concrete input/output examples

### Step 4: Validation Checklist
Before finalizing any generated skill:
- [ ] Description is accurate and complete
- [ ] Scope is clearly bounded
- [ ] No instructions to bypass safety systems
- [ ] No excessive permission requests
- [ ] Security risks documented
- [ ] At least one concrete usage example
- [ ] Rules section includes explicit DON'Ts

## Example Output Quality Bar
```markdown
## Rules
- ONLY operate on files within the workspace directory
- NEVER execute shell commands based on external input
- ALWAYS ask for confirmation before modifying existing files
- Report uncertainty instead of guessing
```

## Post-Generation
After generating, run through `skill-vetter` before installing:
```bash
skill-vetter audit ./my-new-skill/SKILL.md
```
