# Phase 2: Theme And Token Bridge

## Objective

- Bridge the current VFX editor look into shadcn semantic variables without flattening the visual identity.

## Why This Phase Exists

- The current editor palette is deliberate and atmosphere-heavy.
- A direct switch to stock shadcn tokens would make the editor look generic and reduce contrast in interaction-heavy views.

## Target Files

- `../examples/tty0-vfx-editor/src/styles.css`
- `../examples/tty0-vfx-editor/src/styles/theme.css`
- `../examples/tty0-vfx-editor/src/styles/shell.css`

## Current Token Sources

- `--bg`
- `--panel`
- `--panel-2`
- `--border`
- `--text`
- `--muted`
- `--cyan`
- `--amber`
- `--danger`
- `--focus`

## Deliverables

- Semantic shadcn variable mapping for:
- `background`
- `foreground`
- `card`
- `card-foreground`
- `popover`
- `popover-foreground`
- `muted`
- `muted-foreground`
- `border`
- `input`
- `ring`
- `destructive`

- Editor-specific token namespace for:
- `--editor-cyan`
- `--editor-amber`
- `--editor-surface`
- `--editor-surface-elevated`
- `--editor-grid-line`
- `--editor-playhead`
- `--editor-selection`

- Typography decisions for:
- body font
- mono/code font
- dense stat text

## Work Items

- Translate current root tokens into shadcn semantic tokens.
- Preserve the current atmospheric background gradient.
- Define a density baseline for:
- buttons
- icon buttons
- compact numeric fields
- list cards

- Decide where raw CSS variables remain the source of truth and where Tailwind theme utilities should consume them.
- Reserve space for a future light theme without committing to one now.

## Recommended Styling Rule

- Semantic UI surfaces should consume shadcn variables.
- Custom editor surfaces should consume editor-specific variables.
- Do not hardcode raw hex values into component-level Tailwind classes if a token exists.

## Exit Criteria

- The editor can render shadcn components with the existing VFX visual language.
- Tokens are structured so a future theme variant would not require component rewrites.

## Risks

- If semantic and editor tokens are mixed carelessly, future theming becomes brittle.
- If shell tokens leak into custom surfaces, the arrangement and shape editors become harder to tune independently.

## Notes

- This phase should complete before large-scale visible component migration begins.
