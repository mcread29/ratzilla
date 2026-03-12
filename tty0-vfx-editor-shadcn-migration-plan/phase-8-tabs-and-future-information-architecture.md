# Phase 8: Tabs And Future Information Architecture

## Objective

- Prepare the shell for the future `State / Clips / Export` structure described in `../clip-sequencer-editor-plan.md` without blocking the current migration.

## Why This Phase Exists

- The current screen does not use tabs.
- The future sequencer plan explicitly calls for tabbed top-level editor organization.

## Target Files

- `../examples/tty0-vfx-editor/src/editor/VfxEditorScreen.tsx`
- `../clip-sequencer-editor-plan.md`

## Deliverables

- A tabbed shell plan using shadcn `Tabs`.
- A routing or state strategy for active editor sections.
- A migration map from the current layout to the future tab layout.

## Proposed Tab Model

- `State`
- `Clips`
- `Export`

## Work Items

- Decide whether tabs are:
- purely local component state
- URL-backed in web mode
- persisted in local storage

- Define what moves under each tab without breaking existing workflows.
- Decide whether the preview stays global across tabs or is owned by specific tabs.
- Ensure resizable shell work from Phase 6 does not need to be re-done when tabs arrive.

## Constraints

- Do not block Phases 0 through 7 on this information architecture change.
- Do not change the underlying custom arrangement or shape editing surfaces just to fit a tab component.

## Exit Criteria

- There is a concrete shell plan for future tabs.
- Tab introduction can happen as a separate implementation step after the shell migration is stable.

## Risks

- Tabs introduced too early will multiply layout churn while shell migration is still in motion.
- If tab ownership of the preview is unclear, the editor may gain awkward remount behavior.

## Notes

- Treat this phase as optional until the product work in `clip-sequencer-editor-plan.md` is ready to move.
- 2026-03-12 implementation note: tab introduction remains deferred. The migrated shell keeps preview and editor surfaces separable so `State / Clips / Export` can be layered on later without redoing the resizable shell.
