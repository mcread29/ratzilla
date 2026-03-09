# Implement `tty0` as a Curated Archive Workstation Example

## Summary

Turn `examples/tty0` from its current boot-screen prototype into the archive browser described in the docs: boot flow first, archive workstation by default, focused record view on demand, and a visible but locked terminal subsystem.

This stays entirely inside [`examples/tty0`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0). There are no root-crate API changes. Playback in v1 is metadata-only because the repo has no audio assets; the UI will expose a forward-compatible `audio_source` field and render an unavailable transport panel.

## Current Baseline

The docs in [`README.md`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/README.md), [`EXPERIENCE.md`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/EXPERIENCE.md), and [`TRACKS.md`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/TRACKS.md) clearly target a multi-state archive interface, but the code currently only has:
- one registered state in [`src/app.rs`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/src\/app.rs)
- a boot intro in [`src/introstate.rs`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/src\/introstate.rs)
- a generic string-based state machine in [`src/state/statemachine.rs`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/src\/state\/statemachine.rs)
- a temporary image test overlay in [`src/app.rs`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/src\/app.rs)

## Implementation

1. Refactor the example-local state system to support the four documented states.
   - Replace string state ids with an example-local `StateId` enum: `Intro`, `Archive`, `Record`, `Terminal`.
   - Simplify the example state machine API so states expose `on_enter`, `handle_key`, `update`, `render`, and `take_transition`.
   - Remove the current `can_exit_state` gating pattern; it is unnecessary for this example and is the reason `IntroState` cannot transition today.
   - Keep the state machine example-local under `examples/tty0/src`.

2. Add a typed archive data model and seed content from the markdown docs.
   - Create a new module such as `src/archive.rs` for typed seed data.
   - Add `CategoryId`, `RecordKind`, `ArchiveCategory`, `ArchiveRecord`, `RecordSection`, and `PlaybackAvailability`.
   - Seed categories from the docs: `incidents`, `witnesses`, `transmissions`, `collapse_vectors`, `signal_residue`, `tty0/private`, `unauthorized_tools`.
   - Seed full records for `0x01EPERM` and `0x07E2BIG` from [`TRACKS.md`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/TRACKS.md).
   - Add one lightweight placeholder/stub record for each otherwise-empty category so the left pane is never dead.
   - Store all content as Rust constants/structs. Do not parse markdown at runtime.

3. Introduce a shared session model used by all states.
   - Add a `SessionModel` containing selected category index, selected record index per category, current detail scroll offset, decryption status text, terminal lock status, and low-frequency corruption tick state.
   - Share it across states with `Rc<RefCell<SessionModel>>`.
   - Preserve archive selection when moving `Archive -> Record -> Archive`.

4. Rework the intro into the documented boot handoff.
   - Keep the existing line-by-line boot effect as the base.
   - Rewrite the final lines/copy so they align with the lore: identity mismatch, unauthorized session takeover, archive mounted, decryption stalled at `0%`.
   - Interaction rule:
     - If boot is still animating, first key skips to the end.
     - After boot is complete, next key transitions to `Archive`.
   - Keep the CRT/post-processing effect already present.

5. Build `ArchiveState` as the default workstation screen.
   - Layout:
     - Left pane split vertically into categories list and records list.
     - Center pane shows dossier/preview for the selected record.
     - Right pane shows metadata, signal integrity, timeline, corruption status, playback panel, and terminal lock card.
     - Bottom status bar shows key hints and subsystem status.
   - Keys:
     - `Left/Right`: switch category
     - `Up/Down`: move within records of the active category
     - `Enter`: open selected record if not locked
     - `t`: open locked terminal screen
   - Empty/stub records render as “recovered index only” states rather than errors.

6. Build `RecordState` as the focused evidence view.
   - Layout:
     - Top header with breadcrumb, error code/title, and timeline.
     - Main body split 70/30.
     - Left side: synopsis, recovered excerpt, context notes, tty0 annotation.
     - Right side: metadata block using the exact template from [`TRACKS.md`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/TRACKS.md), plus transport/status cards.
     - Bottom status bar with return/help copy.
   - Keys:
     - `Esc` or `Backspace`: return to `Archive`
     - `Up/Down`: scroll long content
     - `t`: open locked terminal screen
   - Reset detail scroll to `0` when entering a different record.

7. Build `TerminalState` as a deliberate locked subsystem, not a shell.
   - Render a full-screen denial panel with subsystem id, lock reason, and one hint line tied to the current record when available.
   - Copy should match the docs: visible early, clearly restricted, no fake prompt.
   - Keys: `Esc`, `Enter`, or `Backspace` return to the previous archive/record state.
   - Do not implement unlock logic in this pass.

8. Remove prototype-only UI that conflicts with the new direction.
   - Delete the temporary image test panel in [`src/app.rs`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/src\/app.rs).
   - Remove `CanvasImageLayer` plumbing unless a real tty0-specific image widget remains after implementation.
   - Keep the existing WebGL post-processing hook unless it causes layout/readability issues.

9. Keep the visual language stable and readable, with restrained corruption.
   - Use crisp pane borders and archive-language labels from [`EXPERIENCE.md`](\/home\/mchan\/dev\/ratzilla_real\/examples\/tty0\/EXPERIENCE.md).
   - Corruption should be small and intermittent: checksum warnings, occasional duplicated/redacted line fragments, flickering status text.
   - Do not add constant jitter or aggressive shader distortion that harms navigation.

## Important Interfaces and Types

No public `ratzilla` library API changes.

New example-local interfaces/types to add:
- `StateId`
- `SessionModel`
- `CategoryId`
- `RecordKind`
- `ArchiveCategory`
- `ArchiveRecord`
- `RecordSection`
- `PlaybackAvailability` with `Unavailable` in v1 and `audio_source: Option<&'static str>` for future real playback

## Test Cases and Acceptance Criteria

Run:
- `cargo check -p tty0 --manifest-path examples/Cargo.toml`
- `cd examples/tty0 && trunk build`

Manual scenarios:
- Boot animates, can be skipped, and transitions into archive cleanly.
- Archive opens by default after boot and never shows a fake shell prompt.
- Category and record navigation update the preview and metadata panes correctly.
- `Enter` opens detail view for real records and does nothing unsafe for locked/stub records.
- `Esc` from detail returns to the same category/record selection.
- `t` always opens the locked terminal screen and returns cleanly.
- Playback panel always renders a disabled/unavailable state and never implies working audio.
- Empty or placeholder categories render informative copy instead of blank panels or panics.
- Small viewport still shows a usable reduced layout; large viewport preserves the 3-pane structure.

## Assumptions and Defaults

- Scope is example-only inside `examples/tty0`.
- Playback is metadata-only in v1 because there are no audio files in the repo.
- The markdown docs are source material for seeded Rust data, not runtime content.
- The terminal remains locked in this implementation; no unlock path, no shell, no hidden commands.
- Existing post-processing stays unless it materially hurts readability.
