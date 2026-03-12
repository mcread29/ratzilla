# tty0 VFX Editor UI Migration Plan

This directory contains the execution plan for migrating `examples/tty0-vfx-editor` to `shadcn/ui` and `Radix UI` without replacing the editor's custom interaction surfaces.

## Goal

- Move the editor shell, standard controls, overlays, and theming primitives to `shadcn/ui` and `Radix UI`.
- Keep the arrangement surface, SVG/LFO editing surface, WebGL preview, and other domain-specific interaction layers custom.
- Optimize for long-term customizability, density, and theming rather than a fast one-pass restyle.

## Scope Boundary

- Migrate:
- app shell
- panel chrome
- buttons and icon buttons
- fields and labels
- selects or native selects
- tool toggles
- scroll areas for standard lists
- status and toast messaging
- dialogs and confirmation flows
- resizable shell layout
- future tabs if and when the screen is reorganized

- Keep custom:
- arrangement grid and ruler
- arrangement overview scrollbar
- placement block rendering and drag/resize behavior
- clip drag-and-drop behavior
- shape editor SVG surface
- WebGL preview canvas

## Primary References

- Current app entry: `../examples/tty0-vfx-editor/src/App.tsx`
- Main screen layout: `../examples/tty0-vfx-editor/src/editor/VfxEditorScreen.tsx`
- Global styles: `../examples/tty0-vfx-editor/src/styles.css`
- Package manifest: `../examples/tty0-vfx-editor/package.json`
- Product context: `../examples/tty0-vfx-editor/README.md`
- Sequencer product direction: `../clip-sequencer-editor-plan.md`

## Plan Structure

- Progress tracker: [progress.md](./progress.md)
- Phase 0: [phase-0-boundary-and-style-separation.md](./phase-0-boundary-and-style-separation.md)
- Phase 1: [phase-1-setup-and-scaffolding.md](./phase-1-setup-and-scaffolding.md)
- Phase 2: [phase-2-theme-and-token-bridge.md](./phase-2-theme-and-token-bridge.md)
- Phase 3: [phase-3-low-risk-panels.md](./phase-3-low-risk-panels.md)
- Phase 4: [phase-4-toolbars-and-standard-controls.md](./phase-4-toolbars-and-standard-controls.md)
- Phase 5: [phase-5-scroll-and-list-shell.md](./phase-5-scroll-and-list-shell.md)
- Phase 6: [phase-6-resizable-layout.md](./phase-6-resizable-layout.md)
- Phase 7: [phase-7-overlays-status-and-confirmations.md](./phase-7-overlays-status-and-confirmations.md)
- Phase 8: [phase-8-tabs-and-future-information-architecture.md](./phase-8-tabs-and-future-information-architecture.md)

## Recommended Order

- Complete Phase 0 before adding dependencies.
- Complete Phase 1 and Phase 2 before converting any visible panels.
- Complete Phase 3 before touching the arrangement grid or shape editor adjacency.
- Complete Phase 4 and Phase 5 before introducing resizable layout.
- Complete Phase 6 before any tabbed shell changes.
- Treat Phase 8 as optional until the product structure in `clip-sequencer-editor-plan.md` is ready to land.

## Non-Goals

- Do not rewrite the editor into a generic dashboard layout.
- Do not replace the arrangement or shape editing surfaces with library widgets.
- Do not introduce repo-wide frontend infrastructure unless a second frontend package proves the need.
