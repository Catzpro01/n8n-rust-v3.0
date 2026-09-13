---
name: obsidian
description: >-
  Knowledge management and note-taking with local Markdown files. 
  Supports bidirectional linking, graph view, and extensive plugin ecosystem.
  Proprietary app with free personal use tier.
---

# Obsidian — Knowledge Management

## Core Concepts

### Vault Structure
```
vault/
├── 00-Inbox/          # Capture new notes here
├── 01-Projects/       # Active project notes
├── 02-Areas/          # Ongoing responsibilities
├── 03-Resources/      # Reference material
├── 04-Archive/        # Completed/inactive
└── Templates/         # Note templates
```

### Linking Philosophy (Zettelkasten)
- Every note has one atomic idea
- Link related notes with `[[Note Title]]`
- Use tags `#topic` for broad categorization
- Use backlinks to discover connections

### Daily Notes Template
```markdown
# {{date:YYYY-MM-DD}}

## Focus
- Top priority today: 

## Notes
- 

## Decisions Made
- 

## Links
- [[Yesterday]] | [[Tomorrow]]
```

## Useful Plugins (Security-Vetted)
| Plugin | Purpose | Risk |
|--------|---------|------|
| Dataview | Query notes as database | LOW |
| Templater | Advanced templates | LOW |
| Excalidraw | Diagrams inside notes | LOW |
| Tasks | Task management | LOW |
| Git | Sync vault to GitHub | MEDIUM (exposes vault) |

## Security Warnings
⚠️ **Community plugins run arbitrary JavaScript** — only install from trusted sources
⚠️ **Sync services** (Obsidian Sync, iCloud, Dropbox) may expose vault content
⚠️ **Git sync**: if vault contains secrets, `.gitignore` sensitive files
✅ For confidential notes: keep vault local-only, no sync
✅ Before installing any plugin: check plugin source code on GitHub

## Agent Integration
```
# Knowledge query from vault
Agent can:
1. Read Markdown files in vault directory
2. Search for related notes using grep
3. Create new notes following vault structure
4. Link new notes to existing related notes
```
