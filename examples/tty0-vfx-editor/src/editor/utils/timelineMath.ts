import { PLAYHEAD_SNAP_DIVISION, TIMELINE_SNAP_DIVISION } from "../constants";

export function beatToPx(beat: number, timelineZoom: number): number {
  return Math.max(0, beat) * timelineZoom;
}

export function pxToBeat(px: number, timelineZoom: number): number {
  return Math.max(0, px / timelineZoom);
}

export function clampBeat(beat: number, totalTimelineBeats: number): number {
  return Math.max(0, Math.min(totalTimelineBeats, beat));
}

export function snapBeatToGrid(beat: number, totalTimelineBeats: number): number {
  return Math.round(clampBeat(beat, totalTimelineBeats) * TIMELINE_SNAP_DIVISION) / TIMELINE_SNAP_DIVISION;
}

export function snapPlaybackBeat(beat: number, totalTimelineBeats: number): number {
  return Math.round(clampBeat(beat, totalTimelineBeats) * PLAYHEAD_SNAP_DIVISION) / PLAYHEAD_SNAP_DIVISION;
}
