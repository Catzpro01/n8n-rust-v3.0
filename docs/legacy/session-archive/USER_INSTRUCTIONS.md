# Standing Owner Instructions

These are explicit instructions stated by the Owner and must survive chat compaction.

## Communication and process

- Explain technical matters first in very simple Indonesian (“bahasa bayi”) using everyday analogies.
- Use `/grill-with-docs` to decompose and document consequential decisions.
- Following `/ask-matt`, keep decision interviews bounded inside the relevant Wayfinder ticket rather than conducting one unbounded interview.
- Do not begin production implementation until shared understanding is explicitly confirmed.
- Continue work until the project is complete.
- Never infer approval merely from silence or a skipped decision prompt.

## Design-skill behavior

- Automatically activate the installed design-skill suite when a request concerns design.
- Select only the smallest relevant set of skills, not all 13.
- Keep the suite inactive for pure backend/database/infrastructure work without a design signal.
- Respect explicit per-request opt-out.
- Installation target was `arena_session`, scope `curated_all`: lightweight curated wrappers for all 13 names, not literal installation/execution of every linked upstream repository.
- Playwright 1.62.1 managed Chromium is approved on the VPS for development-only browser, accessibility, desktop/mobile, and visual-regression testing.

## Product and architecture

- Initial deployment is private personal use.
- Build an independent editor that uses familiar workflow concepts and improves usability; never copy n8n source, assets, icons, text, or distinctive trade dress.
- Preserve n8n-visible workflow capability, MCP behavior, import compatibility, major features, and staged exact-version compatibility testing.
- Workflow Hub and Skill Hub should feel like an app store, including search connected directly to the catalog.
- Core/backend and ordinary production execution must be Rust and lightweight.
- Default installed target: 500 MiB RAM, 0.5 CPU core, and 10 GiB disk.
- Optimize for maximum practical efficiency without sacrificing extension flexibility.
- Heavy/non-Rust execution is remote-first and locally optional; recreate common capabilities in Rust where practical.
- Native systemd is the production deployment default. Docker and Podman remain optional and must use the same binary/data formats.
- GPU defaults Off. Safe Auto may use only healthy, eligible, certified implementations with clear CPU/remote fallback and trace evidence.
- The product should host or coordinate Hermes, OpenClaw, OpenCode CLI, MiroFish, Claude Code, Antigravity CLI, and similar agent systems while exposing installed skills and MCPs visually.
- 9Router-style routing is wanted at node, workflow, project, and global scope.

## Security

- The SSH key exposed earlier was to be replaced.
- Replacement is complete: old-key login is rejected and local old-key files are gone.
- Never embed secrets, private keys, passwords, cookies, recovery phrases, tokens, server addresses presented as secrets, nonces, wrapped keys, or Artifact paths in public repository content or handoff prose.
