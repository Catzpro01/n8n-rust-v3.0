---
name: shadcn-ui-engine
description: >-
  Shadcn/UI and Radix Primitives component engineering pattern (21st.dev). Generates accessible, copy-pasteable, Tailwind-styled React components.
---

# Shadcn UI Engine — Component Architecture (21st.dev / Radix UI)

## Purpose
Enforces production-grade component construction using Radix UI primitives and Tailwind CSS styling, ensuring zero runtime bloat and complete accessibility out of the box.

## Guidelines
- Use Radix UI primitives for complex behaviors (Dialog, Popover, Dropdown, Accordion).
- Style with semantic Tailwind tokens via cn() utility (clsx + 	ailwind-merge).
- Keep components copy-pasteable and locally editable in components/ui/.
