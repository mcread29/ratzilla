# Phase 1: Setup And Scaffolding

## Objective

- Add the minimum infrastructure needed for shadcn/ui and Radix UI inside `examples/tty0-vfx-editor`.

## Why This Phase Exists

- The package currently has only React, React DOM, Vite, and Tauri dependencies.
- There is no Tailwind, no shadcn config, and no path alias support yet.

## Target Files

- `../examples/tty0-vfx-editor/package.json`
- `../examples/tty0-vfx-editor/vite.config.ts`
- `../examples/tty0-vfx-editor/tsconfig.json`
- `../examples/tty0-vfx-editor/src/styles.css`
- `../examples/tty0-vfx-editor/components.json`
- `../examples/tty0-vfx-editor/src/lib/utils.ts`
- `../examples/tty0-vfx-editor/src/components/ui/*`

## Deliverables

- Tailwind v4 configured through the Vite plugin flow.
- `components.json` configured for:
- style `new-york`
- CSS variables enabled
- local CSS entrypoint
- local aliases

- shadcn utility support:
- `cn()` helper
- class variance support
- merge utilities

- base UI primitives added but not yet widely used.

## Recommended Dependencies

- Runtime:
- `radix-ui`
- `class-variance-authority`
- `clsx`
- `tailwind-merge`
- `lucide-react`

- Dev/build:
- `tailwindcss`
- `@tailwindcss/vite`
- `tw-animate-css`
- `shadcn`

- Optional later:
- `sonner`

## Work Items

- Add Vite Tailwind plugin wiring.
- Add TS path aliases for `@/*`.
- Create `components.json` scoped to this package.
- Create `src/lib/utils.ts` with `cn`.
- Generate or add the initial `src/components/ui` directory.
- Keep all setup local to `examples/tty0-vfx-editor`.

## Initial Component Set To Add

- `button`
- `card`
- `input`
- `label`
- `native-select`
- `toggle-group`
- `scroll-area`
- `separator`
- `tooltip`
- `alert-dialog`
- `resizable`
- `sonner`

## Exit Criteria

- The app builds with Tailwind enabled.
- shadcn components can be imported locally without touching other repo packages.
- The project has a stable alias scheme for `src/components/ui` and `src/lib`.

## Risks

- Alias changes can break relative imports if introduced carelessly.
- Tailwind setup must not interfere with Tauri build targets in `vite.config.ts`.

## Notes

- Keep dependency additions minimal and editor-local.
- Avoid introducing repo-root Tailwind or workspace config in this phase.
