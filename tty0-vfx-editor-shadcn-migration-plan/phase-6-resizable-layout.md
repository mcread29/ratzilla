# Phase 6: Resizable Layout

## Objective

- Introduce resizable shell boundaries without disturbing the custom editor surfaces inside them.

## Why This Phase Exists

- The current shell uses fixed grid columns and media-query fallbacks.
- The editor’s layout naturally maps to resizable panel groups and would benefit from user-adjustable widths.

## Target Files

- `../examples/tty0-vfx-editor/src/editor/VfxEditorScreen.tsx`
- `../examples/tty0-vfx-editor/src/styles.css`

## Candidate Boundaries

- Left shell vs preview column
- Timeline sidebar vs arrangement panel
- Clip library vs clip editor

## Deliverables

- Resizable shell layout using shadcn `ResizablePanelGroup`.
- Persistence strategy for panel sizes if desired.
- Fallback mobile layout that keeps the current stacked behavior.

## Work Items

- Replace outer grid splits with resizable groups where desktop width allows.
- Keep responsive stacked layout below the current mobile thresholds.
- Ensure arrangement and preview surfaces still fill available space correctly.
- Confirm clip editor sidebar and canvas sizing still work after shell changes.

## Constraints

- Resizable boundaries stop at the shell container edge.
- Arrangement internals remain custom and unchanged.
- Preview canvas remains custom and must continue to size from its container.

## Exit Criteria

- Desktop users can resize major shell boundaries.
- Mobile and narrow viewports remain usable.
- No regressions in timeline width calculations or preview canvas sizing.

## Risks

- `ArrangementGrid` width calculations currently depend on container measurements and may need adjustment after panel nesting changes.
- Preview aspect-ratio behavior may need refinement once panel widths become user-controlled.

## Notes

- Introduce persistence only if it is low-risk and clearly useful.
- Do not combine this phase with tabbed IA changes.
- 2026-03-12 implementation note: the first pass uses desktop-only resizable groups above `1200px` and preserves the prior stacked/mobile layout below that breakpoint.
- 2026-03-12 implementation note: panel-size persistence was intentionally deferred so the shell migration can stabilize before layout state is stored.
