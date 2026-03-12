# Phase 0: Boundary And Style Separation

## Objective

- Separate custom interaction-surface CSS from general UI chrome.
- Lock in the migration boundary before dependencies are introduced.

## Why This Phase Exists

- `../examples/tty0-vfx-editor/src/styles.css` currently mixes theme tokens, global element rules, layout, panel styling, and custom editor surface styling in one file.
- If this is not split first, shadcn components will inherit or fight against broad element selectors.

## Target Files

- `../examples/tty0-vfx-editor/src/styles.css`
- `../examples/tty0-vfx-editor/src/main.tsx`
- `../examples/tty0-vfx-editor/src/editor/VfxEditorScreen.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ArrangementGrid.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ShapeEditorPanel.tsx`

## Deliverables

- A documented style split plan for:
- app theme tokens
- shell and layout styles
- custom arrangement surface styles
- custom shape editor styles
- preview/canvas styles

- Removal or isolation plan for broad global rules affecting:
- `button`
- `input`
- `select`
- `label`

- A file ownership map that says which UI regions are allowed to move to shadcn and which remain custom.

## Work Items

- Inventory every selector in `styles.css` and classify it as:
- token
- shell/layout
- standard control
- custom surface
- responsive override

- Identify selectors that must be removed or narrowed before migration:
- `button`
- `input`
- `select`
- `label`

- Define the intended post-split CSS layout. Recommended shape:
- `src/styles.css` as the single import
- `src/styles/theme.css`
- `src/styles/shell.css`
- `src/styles/editor-surfaces.css`
- `src/styles/preview.css`

- Document the custom-surface boundary:
- arrangement grid remains custom
- arrangement scrollbar remains custom
- placement rendering remains custom
- shape editor remains custom
- preview canvas remains custom

- Document the shell boundary:
- panels
- headers
- action rows
- field rows
- messages
- buttons
- overlays

## Exit Criteria

- There is an agreed file split for CSS responsibilities.
- There is an agreed list of global selectors to eliminate or scope.
- There is no ambiguity about whether arrangement and shape surfaces are in or out of library migration.

## Risks

- The current responsive rules may be tied to existing class names and could break if shell markup changes too early.
- Layout classes around `.workspace`, `.timeline-row`, and `.clip-row` must remain stable until resizable panels are introduced later.

## Notes

- Do not add dependencies in this phase.
- Do not convert components in this phase.
