# Clip Sequencer Editor For `chromatic_bulge_grid`

## Summary
Replace the current raw per-lane timeline authoring flow in [visualizer_editor.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/visualizer_editor.rs) with a clip-based sequencer for `chromatic_bulge_grid`.

The result will be:
- left pane: clip library
- right top: arrangement timeline
- right bottom: clip editor
- existing `State` tab remains for base shader values
- `Export` tab remains for JSON/save
- the current `Timeline` tab becomes `Clips`

This plan keeps runtime compatibility with existing records by supporting both legacy lane automation and the new clip timeline schema. The editor will migrate legacy automation into an in-memory clip timeline on open, and saving from the new editor will write the new clip timeline schema only.

## Scope
In scope:
- new data model for reusable clips plus arrangement placements
- runtime resolution of clip placements into shader state
- new clip sequencer UI and interaction model
- editor draft migration from legacy automation to clip timeline
- save/export through the existing bridge
- tests for parsing, validation, resolver behavior, migration, and key editor flows

Out of scope:
- multi-track layered arrangement in v1
- arbitrary overlap/blend modes between placements
- non-`chromatic_bulge_grid` visualizers
- mouse interaction
- file format migration tool outside the editor/save flow

## Chosen Product Decisions
- There is one global arrangement lane in v1. Placements cannot overlap in time.
- A clip can automate any subset of existing shader lanes.
- A placement references one clip and can repeat it contiguously with a `repeats` count.
- Base shader values remain the default visual state and stay unified for playing/idle editing.
- The editor preview should show authored clip output at the current playhead even while paused, so scrubbing and arrangement edits are visible without pressing play.
- Saving from the new clip editor writes canonical clip-timeline JSON and omits legacy `automation`.
- Existing records with only legacy `automation` are imported into one generated clip plus one placement, preserving behavior exactly.

## Public API / Schema Changes
Add a new optional field to [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs):

```rust
pub struct TrackVisualizerConfig {
    pub mode: TrackVisualizerMode,
    #[serde(default)]
    pub params: TrackVisualizerParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<ChromaticBulgeGridAutomation>, // legacy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<ChromaticBulgeGridClipTimeline>, // new canonical schema
}
```

Add new types in [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs):

```rust
pub struct ChromaticBulgeGridClipTimeline {
    pub bpm: f32,
    pub measures: u32,
    #[serde(default = "default_beats_per_measure")]
    pub beats_per_measure: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<ChromaticBulgeGridClip>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrangement: Vec<ClipPlacement>,
}

pub struct ChromaticBulgeGridClip {
    pub id: String,
    pub name: String,
    pub length_beats: f32,
    #[serde(default = "default_clip_color")]
    pub color: [f32; 3],
    #[serde(default)]
    pub lanes: ChromaticBulgeGridAutomationLanes,
}

pub struct ClipPlacement {
    pub clip_id: String,
    pub start_beat: f32,
    #[serde(default = "default_repeat_count")]
    pub repeats: u32,
}
```

Compatibility rules:
- `automation` and `timeline` may not both be present in saved JSON. Validation rejects that combination.
- Resolver priority is `timeline` first, then legacy `automation`, then base state only.
- `normalized_for_export()` becomes canonical:
  - if `timeline` exists, normalize and serialize `timeline`, set `automation = None`
  - if only `automation` exists, keep current behavior unchanged
- `TrackVisualizerConfig::resolve_chromatic_bulge_grid()` must support both representations

## Runtime Behavior
For timeline-based configs:
1. Compute `current_beat` from `playback.current_time_secs * bpm / 60.0`.
2. Resolve base shader state from `params.shader_states`.
3. If preview/runtime is in timeline-preview mode, apply clip placements at `current_beat` even when transport is paused.
4. Otherwise, keep current archive behavior outside the editor: paused archive view uses base state only.
5. For each placement whose active span contains `current_beat`, compute `local_beat = (current_beat - start_beat) % clip.length_beats`.
6. Sample the clip’s lanes against base state using the existing `FloatKeyframe` and `ColorKeyframe` semantics.
7. Because v1 has a single arrangement lane and non-overlap validation, there is at most one active placement at any beat.

Placement duration:
- `placement_end = start_beat + clip.length_beats * repeats`

Sampling semantics:
- local beats are clamped to `[0, clip.length_beats]`
- keyframes preserve existing `Hold` and `Linear` interpolation
- empty clip lanes fall back to base shader values

## Validation Rules
Extend validator in [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs):

For `timeline`:
- `bpm` finite and `> 0`
- `measures >= 1`
- `beats_per_measure >= 1`
- clip ids unique and non-empty
- clip names non-empty
- `length_beats` finite and `> 0`
- placement `clip_id` must exist
- placement `start_beat` finite and `>= 0`
- `repeats >= 1`
- each clip lane must have no duplicate local beats
- every local beat in a clip must satisfy `0.0 <= beat <= length_beats`
- each placement end must be `<= total_beats`
- placements may not overlap in time on the single arrangement lane

For dual-schema ambiguity:
- reject configs that contain both non-empty `automation` and non-empty `timeline`

## Editor Draft Model
Refactor [visualizer_editor.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/visualizer_editor.rs) draft state to support a clip timeline draft.

Add editor-only draft types:
- `ClipEditorDraft`
- `SelectedClipId`
- `SelectedPlacementIndex`
- `clip_cursor_beat: f32`
- `preview_mode: EditorPreviewMode`

Use this migration path on open:
- if `config.timeline` exists, use it directly
- else if `config.automation` exists, convert it into:
  - one clip:
    - `id = "imported_timeline"`
    - `name = "Imported Timeline"`
    - `length_beats = automation.total_beats()`
    - `lanes = automation.lanes.clone()`
  - one placement:
    - `clip_id = "imported_timeline"`
    - `start_beat = 0.0`
    - `repeats = 1`
- else create an empty default timeline with:
  - bpm/measures/beats_per_measure from current defaults
  - one clip named `Clip 1`
  - one empty arrangement

This migration is editor-only until save.

## UI Layout
Keep the outer editor frame and tabs. Change tabs to:
- `State`
- `Clips`
- `Export`

Inside `Clips`:
- left pane width: 28 columns minimum, stretch to 30 if space allows
- right pane: vertical split
- right top height: 11 rows minimum, 40% of right pane
- right bottom: remaining space

Left pane: clip library
- one card per clip
- each card shows:
  - clip color swatch
  - clip name
  - clip length in beats/measures
  - placement count
  - automated lane count
  - one tiny preview sparkline row
- selected clip card uses cyan border
- focused library pane uses cyan border on the block

Right top: arrangement
- horizontal beat grid with measure markers and beat subdivisions
- one single arrangement row of colored clip blocks
- current global playhead vertical marker
- selected placement inverse-highlighted
- repeated placements render as repeated adjacent sub-blocks within the same card color
- top status line shows:
  - bpm
  - measures
  - beats per measure
  - current beat
  - selected placement span

Right bottom: clip editor
- internal horizontal split:
  - left: lane list
  - center: local clip graph
  - right: inspector
- lane list reuses current lane labels and key counts, but key counts are clip-local
- graph is local to the selected clip from beat `0` to `clip.length_beats`
- graph always shows local grid lines at quarter-beat resolution
- graph shows `clip_cursor_beat` as the main edit cursor
- if the selected clip also has a selected placement and the global playhead is inside that placement, show a secondary ghost marker for the live local playback position
- inspector shows:
  - clip metadata: `length_beats`
  - lane base/default value
  - selected keyframe fields
  - sampled value at `clip_cursor_beat`

## Interaction Model
Global:
- `E` closes editor
- `Tab` cycles focus in this order:
  - clip library
  - arrangement
  - clip lanes
  - clip graph
  - clip inspector
- `Shift+Tab` cycles backward if already supported by the event layer; otherwise keep forward-only `Tab`
- `P` play/pause transport
- `S` save via existing bridge
- `[` / `]` moves global playhead by one measure
- `Left` / `Right` outside clip graph move global playhead by `0.25` beat

Clip library focus:
- `Up` / `Down` select previous/next clip
- `A` create new empty clip
- `C` duplicate selected clip
- `R` rename selected clip and enter text mode
- `Delete` / `D` delete selected clip if unused
- deleting a clip with placements is blocked with status message
- `Enter` jumps focus to clip graph

Arrangement focus:
- `Up` / `Down` select previous/next placement
- `A` add placement of selected clip at current global playhead
- `G` move selected placement start to current global playhead if no overlap is created
- `,` / `.` select previous/next placement
- `-` / `=` decrement/increment `repeats`
- `Delete` / `D` delete placement
- `Enter` selects the placement under the playhead if one exists
- if no placement exists yet, arrangement shows an empty grid with helper text

Clip lanes focus:
- `Up` / `Down` select lane
- `Enter` jumps to clip graph
- lane changes preserve current `clip_cursor_beat`

Clip graph focus:
- `Left` / `Right` move `clip_cursor_beat` by `0.25`
- `Shift+Left` / `Shift+Right` move `clip_cursor_beat` by `1.0`
- `Home` sets `clip_cursor_beat = 0`
- `End` sets `clip_cursor_beat = clip.length_beats`
- `N` inserts sampled keyframe at `clip_cursor_beat`
- `Y` clones selected keyframe to `clip_cursor_beat`
- `G` moves selected keyframe to `clip_cursor_beat`
- `,` / `.` select previous/next keyframe in the lane
- `D` / `Delete` removes selected keyframe
- `I` toggles interpolation
- `Enter` edits the selected numeric field exactly as today

Clip inspector focus:
- `Up` / `Down` moves between inspector fields
- `-` / `=` nudges selected field
- `_` / `+` coarse nudge
- `Enter` opens numeric input for selected field
- for color lanes, inspector exposes `R/G/B`
- for float lanes, inspector exposes `beat`, `value`, `interpolation`

Numeric editing rules:
- reuse existing numeric edit flow
- arrangement `repeats`, clip `length_beats`, timeline `bpm`, `measures`, and `beats_per_measure` all use the same numeric input widget
- clip rename uses a simple line-input mode distinct from numeric input

## Visual Preview Rules
Clip library card preview:
- generate a tiny sparkline from 16 local samples across the selected “headline lane”
- choose headline lane by priority:
  - `circle_radius`
  - `scroll_base`
  - `motion_rate`
  - first non-empty lane
- if only color lanes are used, render a color strip instead of numeric sparkline

Arrangement preview:
- each placement block uses the clip’s saved color
- measure boundaries use brighter guide columns
- the currently selected placement displays:
  - name
  - repeats
  - total span
- if space is too tight, labels collapse to clip initials

Clip graph preview:
- reuse existing timeline graph logic but convert it to local-beat coordinates
- render quarter-beat tick marks
- render selected keyframe with `>` marker plus highlighted point
- render the clip cursor line distinctly from live local playback ghost line

## Save / Export Behavior
When saving/exporting from the clip editor:
- export canonical `timeline`
- set `automation = None`
- keep `mode`, `params`, and `shader_states`
- keep base playing/idle states synced exactly as the current editor does
- preserve pretty-printed JSON formatting through `normalized_for_export()`

If the user opened a legacy automation-only record and saves without making changes:
- the file still converts to timeline schema
- this is intentional and should be documented in the status message once per session:
  - `save will migrate legacy lane automation to clip timeline format`

## Implementation Steps
1. Extend schema/types/serde in [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs).
2. Add validation and normalization for `timeline`.
3. Add clip-timeline resolver path to `TrackVisualizerConfig::resolve_chromatic_bulge_grid()`.
4. Add conversion helper:
   - `legacy_automation_to_timeline(automation: &ChromaticBulgeGridAutomation) -> ChromaticBulgeGridClipTimeline`
5. Add clip-timeline sampling helpers that reuse current lane sampling logic.
6. Refactor editor draft state in [visualizer_editor.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/visualizer_editor.rs) to operate on `timeline`.
7. Replace `Timeline` tab rendering with the new 3-pane `Clips` layout.
8. Add clip library actions and rename mode.
9. Add arrangement rendering and placement editing.
10. Reuse existing lane graph/inspector components in the clip editor bottom pane with local-beat coordinates.
11. Add editor preview override so paused editor scrubbing still renders the selected authored state.
12. Keep `Export` tab and save bridge integration unchanged except for canonical timeline export.
13. Update footer/help text for the new focus areas and keys.

## Tests And Scenarios
Parser/validation tests in [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs):
- parses valid `timeline`
- rejects duplicate clip ids
- rejects placement referencing missing clip id
- rejects clip keyframes outside clip `length_beats`
- rejects duplicate local beats in a clip lane
- rejects overlapping placements
- rejects placement extending beyond total timeline beats
- rejects config containing both `automation` and `timeline`

Resolver tests in [archive.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/archive.rs):
- base state is returned before the first active placement
- a single placement samples local clip beats correctly
- repeated placement wraps local beat correctly
- imported legacy automation timeline matches legacy `automation` output at representative beats
- paused non-editor path still returns base idle state
- editor-preview path applies clip timeline while paused

Editor tests in [visualizer_editor.rs](/home/mchan/dev/ratzilla_real/examples/tty0/src/visualizer_editor.rs):
- opening a legacy automation record creates one imported clip and one placement in draft state
- adding a placement at playhead creates the correct `start_beat`
- increasing `repeats` updates total span and blocks overlap if invalid
- selecting a clip updates the bottom pane data
- deleting an in-use clip is rejected
- clip rename updates placement labels but preserves `clip_id`
- save/export clears dirty state against canonical timeline payload
- preview config emitted by the editor contains `timeline` and no `automation`

UI acceptance scenarios:
- user can author a one-beat radius pulse clip once and place it across the song with repeated placements instead of 256 individual keyframes
- user can see colored clip blocks in arrangement and card previews in the library
- user can scrub while paused and the right-side preview updates visibly
- user can duplicate a clip, tweak it, and swap placements without reauthoring all keyframes

## Assumptions And Defaults
- v1 uses one arrangement lane only; no overlapping placements and no layer blending
- clip `repeats` are contiguous with no gap parameter
- clip `length_beats` may be fractional but must align to finite positive `f32`
- clip colors are editor metadata and are stored in JSON for stable visuals
- `clip_id` is stable and never auto-renames after creation
- renaming a clip changes `name` only
- current `FloatKeyframe` and `ColorKeyframe` types are reused unchanged
- existing save bridge endpoint remains `http://127.0.0.1:4777/save_visualizer`
- current base-state unification and visual idle clock behavior remain as already implemented
