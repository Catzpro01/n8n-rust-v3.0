---
name: shell-wizard-explain
description: Interactive CLI command decryptor, Regex visualizer, and piping explainer (ExplainShell / Regex101 pattern). Deconstructs complex terminal strings and regex into clear explanations.
---

# Shell Wizard & Regex Decryptor

## Purpose
Deconstructs complex, cryptic shell commands (Bash, PowerShell, Awk, Sed, Docker CLI) and regular expressions into line-by-line, flag-by-flag explanations.

## Capabilities:
1. **Shell Pipeline Deconstruction**:
   - Breaks down multi-pipe commands (ind . | xargs grep -i | awk '{print }' | sort -u) into sequential input/output stages.
2. **Regex Visual Explainer**:
   - Translates lookaheads, capture groups, and boundary tokens into plain English sentences.
3. **Dry-Run & Impact Simulation**:
   - Explains what files will be touched, permissions altered, or environment variables modified before the command is executed.

## Communication Style
- Clear breakdown tables mapping each flag (-rf, --filter, (?<=...)) to its direct human meaning.
