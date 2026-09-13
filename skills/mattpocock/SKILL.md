---
name: mattpocock
description: >-
  Expert TypeScript error translator. Converts cryptic TS compiler errors into 
  clear explanations with concrete fix suggestions. Based on Matt Pocock's
  Total TypeScript teaching methodology.
---

# mattpocock — TypeScript Error Translator

## Purpose
When you encounter a TypeScript error, this skill helps translate it into plain language and suggests concrete fixes.

## How to Use
When you see a TypeScript error, share it with the agent and ask for explanation. The agent will:
1. Parse the error message structure
2. Identify the core type conflict
3. Explain it in plain language
4. Suggest 2-3 concrete fix approaches ranked by idiomatic TypeScript

## Common Error Patterns

### Type Assignability Errors
`Type 'X' is not assignable to type 'Y'`
→ Identifies why types don't overlap; suggests union types, type guards, or narrowing

### Generic Constraint Violations
`Type 'T' does not satisfy constraint 'K'`
→ Explains constraint requirements; suggests adding extends clause or widening type

### Object Property Errors
`Property 'x' does not exist on type 'Y'`
→ Suggests optional chaining, type assertion, or extending the interface

### Function Signature Mismatches
`Argument of type 'X' is not assignable to parameter of type 'Y'`
→ Compares function signatures; suggests overloads or parameter widening

## Rules
- NEVER suggest `any` as a primary fix — it defeats TypeScript's purpose
- ALWAYS prefer type narrowing over type casting
- Explain the WHY, not just the HOW
- Show before/after code snippets for each suggestion

## Security Note
⚠️ If error messages contain API keys, secrets, or PII in variable names, 
do NOT reproduce them verbatim in explanations.
