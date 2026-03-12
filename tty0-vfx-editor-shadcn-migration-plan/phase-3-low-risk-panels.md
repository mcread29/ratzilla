# Phase 3: Low-Risk Panels

## Objective

- Migrate the safest, highest-value shell panels first.

## Why This Phase Exists

- These panels are mostly standard form and button composition.
- They provide early wins without touching the custom interaction surfaces.

## Target Components

- `../examples/tty0-vfx-editor/src/editor/components/SessionPanel.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/PropertySidebarPanel.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ClipMetadataForm.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ClipEditorPanel.tsx`

## Deliverables

- Session panel using shadcn cards and buttons.
- Property sidebar using labels, inputs, badges, and card structure.
- Clip metadata form using shadcn fields and select strategy.
- Empty states and helper text styled consistently.

## Work Items

- Convert `SessionPanel`:
- use `Card`
- use `Button` variants for primary and secondary actions
- preserve hidden file input behavior
- preserve current action ordering

- Convert `PropertySidebarPanel`:
- use `Card`
- use `Label` and `Input`
- style variable name as a badge or code pill
- keep color value transformation logic custom

- Convert `ClipMetadataForm`:
- use consistent field layout
- keep the “legacy step clip” warning visible
- prefer `native-select` in the first pass to avoid unnecessary popup complexity

- Convert empty/fallback states in `ClipEditorPanel` to a standard card-empty treatment.

## Component Mapping

- Session actions: `Button`
- Panel shell: `Card`
- Field labels: `Label`
- Numeric text input: `Input`
- Variable name / status pill: `Badge` or custom tokenized badge
- Legacy warning: `Alert` or muted block depending on visual density

## Exit Criteria

- These panels no longer depend on broad global element styling.
- They visually match the new token system.
- Their data wiring and file import behavior are unchanged.

## Risks

- Compact numeric fields may need custom size variants.
- Native color inputs may not visually match other controls and may need wrapper styling even after migration.

## Notes

- Do not migrate the SVG shape surface in this phase.
- Do not introduce dialogs or toasts yet unless they are required by panel regressions.
