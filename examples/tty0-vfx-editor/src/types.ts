export type TrackVisualizerMode =
  | "diplomatic_signal_bloom"
  | "hex_walker_relay"
  | "containment_lattice"
  | "chromatic_bulge_grid";

export type InterpolationMode = "hold" | "linear";
export type ClipTweenEase =
  | "hold"
  | "linear"
  | "sine_in"
  | "sine_out"
  | "sine_in_out";

export type ClipTweenValue =
  | { kind: "float"; value: number }
  | { kind: "color"; value: [number, number, number] };

export interface FloatKeyframe {
  beat: number;
  value: number;
  interpolation: InterpolationMode;
}

export interface ColorKeyframe {
  beat: number;
  value: [number, number, number];
  interpolation: InterpolationMode;
}

export interface ChromaticBulgeGridAutomationLanes {
  motion_rate?: FloatKeyframe[];
  lattice_density?: FloatKeyframe[];
  circle_radius?: FloatKeyframe[];
  circle_falloff_start?: FloatKeyframe[];
  circle_falloff_end?: FloatKeyframe[];
  bulge_amount?: FloatKeyframe[];
  rim_guard?: FloatKeyframe[];
  rim_exponent?: FloatKeyframe[];
  rim_warp?: FloatKeyframe[];
  spacing_max_px?: FloatKeyframe[];
  spacing_min_px?: FloatKeyframe[];
  dot_size?: FloatKeyframe[];
  outer_dot_scale?: FloatKeyframe[];
  edge_softness?: FloatKeyframe[];
  chromatic_aberration?: FloatKeyframe[];
  scroll_base?: FloatKeyframe[];
  scroll_motion_scale?: FloatKeyframe[];
  scroll_motion_floor?: FloatKeyframe[];
  scroll_motion_ceiling?: FloatKeyframe[];
  cold_color?: ColorKeyframe[];
  hot_color?: ColorKeyframe[];
  color_cycle_rate?: FloatKeyframe[];
  inner_alpha?: FloatKeyframe[];
}

export interface ChromaticBulgeGridShaderState {
  motion_rate: number;
  lattice_density: number;
  circle_radius: number;
  circle_falloff_start: number;
  circle_falloff_end: number;
  bulge_amount: number;
  rim_guard: number;
  rim_exponent: number;
  rim_warp: number;
  spacing_max_px: number;
  spacing_min_px: number;
  dot_size: number;
  outer_dot_scale: number;
  edge_softness: number;
  chromatic_aberration: number;
  scroll_base: number;
  scroll_motion_scale: number;
  scroll_motion_floor: number;
  scroll_motion_ceiling: number;
  cold_color: [number, number, number];
  hot_color: [number, number, number];
  color_cycle_rate: number;
  inner_alpha: number;
}

export interface TrackVisualizerParams {
  motion_rate: number;
  energy_gain: number;
  bass_gain: number;
  mid_gain: number;
  treble_gain: number;
  ring_count: number;
  particle_count: number;
  lattice_density: number;
  shader_states?: {
    playing: ChromaticBulgeGridShaderState;
    idle: ChromaticBulgeGridShaderState;
  };
}

export interface ChromaticBulgeGridAutomation {
  bpm: number;
  measures: number;
  beats_per_measure: number;
  lanes: ChromaticBulgeGridAutomationLanes;
}

export interface ClipParamTrack {
  lane: LaneId;
  steps: ClipTweenStep[];
}

export interface ChromaticBulgeGridClipAuthoring {
  tracks: ClipParamTrack[];
}

export interface ClipTweenStep {
  to: ClipTweenValue;
  duration_beats: number;
  ease: ClipTweenEase;
}

export interface ChromaticBulgeGridClip {
  id: string;
  name: string;
  length_beats: number;
  color: [number, number, number];
  authoring?: ChromaticBulgeGridClipAuthoring;
  lanes: ChromaticBulgeGridAutomationLanes;
}

export interface ClipPlacement {
  clip_id: string;
  start_beat: number;
  track: number;
  repeats: number;
}

export interface ChromaticBulgeGridClipTimeline {
  bpm: number;
  measures: number;
  beats_per_measure: number;
  clips: ChromaticBulgeGridClip[];
  arrangement: ClipPlacement[];
}

export interface TrackVisualizerConfig {
  mode: TrackVisualizerMode;
  params: TrackVisualizerParams;
  automation?: ChromaticBulgeGridAutomation | null;
  timeline?: ChromaticBulgeGridClipTimeline | null;
}

export interface RecordSummary {
  record_id: string;
  title: string;
  record_path: string;
  has_audio: boolean;
}

export interface LoadedRecord {
  record_id: string;
  record_path: string;
  record_title: string;
  audio_url?: string | null;
  visualizer: TrackVisualizerConfig;
  raw_record_json: string;
  has_legacy_automation: boolean;
}

export interface ProjectHandle {
  root: string;
  manifest_path: string;
}

export type LaneId =
  | "motion_rate"
  | "lattice_density"
  | "circle_radius"
  | "circle_falloff_start"
  | "circle_falloff_end"
  | "bulge_amount"
  | "rim_guard"
  | "rim_exponent"
  | "rim_warp"
  | "spacing_max_px"
  | "spacing_min_px"
  | "dot_size"
  | "outer_dot_scale"
  | "edge_softness"
  | "chromatic_aberration"
  | "scroll_base"
  | "scroll_motion_scale"
  | "scroll_motion_floor"
  | "scroll_motion_ceiling"
  | "cold_color"
  | "hot_color"
  | "color_cycle_rate"
  | "inner_alpha";
