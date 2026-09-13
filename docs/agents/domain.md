# Domain docs

This is a single-context repository containing the recovered Canopy Workbench
implementation baseline and the surrounding planning tools.

## Before exploring

Read:

- `CONTEXT.md` at the repository root;
- relevant ADRs under `docs/adr/`;
- the active map and ticket under `.scratch/`.

The recovered glossary in `CONTEXT.md` is canonical for the workflow engine.
Use its terms in tickets, tests, code, and review. If a needed concept is not
present, use `/domain-modeling` rather than silently inventing a competing
term. If a change contradicts an ADR, surface the conflict and supersede it
explicitly instead of overriding it silently.

## Layout

```text
/
├── CONTEXT.md                 # current context and recovered glossary
├── Cargo.toml                 # Rust workspace
├── crates/                    # Rust node contract and workflow daemon
├── editor/                    # independently authored Preact editor assets
├── contracts/                 # versioned node contract fixtures
├── docs/
│   ├── adr/                   # retained architecture decisions
│   ├── agents/                # agent coordination and handoffs
│   ├── legal/                 # clean-room and licensing constraints
│   ├── operations/            # operator/recovery contracts
│   └── spec/                  # product and runtime specifications
├── .scratch/                  # dependency-ordered implementation tickets
├── tests/acceptance/           # public-seam and release acceptance tests
└── tools/                     # indexer and release tooling
```

The application is no longer an empty scaffold: the recovered Rust daemon and
editor are the baseline to audit and continue. Keep the current Arena/GitHub
planning layer; do not delete the recovered implementation to restart from a
new framework.
