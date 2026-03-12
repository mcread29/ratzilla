export const TRACK_HEIGHT = 28;
export const TIMELINE_TOP_SCROLLBAR_HEIGHT = 14;
export const RULER_HEIGHT = TIMELINE_TOP_SCROLLBAR_HEIGHT * 1.5;
export const TIMELINE_SNAP_DIVISION = 4;
export const TIMELINE_SNAP_THRESHOLD_PX = 12;
export const PLAYHEAD_SNAP_DIVISION = 1;
export const TIMELINE_LABEL_WIDTH = 160;
export const DEFAULT_TIMELINE_VIEWPORT_WIDTH = 960;
export const MAX_VISIBLE_MEASURES_AT_MAX_ZOOM = 16;
export const SHAPE_EDITOR_WIDTH = 320;
export const SHAPE_EDITOR_HEIGHT = 180;
export const SHAPE_EDITOR_VERTICAL_PADDING = 10;
export const SHAPE_EDITOR_BOUND_INSET = 2;
export const SHAPE_EDITOR_HANDLE_INSET = 6;
export const LFO_POINT_SNAP = 0.025;

export const CLIP_LENGTH_BAR_OPTIONS = [
  { label: "4 bars", bars: 4 },
  { label: "3 bars", bars: 3 },
  { label: "2 bars", bars: 2 },
  { label: "1 bar", bars: 1 },
  { label: "1/2 bar", bars: 0.5 },
  { label: "1/4 bar", bars: 0.25 },
  { label: "1/8 bar", bars: 0.125 },
] as const;
