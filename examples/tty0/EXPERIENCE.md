# tty0 Experience Plan

## Product stance

The first public version should not open on a fake command prompt. It should open on a `recovered analysis workstation`.

That keeps the TUI identity while avoiding the weakest version of terminal nostalgia. The user is not roleplaying as a shell user yet. They are operating inside an unauthorized archive reader.

## Interface model

The UI should feel like a curated record browser built on top of a filesystem the user cannot fully access.

Recommended pane layout:

- Left pane: one archive index of recovered records
- Center pane: selected record header, page tabs, and active record content
- Right pane: metadata, signal integrity, corruption, timeline ID, playback state
- Bottom status bar: navigation help, subsystem state, and subtle warnings

## Screen flow

1. Boot sequence
2. Identity mismatch and session takeover
3. Auto-mount archive
4. Open record browser by default
5. Permit record navigation and playback
6. Show terminal subsystem as present but locked
7. Unlock deeper access later through hidden conditions

This lets the terminal exist as a promise before it becomes a feature.

## State ideas for the example

The current example already has a boot-like intro and a state machine. A clean near-term structure would be:

- `IntroState`: boot logs, decrypt progress, press-any-key handoff
- `ArchiveState`: primary record-first workstation for browsing incidents and tracks
- `TerminalState`: hidden or locked state for later easter eggs

## Interaction language

Prefer wording that suggests recovery and interpretation rather than ordinary file browsing:

- `lock signal`
- `open record`
- `recover transcript`
- `mount archive`
- `trace residue`
- `inspect collapse vector`
- `terminal access denied`

## Visual behavior

- The interface should be stable enough to read, but never fully trustworthy.
- Small corruption events are stronger than constant chaos.
- Use drift, checksum failures, or partial redraw artifacts to imply the archive is larger than the viewport.
- Keep the actual navigation crisp so the fiction does not interfere with use.

## Audio presentation

Tracks should appear as evidence objects, not as a normal album list.

Each record should feel like it contains:

- a recovered audio artifact
- contextual metadata
- one human-facing interpretation
- one tty0-facing annotation

The music player can therefore live inside the right pane as an instrument panel rather than as a consumer media widget.

## Hidden terminal plan

The terminal should arrive later as a restricted subsystem.

Best approach:

- show it early in the UI as `LOCKED`
- attach small hints to certain records
- allow a later unlock path through pattern recognition, not random guessing
- keep the shell sparse and rewarding, with lore fragments and one or two meaningful tools

The terminal should feel like the user crossed a boundary, not like they clicked a tab.
