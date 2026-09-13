---
name: onlook
description: >-
  Visual React/Next.js editor that writes design changes directly back to source
  code. Apache-2.0. Inspect elements visually and have changes reflected in code.
---

# Onlook — Visual React Code Editor

> ⚠️ FILE WRITE ACCESS: Onlook has write access to your source code.
> Always use with version control. Review changes before committing.

## What It Does
Onlook lets you inspect and edit React components visually in the browser while the changes are written back to the source code in real time — like browser DevTools, but changes persist.

## Use Cases
- Adjust spacing, colors, typography visually
- Prototype layout changes without manual CSS editing
- Inspect computed styles of any component
- Explore Tailwind class changes live

## Setup
```bash
npm install -g @onlook/cli
onlook dev       # Starts dev server with Onlook overlay
```

Or add to Next.js:
```tsx
// app/layout.tsx
import { OnlookProvider } from '@onlook/next';

export default function Layout({ children }) {
  return (
    <html>
      <body>
        <OnlookProvider>
          {children}
        </OnlookProvider>
      </body>
    </html>
  );
}
```

## Agent Workflow
1. Start dev server with Onlook
2. Describe visual change needed
3. Agent identifies the component and CSS changes
4. Apply changes via Onlook or direct file edit
5. Preview in browser
6. Commit if satisfied

## Security Rules
⚠️ ALWAYS use git — Onlook writes directly to source files
⚠️ Do not run Onlook in production — dev environment only
✅ Changes are traced to specific file/line — easy to review
✅ Works entirely locally — no cloud processing of your code
