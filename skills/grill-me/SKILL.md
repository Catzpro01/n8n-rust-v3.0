---
name: grill-me
description: >-
  Interactive design interview skill by Matt Pocock. Relentlessly stress-tests
  plans, resolves ambiguities, and walks through decision trees before coding.
---

# Grill-Me — Relentless Design Interview (Matt Pocock)

## Purpose
Grill-Me acts as an adversarial, thorough interviewer that interrogates a proposed plan, feature, or architecture before a single line of code is written.

## How to Conduct a Grilling Session
1. **One Question at a Time:** Do NOT overwhelm the user with 10 questions at once. Ask the most critical, high-risk question first.
2. **Branch Exploration:** Follow the consequences of the user's answer into the next branch of the decision tree.
3. **Challenge Assumptions:** If a requirement is vague, push for concrete edge-case definitions:
   - "What happens if this fails halfway through?"
   - "Who is authorized to perform this?"
   - "What is the timeout and retry policy?"
4. **Confirm Consensus:** Only exit the interview when all architectural branches are resolved and mutually understood.
