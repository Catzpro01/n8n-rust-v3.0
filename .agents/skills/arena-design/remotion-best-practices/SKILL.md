---
name: remotion-best-practices
description: Plan deterministic React-based video compositions, timelines, assets, captions, audio, and render validation.
mode: arena-workspace
auto_activate: true
triggers:
  - "remotion"
  - "programmatic video"
  - "video"
  - "animation"
  - "animasi"
  - "composition"
source: https://github.com/remotion-dev/remotion
source_license: Custom Remotion License
audited_at: 2026-09-11
---

# remotion-best-practices

## Use when
Use when the user requests a programmatic video, animation composition, or Remotion code review.

## Workflow
1. Define output size, frame rate, duration, codec, safe areas, narrative beats, and asset provenance.
2. Make every frame a deterministic function of frame number and props; avoid wall clock, network fetches during render, and CSS transitions with hidden timing.
3. Preload assets, bound memory, split long work into compositions/sequences, and design captions/audio for accessibility.
4. Render representative frames and a short proof segment before the full output.
5. Validate duration, dropped frames, audio levels, text clipping, licenses, and final metadata.

## Deliverables
- Composition plan/code, asset manifest, preview frames, and render report when the runtime is approved.

## Guardrails
No Remotion package or code is installed by this skill. Its repository uses a custom license: individuals, non-profits, and qualifying small companies may be eligible for free use, while other for-profit organizations need a company license. Confirm eligibility and exact terms before dependency installation or rendering. Arena speech tools cannot sing and are not a substitute for music licensing.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
