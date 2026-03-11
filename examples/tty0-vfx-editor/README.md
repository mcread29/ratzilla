# tty0 VFX Editor

Standalone clip-based VFX editor for the `tty0` example.

## What it does

- opens a real `examples/tty0` project root in desktop mode
- lists record JSON files from `data/records`
- loads and edits `media_page.visualizer`
- migrates legacy `automation` to canonical `timeline` on load
- previews `chromatic_bulge_grid` against mounted audio
- saves canonical visualizer JSON back into the record file

## Web mode

```bash
npm install
npm run dev
```

Browser mode is import/export oriented. Use `Import JSON` to load a record file and `Save` to export the current visualizer JSON.

## Desktop mode

```bash
npm install
npm run tauri dev
```

Desktop mode expects you to open the `examples/tty0` directory as the project root.

## Notes

- The Tauri desktop build needs the usual platform GUI dependencies. On Linux that means the GTK/WebKit development packages required by Tauri.
- The shared VFX schema, migration logic, validation, tween compilation, and shader strings live in [`../tty0-vfx-core`](/home/mchan/dev/ratzilla_real/examples/tty0-vfx-core).
