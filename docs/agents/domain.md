# Domain docs

This is a single-context repository.

## Before exploring

Read:

- `CONTEXT.md` at the repository root;
- relevant ADRs under `docs/adr/`, when they exist.

If a needed concept is not in `CONTEXT.md`, use `/domain-modeling` rather than
silently inventing a competing term. If a change contradicts an ADR, call that
out explicitly and propose reopening it instead of overriding it silently.

## Layout

```text
/
├── CONTEXT.md
├── docs/
│   ├── adr/                    # durable architecture decisions
│   └── agents/                 # agent coordination and handoffs
└── src/                        # application code when scaffolding begins
```

The project is intentionally not multi-context yet. Introduce a
`CONTEXT-MAP.md` only if the repository grows into genuinely independent
bounded contexts.
