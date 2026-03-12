# Phase 7: Overlays, Status, And Confirmations

## Objective

- Replace the current inline message-only approach with a clearer split between transient feedback, persistent status, and destructive confirmations.

## Why This Phase Exists

- The controller currently writes a single message string for many kinds of feedback.
- That works, but it does not scale cleanly once the shell becomes more structured.

## Target Components

- `../examples/tty0-vfx-editor/src/editor/components/SessionPanel.tsx`
- `../examples/tty0-vfx-editor/src/editor/controller/useVfxEditorController.ts`
- future overlay components under `../examples/tty0-vfx-editor/src/components/ui`

## Deliverables

- Toasts for transient success and non-blocking status updates.
- Inline persistent status for current document state if still desired.
- Confirmation dialog strategy for destructive actions.

## Work Items

- Introduce `Sonner` for:
- saved successfully
- imported audio
- copied placements
- pasted placements
- load failure

- Keep or redesign inline persistent status for:
- loaded filename
- saved/unsaved state
- current editor message that should remain visible

- Introduce `AlertDialog` for:
- revert changes
- delete clip when a confirmation step is desired
- future destructive actions if requirements expand

## Radix Primitive Guidance

- Use shadcn wrappers for `AlertDialog` and `Sonner`.
- Use raw Radix primitives directly only if a custom anchored status popover or context menu is later introduced near custom surfaces.

## Exit Criteria

- Status feedback is categorized rather than routed through one generic inline message path.
- Destructive flows have a consistent confirmation strategy where needed.
- The shell remains dense and editor-appropriate rather than app-marketing styled.

## Risks

- Overuse of toasts can make editor feedback noisy.
- Some current messages may be both transient and persistent and need product decisions.

## Notes

- Keep this phase conservative. Not every action needs a toast.
