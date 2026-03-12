# Progress Tracker

Use this file as the single status board for the migration.

## Overall Status

- Project: `tty0-vfx-editor` shell migration to `shadcn/ui` + `Radix UI`
- Current phase: Phase 8 review / complete
- Owner: Codex
- Last updated: 2026-03-12

## Phase Status

- [x] Phase 0: Boundary and style separation
- [x] Phase 1: Setup and scaffolding
- [x] Phase 2: Theme and token bridge
- [x] Phase 3: Low-risk panels
- [x] Phase 4: Toolbars and standard controls
- [x] Phase 5: Scroll and list shell
- [x] Phase 6: Resizable layout
- [x] Phase 7: Overlays, status, and confirmations
- [x] Phase 8: Tabs and future information architecture

## Current Risks

- Existing global `button`, `input`, `select`, and `label` rules in `../examples/tty0-vfx-editor/src/styles.css` will conflict with shadcn component styling if they are not isolated first.
- Density and spacing will likely need custom variants rather than stock shadcn spacing.
- The arrangement surface and SVG editor can be visually wrapped, but their interaction models should remain custom.
- Accessibility will improve for shell controls first, but custom surfaces still need their own keyboard and focus model.

## Decision Log

- 2026-03-12: Chosen migration boundary is hybrid rather than full-library replacement.
- 2026-03-12: Setup should remain isolated to `examples/tty0-vfx-editor` unless reuse appears elsewhere in the repo.
- 2026-03-12: `new-york` style and CSS variables are the baseline shadcn configuration.

## Completion Checklist

- [x] Tailwind v4 and shadcn infrastructure added only to `examples/tty0-vfx-editor`
- [x] Semantic UI tokens mapped from current VFX editor palette
- [x] Standard fields and actions migrated to shadcn primitives
- [x] List scrolling migrated where appropriate
- [x] Resizable shell boundaries introduced
- [x] Status/toast/confirmation flows migrated
- [x] Custom interaction surfaces preserved
- [x] No regression in playback, import/export, clip editing, or placement editing

## Notes For Implementation Agents

- Update this file at the start and end of each implementation pass.
- Mark only one active phase at a time.
- Link the implementation PR or commit under the phase notes section when work begins.

## Phase Notes

### Phase 0

- Status: Complete
- Notes:
- 2026-03-12: Split CSS into `theme.css`, `shell.css`, `editor-surfaces.css`, and `preview.css`, with `src/styles.css` as the single import entrypoint.
- 2026-03-12: Removed broad global `button`, `input`, `select`, and `label` styling so shadcn components own shell control presentation while arrangement, shape, and preview surfaces stay custom.

### Phase 1

- Status: Complete
- Notes:
- 2026-03-12: Added Tailwind v4 Vite integration, local `components.json`, `@/*` aliases, shadcn-compatible `cn()`, and editor-local UI primitives under `src/components/ui`.

### Phase 2

- Status: Complete
- Notes:
- 2026-03-12: Bridged the existing VFX palette into semantic shadcn variables and editor-specific tokens without flattening the atmospheric background treatment.

### Phase 3

- Status: Complete
- Notes:
- 2026-03-12: Migrated session, property, and clip metadata shell panels to shadcn card, badge, input, label, and native select primitives while preserving current editor logic.

### Phase 4

- Status: Complete
- Notes:
- 2026-03-12: Replaced manual toolbar toggles and icon actions with shadcn button, tooltip, and toggle-group primitives, while keeping the transport scrubber native.

### Phase 5

- Status: Complete
- Notes:
- 2026-03-12: Moved clip library list scrolling to `ScrollArea` and updated clip cards to use tokenized shell styling without touching arrangement scrolling.

### Phase 6

- Status: Complete
- Notes:
- 2026-03-12: Introduced desktop-only resizable panel groups for the major shell boundaries and kept the previous stacked layout as the fallback below `1201px`.
- 2026-03-12: Deferred panel-size persistence for now to avoid coupling early shell migration to storage semantics.

### Phase 7

- Status: Complete
- Notes:
- 2026-03-12: Split persistent inline status from transient feedback by keeping inline document state in the session panel and routing saves, audio import, copy/paste, and load failures through Sonner toasts.
- 2026-03-12: Added `AlertDialog` confirmations for revert and clip deletion while preserving controller-side safety checks for blocked deletes.

### Phase 8

- Status: Complete
- Notes:
- 2026-03-12: Deliberately deferred top-level tabs. The current shell migration keeps the layout tab-ready, but the actual `State / Clips / Export` IA change remains separate until `clip-sequencer-editor-plan.md` is ready to land.
