---
name: taste-skill
description: >-
  Design taste skill enforcing typographic hierarchy, spatial rhythm, and 
  visual refinement. Eliminates generic AI visual patterns in frontend code.
---

# Taste — Design Aesthetic Skill

## Typography
- **Scale**: Use a modular scale (1.25 or 1.333 ratio) — not arbitrary px values
- **Line height**: 1.4–1.6 for body, 1.1–1.2 for headings
- **Measure**: 60–75 characters per line for body text
- **Font pairing**: Maximum 2 typeface families per project
- **Weight contrast**: Use weight (not just size) to create hierarchy

## Color
- **Palette**: 60-30-10 rule (dominant-secondary-accent)
- **Neutrals**: Slightly warm or cool — pure #000/#fff reads as cold
- **Interactive states**: Hover, active, focus, disabled — all designed
- **Dark mode**: Don't just invert — test each color independently

## Spacing Rhythm
- Everything on a base-8 grid (8px, 16px, 24px, 32px...)
- Vertical rhythm: consistent baseline grid
- Related items closer; unrelated items with more space (proximity principle)
- Internal padding always larger than external spacing for cards

## Motion
- Duration: 150–300ms for UI micro-interactions
- Easing: ease-out for elements entering; ease-in for elements leaving
- Never animate content that carries meaning (readability first)
- Respect `prefers-reduced-motion`

## "Does It Have Taste?" Test
Ask these questions before shipping:
1. Would I be embarrassed to show this to a senior designer?
2. Does every visual element earn its place?
3. Is the hierarchy clear in 3 seconds?
4. Does it look like a decision was made, or like defaults were accepted?
