# Phase 5: Scroll And List Shell

## Objective

- Move standard list and shell scrolling to shadcn patterns where that improves consistency, while leaving timeline scrolling custom.

## Why This Phase Exists

- The current app uses a mix of plain overflow containers and custom scroll regions.
- Only some of them are ordinary list shells that benefit from Radix scroll-area behavior.

## Target Components

- `../examples/tty0-vfx-editor/src/editor/components/ClipLibraryPanel.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ClipCard.tsx`
- `../examples/tty0-vfx-editor/src/styles.css`

## Deliverables

- Clip library list migrated to `ScrollArea`.
- Clip cards visually aligned with shadcn card/button patterns.
- Standard list empty states and selected states normalized.

## Work Items

- Wrap clip list in `ScrollArea`.
- Keep clip ordering drag-and-drop behavior custom.
- Refactor `ClipCard` visual structure to align with tokenized card styles.
- Preserve selected, dragging, drop-before, and drop-after states.
- Keep clip color swatch and metadata layout custom where needed.

## Explicit Non-Goals

- Do not migrate `ArrangementGrid` scrolling to `ScrollArea`.
- Do not replace the overview scrollbar with a library component.
- Do not replace placement blocks with generic list items.

## Exit Criteria

- The clip library uses standard shell scrolling.
- Clip selection and drag/drop remain intact.
- No changes to arrangement scrolling behavior.

## Risks

- ScrollArea wrappers can interfere with drag hit testing if the DOM structure changes too much.
- Clip card focus styles must still read clearly during drag operations.

## Notes

- This phase is about standard list shell behavior only.
