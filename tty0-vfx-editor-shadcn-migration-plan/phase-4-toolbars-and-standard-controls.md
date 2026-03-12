# Phase 4: Toolbars And Standard Controls

## Objective

- Replace the manually styled toolbars and grouped controls with shadcn/Radix primitives.

## Why This Phase Exists

- Toolbars are currently standard controls wrapped in custom CSS.
- They benefit directly from consistent pressed, hover, focus, and disabled states.

## Target Components

- `../examples/tty0-vfx-editor/src/editor/components/ArrangementToolbar.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/PlaybackTransport.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ClipLibraryPanel.tsx`
- `../examples/tty0-vfx-editor/src/editor/components/ShapeToolbar.tsx`

## Deliverables

- Arrangement tool mode converted to `ToggleGroup`.
- Shape interaction mode converted to `ToggleGroup`.
- Toolbar icon actions converted to consistent `Button` variants.
- Compact field layout standardized across arrangement and clip editing.

## Work Items

- Convert arrangement mode buttons to single-select `ToggleGroup`.
- Convert shape mode buttons to single-select `ToggleGroup`.
- Replace icon-only action buttons with `Button size="icon"` plus tooltip support.
- Standardize compact field sizing for BPM, measures, and beats-per-measure.
- Review whether playback scrubber remains native for now or uses `Slider`.

## Control Strategy

- Use shadcn wrappers for:
- buttons
- toggle groups
- tooltips

- Keep custom or partially custom for:
- playback scrubber
- transport stat layout

## Exit Criteria

- Manual `.tool-toggle` styling is no longer the primary mechanism for standard grouped controls.
- Toolbars share consistent focus and disabled behavior.
- Pointer-heavy surfaces remain unaffected.

## Risks

- Tight toolbar density may require custom button size tokens.
- Replacing the native range input too early can change scrubber interaction behavior.

## Notes

- Prefer keeping the transport slider native in this phase unless a replacement is demonstrably better and regression-safe.
