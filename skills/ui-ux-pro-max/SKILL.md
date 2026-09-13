---
name: ui-ux-pro-max
description: >-
  Comprehensive UI/UX design system skill. Covers color palettes, typography,
  interaction patterns, accessibility, and component design for AI coding agents.
---

# UI/UX Pro Max — Design System Guide

## Design System Fundamentals

### Color System
- **Primary**: Brand color for key actions (1 color)
- **Secondary**: Supporting actions (1-2 colors)
- **Neutral**: Grays for text, backgrounds, borders
- **Semantic**: Success (green), Warning (amber), Error (red), Info (blue)
- **Surface**: Background layers (3-4 levels of elevation)

### Typography Scale (Recommended)
| Token | Size | Weight | Usage |
|-------|------|--------|-------|
| `display` | 48-72px | 700-900 | Hero headlines |
| `h1` | 32-40px | 700 | Page titles |
| `h2` | 24-28px | 600 | Section headers |
| `h3` | 20-22px | 600 | Sub-sections |
| `body-lg` | 18px | 400 | Primary content |
| `body` | 16px | 400 | Standard content |
| `body-sm` | 14px | 400 | Secondary content |
| `caption` | 12px | 400 | Labels, metadata |

### Spacing Scale
4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96px

### Interaction Design
- Hover: opacity 80% OR lighten/darken 10%
- Active/Pressed: scale(0.97) + darken 15%
- Focus: 2px outline, offset 2px, brand color
- Disabled: opacity 40%, cursor: not-allowed
- Transition: 150ms ease for micro-interactions

## Component Patterns

### Forms
- Labels above inputs (never placeholder-as-label)
- Error messages below input, in red, with error icon
- Success validation with green checkmark
- Required fields marked with asterisk (*) in legend
- Group related fields with fieldset

### Navigation
- Active state clearly distinguished from hover
- Breadcrumbs for deep hierarchies
- Back button always visible on mobile
- Skip navigation link for accessibility

### Data Tables
- Sticky headers for long tables
- Sortable columns with direction indicators
- Row selection with checkboxes
- Pagination or infinite scroll (not both)
- Empty state with meaningful message + action

## Accessibility Standards (WCAG 2.1 AA)
- Color contrast: 4.5:1 for text, 3:1 for UI
- All interactive elements keyboard-navigable
- Focus indicators always visible
- Images have alt text
- Form errors associated with inputs via aria-describedby
- No content conveyed by color alone
