---
name: skill-github-sync
description: >-
  Automated GitHub synchronization and version control for verified skills. Automatically commits and pushes newly crystallized or updated skills to the user's remote GitHub skills repository.
---

# Skill GitHub Sync — Automated Skill Version Control

## Purpose
Monitors the local skill registry (~/.gemini/config/skills/) and automatically commits and pushes verified skills to a remote GitHub repository (ntigravity-skills) with semantic commit messages whenever a new skill is crystallized or updated.

## Capabilities
- Automatically detects new and modified skills in ~/.gemini/config/skills/.
- Verifies remote repository status via GitHub API and creates ntigravity-skills repository if it does not exist.
- Formats semantic versioning commit messages (e.g., eat(skills): sync verified skills (my-new-skill)).
- Pushes changes safely to the remote main branch.

## Execution Pattern
When a new skill is created, tested, and vetted via skill-creator / skill-vetter:
`ash
python ~/.gemini/config/skills/skill-github-sync/scripts/sync_skills.py [optional-commit-message]
`

## Security & Constraints
- Only syncs files located within ~/.gemini/config/skills/.
- Never commits .env, temporary files, or binary caches (enforced by .gitignore).
- Uses existing authenticated GITHUB_PERSONAL_ACCESS_TOKEN securely.
