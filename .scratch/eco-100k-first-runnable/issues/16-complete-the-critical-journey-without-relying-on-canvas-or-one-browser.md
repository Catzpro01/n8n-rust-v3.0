# 16: Complete the critical journey without relying on Canvas or one browser

**What to build:** The Owner can complete critical edit, compatibility, publish, run, trace, and rollback actions by keyboard/screen reader across the supported evergreen browser matrix, with Canvas as a visual enhancement rather than the only interface.

**Blocked by:** 11: Summarize the exact Eco 100K Run; 14: Prove the 100,000-Node-Instance editor seam; 15: Import the first n8n 2.39.0 compatibility subset

**Status:** complete

- [x] Structure Deck, Command Map, declarative forms, diagnostics, Connection list, Run controls, trace list, and revision history expose complete keyboard navigation and visible focus.
- [x] Screen-reader names, roles, relationships, live announcements, error association, save/conflict/progress state, and takeover prompts are meaningful and original.
- [x] Add/connect/configure/search/select/group/publish/run/cancel/inspect/rollback critical actions do not require pointer-only Canvas manipulation.
- [x] The 100,000-node fixture remains usable through virtualized accessible lists without exposing 100,000 accessibility/DOM nodes at once.
- [x] Contrast, target size, reduced motion, zoom/text scaling, high-contrast/focus states, and color-independent branch/status meaning target WCAG 2.2 AA.
- [x] Full interaction suites run on the primary browser(s), with declared smoke/accessibility coverage on current Chrome/Edge/Firefox and Safari desktop support window.
- [x] Browser-specific unsupported capabilities degrade explicitly without corrupting Drafts or packed topology state.
- [x] No external CDN font/script/style/image is required for a usable editor or test preview.
- [x] Automated accessibility tools and manual keyboard/screen-reader evidence are both required; automation alone cannot close the ticket.
