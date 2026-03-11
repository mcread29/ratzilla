# tty0 Shader Track Editor Plan

## Summary
Build a dev-only, in-app editor overlay for `tty0` that authors time-based shader automation tied to track playback, using BPM and measure count as the timeline basis. V1 targets only the active shader mode (`chromatic_bulge_grid`), stores authored data inside each record’s existing `visualizer` JSON, previews changes live against the mounted audio, and exports the updated `visualizer` object via clipboard-first JSON export.

## Goals
- Replace music-reactive shader motion with authored parameter automation sampled from playback time.
- Keep `idle` visuals as a separate static state from the authored `playing` timeline.
- Make the editor usable entirely inside the current archive UI with keyboard-only controls.
- Keep the runtime path clean enough that future shaders can add their own editor drivers later without changing the core archive/playback plumbing.

## Non-goals for V1
- No mouse editing.
- No multi-shader editor UI.
- No tempo maps or time-signature changes.
- No loop regions.
- No direct file writes from the browser app.
- No requirement to support compact/mobile viewport editing beyond a clear fallback message.

## Chosen product decisions
- Editor surface: dev-only overlay inside the existing `ArchiveState`.
- Dev gate: URL query flag `?editor=1`; hotkey `E` toggles the overlay only when enabled.
- Timeline basis: constant BPM plus total measure count.
- Timeline scope: only the shader mode currently in use.
- Automation model: lane keyframes.
- Playback controls: play/pause, scrub, jump by beat, jump by measure, playhead display.
- Persistence: export/copy JSON manually; no file round-trip.
- Storage location: embed editor output in the existing record JSON under `media_page.visualizer`.
- V1 timing assumption: fixed `beats_per_measure = 4`.

## Public interface and schema changes
Add a new optional field to `TrackVisualizerConfig` in [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs):
- `automation: Option<ChromaticBulgeGridAutomation>`

Add new serializable/deserializable types:
- `ChromaticBulgeGridAutomation`
- `AutomationLaneF32`
- `AutomationLaneColor`
- `FloatKeyframe`
- `ColorKeyframe`
- `InterpolationMode` with `hold` and `linear`

Use this JSON shape:
```json
"visualizer": {
  "mode": "chromatic_bulge_grid",
  "params": {
    "shader_states": {
      "playing": { ...base playing values... },
      "idle": { ...base idle values... }
    }
  },
  "automation": {
    "bpm": 132.0,
    "measures": 64,
    "beats_per_measure": 4,
    "lanes": {
      "circle_radius": [
        { "beat": 0.0, "value": 0.24, "interpolation": "hold" },
        { "beat": 16.0, "value": 0.31, "interpolation": "linear" }
      ],
      "cold_color": [
        { "beat": 0.0, "value": [1.0, 1.0, 1.0], "interpolation": "hold" }
      ]
    }
  }
}
```

Lane set for V1 must exactly match the current `chromatic_bulge_grid` uniforms:
- `motion_rate`
- `lattice_density`
- `circle_radius`
- `circle_falloff_start`
- `circle_falloff_end`
- `bulge_amount`
- `rim_guard`
- `rim_exponent`
- `rim_warp`
- `spacing_max_px`
- `spacing_min_px`
- `dot_size`
- `outer_dot_scale`
- `edge_softness`
- `chromatic_aberration`
- `scroll_base`
- `scroll_motion_scale`
- `scroll_motion_floor`
- `scroll_motion_ceiling`
- `cold_color`
- `hot_color`
- `color_cycle_rate`
- `inner_alpha`

## Runtime data flow
1. Keep `params.shader_states.playing` and `params.shader_states.idle` as the base authored states.
2. Add a playback-clock API to `AudioController` and `SessionModel` that exposes:
- `current_time_secs`
- `duration_secs`
- `is_playing`
- `seek_to_secs(f32)`
3. When the active visualizer is `chromatic_bulge_grid`, resolve shader uniforms in Rust before issuing the shader request:
- If not playing, use `idle`.
- If playing and no automation, use `playing`.
- If playing and automation exists, start from `playing` and override lane values by sampling the automation at the current beat.
4. Compute current beat as `current_time_secs * bpm / 60.0`.
5. Convert beat to measure display with fixed 4/4 for V1.
6. Keep the shader layer itself dumb: it should consume resolved uniform values, not sample automation or audio analysis internally.
7. Keep audio analysis in place for transport/status only; do not use spectral signals to drive shader motion in the authored path.

## Automation sampling rules
- Lane keyframes are sorted ascending by `beat` during load or draft normalization.
- Duplicate beats in the same lane are rejected by validation.
- Before the first keyframe, use the base `playing` value.
- Between keyframes:
- `hold`: use the earlier keyframe value.
- `linear`: interpolate to the next keyframe.
- After the last keyframe, hold the last keyframe value.
- Color lanes interpolate channel-wise for `linear`.
- All sampled values are clamped with the same runtime limits already used by the shader uniform path.

## Editor architecture
Add a new module, e.g. `visualizer_editor.rs`, owned by `ArchiveState`.

Add these editor-facing structures:
- `VisualizerEditorOverlay`
- `EditorDraftStore` keyed by record id
- `ChromaticBulgeGridEditorDraft`
- `EditorFocusArea`
- `EditorTab`
- `NumericInputState`

The overlay lives entirely inside `ArchiveState`:
- `ArchiveState::handle_key` routes keys to the editor when open.
- `ArchiveState::render` draws the archive normally, then draws the modal editor on top.
- Normal archive navigation is suspended while the editor is open.

## Editor UI layout
Render a full-screen modal over the archive content.

Use three tabs:
- `State`
- `Timeline`
- `Export`

`State` tab:
- Toggle between editing `playing base` and `idle`.
- Show the full parameter list for the active shader.
- Allow exact numeric edits for floats and RGB edits for colors.

`Timeline` tab:
- Show BPM, measure count, beats-per-measure, current time, current beat, current measure.
- Left pane: parameter lane list.
- Center pane: timeline graph for the selected lane plus playhead and keyframe markers.
- Right pane: selected keyframe inspector with beat, value, interpolation, and lane defaults.

`Export` tab:
- Show the pretty-printed `visualizer` JSON object that should replace the record’s current `visualizer` block.
- `C` copies to clipboard.
- If clipboard copy fails, log the JSON to the browser console and show a status message.

If viewport width/height is below a chosen minimum, show a dev-only message that the editor requires a wider viewport instead of attempting a cramped layout.

## Editor interactions
- `E`: open/close editor when `?editor=1` is present.
- `Esc`: close editor or exit numeric edit mode.
- `Tab`: cycle focus areas.
- `Left` / `Right`: scrub by 1 beat when not editing text.
- `[` / `]`: jump by 1 measure.
- `P`: play/pause transport.
- `N`: add keyframe on the selected lane at the current playhead beat.
- `Backspace` or `Delete`: delete selected keyframe.
- `I`: cycle interpolation for the selected keyframe.
- `Enter`: begin numeric edit for the selected field.
- Digits, `.`, and `-`: input while numeric edit mode is active.
- `C`: copy export JSON from the `Export` tab.

When inserting a keyframe:
- If a keyframe already exists at the playhead beat, select it instead of creating a duplicate.
- Seed new keyframes from the currently sampled value at that beat.

## Persistence and drafts
- Do not mutate repo files from the browser runtime.
- Keep unsaved drafts in memory per record id while the app session is open.
- Reopening the editor for the same record reuses the in-memory draft.
- Export is explicit and manual.
- The overlay shows `dirty` state whenever the draft differs from the loaded record config.

## Implementation steps
1. Extend `archive.rs` types and parsing/validation to support serialized automation.
2. Add serde `Serialize` to the visualizer/editor-owned types needed for export.
3. Add playback-clock and seek APIs to `AudioController` and `SessionModel`.
4. Add a Rust-side automation resolver for `chromatic_bulge_grid`.
5. Change the shader request path to consume resolved uniform state rather than raw reactive inputs.
6. Add the dev-flag parser and editor overlay lifetime to `ArchiveState`.
7. Build the `State` tab for editing base `playing` and `idle` values.
8. Build the `Timeline` tab for BPM/measures plus lane keyframes.
9. Build the `Export` tab with clipboard-first output.
10. Seed `0x07E2BIG` with an initial automation example to prove the full loop.

## Tests and scenarios
- Archive parsing accepts a valid `automation` block for `chromatic_bulge_grid`.
- Archive validation rejects duplicate keyframe beats in the same lane.
- Scalar sampling returns base value before first keyframe.
- Scalar `hold` interpolation works.
- Scalar `linear` interpolation works.
- Color interpolation works channel-wise.
- Paused transport resolves the `idle` state and ignores the timeline.
- Playing transport resolves `playing` base plus sampled automation overrides.
- BPM/measure duration drift warning appears when authored duration and audio duration meaningfully disagree.
- Editor hotkey is ignored unless `?editor=1` is present.
- Editor key handling consumes navigation keys while open.
- Exported JSON round-trips through the archive parser unchanged.
- Existing records without `automation` continue to render exactly as before.

## Acceptance criteria
- A developer can open `tty0` with `?editor=1`, press `E`, and edit the active shader’s base states and timeline live.
- During playback, the previewed shader changes because of authored automation over time, not because of FFT-derived reactivity.
- Pausing playback switches the preview to the authored `idle` state.
- The editor can copy a complete `visualizer` JSON object for the selected record.
- Records without automation or without the editor flag still behave normally.

## Assumptions and defaults
- V1 supports only `chromatic_bulge_grid`.
- V1 uses constant BPM and fixed 4/4.
- V1 is keyboard-only.
- `playing` remains the fallback base for any lane with no keyframes.
- Default palette endpoints remain white unless explicitly authored otherwise.
- Default lattice density continues to align to terminal row height when not explicitly authored.
