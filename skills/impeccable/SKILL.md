---
name: impeccable
description: >-
  Frontend code quality and aesthetic audit skill. Audits UI code for cleanliness,
  structure, and visual polish. Eliminates common AI-generated frontend anti-patterns.
---

# Impeccable — Frontend Code Quality

## Code Quality Standards

### Component Structure
```tsx
// ✅ Impeccable component structure
interface ButtonProps {
  label: string;
  onClick: () => void;
  variant?: 'primary' | 'secondary' | 'ghost';
  disabled?: boolean;
  loading?: boolean;
}

export function Button({ label, onClick, variant = 'primary', disabled, loading }: ButtonProps) {
  return (
    <button
      className={cn(buttonVariants({ variant }), { 'opacity-50 cursor-not-allowed': disabled })}
      onClick={disabled ? undefined : onClick}
      disabled={disabled || loading}
      aria-label={label}
      aria-busy={loading}
    >
      {loading ? <Spinner size="sm" /> : label}
    </button>
  );
}
```

### Anti-Patterns to Eliminate
- ❌ `className="text-gray-500 dark:text-gray-400 hover:text-gray-600 dark:hover:text-gray-300..."` → Use semantic tokens
- ❌ Inline styles anywhere
- ❌ `z-index: 9999` → Use z-index scale
- ❌ Magic numbers in CSS (margin: 13px)
- ❌ Non-semantic HTML (div soup)
- ❌ Missing aria attributes on interactive elements
- ❌ Missing error and loading states
- ❌ Hardcoded colors outside design token system

### File Organization
```
components/
├── Button/
│   ├── Button.tsx        # Component
│   ├── Button.test.tsx   # Tests
│   ├── Button.stories.tsx # Storybook (optional)
│   └── index.ts          # Barrel export
```

## Review Checklist
- [ ] Props are typed with TypeScript interfaces
- [ ] All interactive elements have aria attributes
- [ ] Loading, error, and empty states are handled
- [ ] Colors come from design tokens, not hardcoded hex
- [ ] Component is testable in isolation
- [ ] No inline styles
- [ ] Keyboard navigation works
