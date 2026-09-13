---
name: hallmark
description: >-
  UI quality enforcement for AI-generated interfaces. Audits and corrects 
  common AI UI anti-patterns to ensure structured, professional visual output.
---

# Hallmark — UI Quality Standards

## Core Mandate
AI-generated UIs often have telltale signs of low quality. This skill enforces professional standards across every component.

## Visual Hierarchy Rules
1. **One primary action per screen** — no competing CTAs
2. **Maximum 3 font weights** in any single view
3. **Consistent spacing scale** — use 4px grid (4, 8, 12, 16, 24, 32, 48, 64px)
4. **Color contrast minimum** — WCAG AA (4.5:1 for text, 3:1 for UI elements)
5. **No orphan words** — headlines and body text must reflow cleanly

## Layout Standards
- **Grid system**: 12-column responsive, gutters consistent
- **Component alignment**: Left-aligned content for reading comfort; centered only for marketing
- **Whitespace**: Generous — padding inside cards ≥ 16px
- **Mobile-first**: Design for 375px width minimum

## Component Anti-Patterns (Forbidden)
- ❌ Rainbow gradient buttons on business UIs
- ❌ Shadows stacked on shadows (elevation inflation)
- ❌ Animated decorations that serve no functional purpose
- ❌ Icon + text pairs where text already says what the icon shows
- ❌ Placeholder text as labels (kills accessibility)
- ❌ Auto-playing media without controls

## Review Checklist
- [ ] Primary action is clearly dominant
- [ ] Typography scale follows harmonic ratio
- [ ] All interactive elements have visible focus states
- [ ] Error states are designed (not just happy path)
- [ ] Empty states are designed
- [ ] Loading states are designed
