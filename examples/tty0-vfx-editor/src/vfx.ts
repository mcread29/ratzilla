import {
  ChromaticBulgeGridAutomation,
  ChromaticBulgeGridAutomationLanes,
  ChromaticBulgeGridClip,
  ChromaticBulgeGridClipSource,
  ChromaticBulgeGridClipTimeline,
  ChromaticBulgeGridLfoClip,
  ChromaticBulgeGridLfoShape,
  ChromaticBulgeGridShaderState,
  ClipPlacement,
  ClipTweenStep,
  ColorKeyframe,
  FloatKeyframe,
  LaneId,
  LfoPoint,
  TrackVisualizerConfig,
} from "./types";

export type IndexedTimelinePlacement = {
  clip: ChromaticBulgeGridClip;
  endBeat: number;
  placement: ClipPlacement;
  placementIndex: number;
};

export type TimelineIndex = {
  clipsById: Map<string, ChromaticBulgeGridClip>;
  placements: IndexedTimelinePlacement[];
  placementsByTrack: Map<number, IndexedTimelinePlacement[]>;
};

export type ShapeSegment = {
  index: number;
  left: LfoPoint;
  right: LfoPoint;
  midpointX: number;
  midpointY: number;
  controlX: number;
  controlY: number;
};

const SHARED_LFO_IMPORT_ERROR =
  "Import failed: shared LFO shape libraries are no longer supported. Each LFO clip must embed its own shape.";

export const LANE_ORDER: LaneId[] = [
  "motion_rate",
  "motion_rate_y",
  "lattice_density",
  "circle_radius",
  "circle_falloff_start",
  "circle_falloff_end",
  "bulge_amount",
  "rim_guard",
  "rim_exponent",
  "rim_warp",
  "dot_size",
  "outer_dot_scale",
  "edge_softness",
  "chromatic_aberration",
  "cold_color",
  "hot_color",
  "color_cycle_rate",
  "inner_alpha",
];

const COLOR_LANES = new Set<LaneId>(["cold_color", "hot_color"]);
const LFO_LANES = LANE_ORDER.filter((lane) => !COLOR_LANES.has(lane));

export function isColorLane(lane: LaneId): boolean {
  return COLOR_LANES.has(lane);
}

export function supportsLfo(lane: LaneId): boolean {
  return !isColorLane(lane);
}

export function defaultShaderState(): ChromaticBulgeGridShaderState {
  return {
    motion_rate: 1,
    motion_rate_y: 0,
    lattice_density: 6,
    circle_radius: 0.24,
    circle_falloff_start: 0.78,
    circle_falloff_end: 1,
    bulge_amount: 0.42,
    rim_guard: 0.55,
    rim_exponent: 1.8,
    rim_warp: 0.18,
    dot_size: 0.16,
    outer_dot_scale: 0.33,
    edge_softness: 1,
    chromatic_aberration: 0.28,
    cold_color: [1, 1, 1],
    hot_color: [1, 1, 1],
    color_cycle_rate: 0.16,
    inner_alpha: 0.9,
  };
}

export function defaultClipLfoShape(): ChromaticBulgeGridLfoShape {
  return {
    interpolation: "linear",
    points: [point(0, 0), point(1, 0)],
  };
}

export function defaultLfoClip(
  lane: LaneId,
  base: ChromaticBulgeGridShaderState,
): ChromaticBulgeGridClipSource {
  const baseValue = Number(base[lane as keyof ChromaticBulgeGridShaderState] ?? 0);
  const span = laneDefaultSpan(lane);
  return {
    kind: "lfo",
    lane,
    shape: defaultClipLfoShape(),
    min: baseValue,
    max: baseValue + span,
    period_beats: 4,
    phase_offset_beats: 0,
    start_mode: "retrigger",
  };
}

export function defaultVisualizer(): TrackVisualizerConfig {
  const base = defaultShaderState();
  return {
    mode: "chromatic_bulge_grid",
    params: {
      motion_rate: 1,
      energy_gain: 1.1,
      bass_gain: 1.25,
      mid_gain: 1,
      treble_gain: 1.1,
      ring_count: 4,
      particle_count: 48,
      lattice_density: 6,
      shader_states: {
        playing: base,
        idle: base,
      },
    },
    timeline: {
      bpm: 132,
      measures: 64,
      beats_per_measure: 4,
      clips: [
        {
          id: "clip_1",
          name: "Clip 1",
          length_beats: 4,
          color: [0.43, 0.86, 0.83],
          source: defaultLfoClip("motion_rate", base),
          lanes: {},
        },
      ],
      arrangement: [],
    },
  };
}

export function assertSupportedClipLocalShapeSchema(config: unknown): void {
  if (!config || typeof config !== "object") {
    return;
  }
  const visualizer = config as {
    lfo_library?: unknown;
    timeline?: {
      clips?: Array<{ source?: { kind?: string; shape_id?: unknown; shape?: unknown } | null }> | null;
    } | null;
  };
  if ("lfo_library" in visualizer) {
    throw new Error(SHARED_LFO_IMPORT_ERROR);
  }
  const clips = visualizer.timeline?.clips;
  if (!Array.isArray(clips)) {
    return;
  }
  for (const clip of clips) {
    const source = clip?.source;
    if (!source || source.kind !== "lfo") {
      continue;
    }
    if ("shape_id" in source || !source.shape || typeof source.shape !== "object") {
      throw new Error(SHARED_LFO_IMPORT_ERROR);
    }
  }
}

export function normalizedConfig(config: TrackVisualizerConfig): TrackVisualizerConfig {
  const next = structuredClone(config);
  if (!next.params.shader_states) {
    const base = defaultShaderState();
    next.params.shader_states = { playing: base, idle: base };
  }
  next.params.shader_states.playing = normalizeShaderState(next.params.shader_states.playing);
  next.params.shader_states.idle = structuredClone(next.params.shader_states.playing);
  if (next.timeline) {
    next.automation = null;
    next.timeline = normalizeTimeline(next.timeline, next.params.shader_states.playing);
  } else if (next.automation) {
    next.timeline = legacyAutomationToTimeline(next.automation);
    next.automation = null;
  } else {
    next.timeline = defaultVisualizer().timeline!;
  }
  return next;
}

export function normalizeTimeline(
  timeline: ChromaticBulgeGridClipTimeline,
  base: ChromaticBulgeGridShaderState,
): ChromaticBulgeGridClipTimeline {
  const next = structuredClone(timeline);
  next.bpm = Math.max(1, Number.isFinite(next.bpm) ? next.bpm : 120);
  next.measures = Math.max(1, Math.round(next.measures || 1));
  next.beats_per_measure = Math.max(1, Math.round(next.beats_per_measure || 4));
  next.clips = (next.clips ?? []).map((clip, index) => normalizeClip(clip, index, base));
  next.arrangement = (next.arrangement ?? []).map((placement) => ({
    clip_id: placement.clip_id,
    start_beat: Math.max(0, placement.start_beat || 0),
    track: Math.max(0, Math.round(placement.track || 0)),
    repeats: Math.max(1, Math.round(placement.repeats || 1)),
  }));
  return next;
}

function normalizeClip(
  clip: ChromaticBulgeGridClip,
  index: number,
  base: ChromaticBulgeGridShaderState,
): ChromaticBulgeGridClip {
  const lane = primaryLane(clip)?.lane ?? LFO_LANES[0];
  const source = normalizeClipSource(clip.source, lane, base);
  return {
    ...clip,
    id: clip.id?.trim() || `clip_${index + 1}`,
    name: clip.name?.trim() || `Clip ${index + 1}`,
    length_beats: Math.max(0.25, clip.length_beats || 4),
    color: clampColor(clip.color ?? palette(index)),
    source,
    lanes: clip.lanes ?? {},
  };
}

function normalizeClipSource(
  source: ChromaticBulgeGridClipSource | undefined,
  lane: LaneId,
  base: ChromaticBulgeGridShaderState,
): ChromaticBulgeGridClipSource | undefined {
  if (!source || source.kind !== "lfo") {
    return defaultLfoClip(lane, base);
  }
  if (!source.shape) {
    throw new Error(SHARED_LFO_IMPORT_ERROR);
  }
  const fallback = defaultLfoClip(source.lane, base);
  return {
    kind: "lfo",
    lane: supportsLfo(source.lane) ? source.lane : fallback.lane,
    shape: normalizeClipLfoShape(source.shape),
    min: finiteOr(source.min, fallback.min),
    max: finiteOr(source.max, fallback.max),
    period_beats: Math.max(0.0001, finiteOr(source.period_beats, fallback.period_beats)),
    phase_offset_beats: finiteOr(source.phase_offset_beats, 0),
    start_mode: source.start_mode === "continue" ? "continue" : "retrigger",
  };
}

export function normalizeClipLfoShape(
  shape: ChromaticBulgeGridLfoShape | null | undefined,
): ChromaticBulgeGridLfoShape {
  if (!shape) {
    throw new Error(SHARED_LFO_IMPORT_ERROR);
  }
  const points = normalizeShapePoints(shape.points ?? []);
  if (!points.length) {
    throw new Error("Import failed: each LFO clip must include at least one embedded shape point.");
  }
  return {
    interpolation: "linear",
    points,
  };
}

export function resolveChromaticBulgeGrid(
  config: TrackVisualizerConfig,
  playback: {
    currentTimeSecs: number;
    visualTimeSecs: number;
    isPlaying: boolean;
    timelinePreview: boolean;
  },
): { uniforms: ChromaticBulgeGridShaderState; currentBeat: number } {
  return resolveNormalizedChromaticBulgeGrid(normalizedConfig(config), playback);
}

export function resolveNormalizedChromaticBulgeGrid(
  config: TrackVisualizerConfig,
  playback: {
    currentTimeSecs: number;
    visualTimeSecs: number;
    isPlaying: boolean;
    timelinePreview: boolean;
  },
  timelineIndex?: TimelineIndex,
): { uniforms: ChromaticBulgeGridShaderState; currentBeat: number } {
  const next = config;
  const states = next.params.shader_states!;
  const timeline = next.timeline!;
  const bpm = timeline.bpm || 120;
  const currentBeat = Math.max(0, playback.currentTimeSecs) * bpm / 60;
  const base = playback.isPlaying ? states.playing : states.idle;
  if (!(playback.isPlaying || playback.timelinePreview)) {
    return { uniforms: clampState(base), currentBeat };
  }
  const resolved = resolveTimelineState(timelineIndex ?? buildTimelineIndex(timeline), base, currentBeat);
  return { uniforms: clampState(resolved ?? base), currentBeat };
}

function resolveTimelineState(
  timelineIndex: TimelineIndex,
  base: ChromaticBulgeGridShaderState,
  beat: number,
): ChromaticBulgeGridShaderState | null {
  const active = heldClipsAtBeat(timelineIndex, beat);
  if (!active.length) return null;
  let state = structuredClone(base);
  for (const entry of active) {
    state = applyClipToState(state, entry.clip, entry.localBeat, beat);
  }
  return state;
}

function heldClipsAtBeat(timelineIndex: TimelineIndex, beat: number) {
  const held = new Map<number, { clip: ChromaticBulgeGridClip; localBeat: number; startBeat: number }>();
  for (const { clip, endBeat: end, placement } of timelineIndex.placements) {
    if (beat < placement.start_beat) continue;
    const localBeat =
      beat < end ? ((beat - placement.start_beat) % clip.length_beats + clip.length_beats) % clip.length_beats : clip.length_beats;
    const existing = held.get(placement.track);
    if (!existing || placement.start_beat >= existing.startBeat) {
      held.set(placement.track, { clip, localBeat, startBeat: placement.start_beat });
    }
  }
  return Array.from(held.values());
}

function applyClipToState(
  state: ChromaticBulgeGridShaderState,
  clip: ChromaticBulgeGridClip,
  localBeat: number,
  globalBeat: number,
): ChromaticBulgeGridShaderState {
  if (clip.source?.kind === "lfo") {
    return applyLfoToState(state, clip.source, localBeat, globalBeat);
  }
  return applyLegacyLanes(state, clip.lanes, localBeat);
}

function applyLfoToState(
  state: ChromaticBulgeGridShaderState,
  source: ChromaticBulgeGridLfoClip,
  localBeat: number,
  globalBeat: number,
): ChromaticBulgeGridShaderState {
  const phaseBeats =
    (source.start_mode === "continue" ? globalBeat : localBeat) + source.phase_offset_beats;
  const phase = (phaseBeats / Math.max(0.0001, source.period_beats)) % 1;
  const sample = sampleLfoShape(source.shape, phase);
  const value = source.min + (source.max - source.min) * sample;
  return applyLfoValueToState(state, source.lane, value);
}

function applyLegacyLanes(
  base: ChromaticBulgeGridShaderState,
  lanes: ChromaticBulgeGridAutomationLanes,
  beat: number,
): ChromaticBulgeGridShaderState {
  return {
    ...base,
    motion_rate: sampleFloatLane(lanes.motion_rate ?? [], beat, base.motion_rate),
    motion_rate_y: sampleFloatLane(lanes.motion_rate_y ?? [], beat, base.motion_rate_y),
    lattice_density: sampleFloatLane(lanes.lattice_density ?? [], beat, base.lattice_density),
    circle_radius: sampleFloatLane(lanes.circle_radius ?? [], beat, base.circle_radius),
    circle_falloff_start: sampleFloatLane(lanes.circle_falloff_start ?? [], beat, base.circle_falloff_start),
    circle_falloff_end: sampleFloatLane(lanes.circle_falloff_end ?? [], beat, base.circle_falloff_end),
    bulge_amount: sampleFloatLane(lanes.bulge_amount ?? [], beat, base.bulge_amount),
    rim_guard: sampleFloatLane(lanes.rim_guard ?? [], beat, base.rim_guard),
    rim_exponent: sampleFloatLane(lanes.rim_exponent ?? [], beat, base.rim_exponent),
    rim_warp: sampleFloatLane(lanes.rim_warp ?? [], beat, base.rim_warp),
    dot_size: sampleFloatLane(lanes.dot_size ?? [], beat, base.dot_size),
    outer_dot_scale: sampleFloatLane(lanes.outer_dot_scale ?? [], beat, base.outer_dot_scale),
    edge_softness: sampleFloatLane(lanes.edge_softness ?? [], beat, base.edge_softness),
    chromatic_aberration: sampleFloatLane(lanes.chromatic_aberration ?? [], beat, base.chromatic_aberration),
    cold_color: sampleColorLane(lanes.cold_color ?? [], beat, base.cold_color),
    hot_color: sampleColorLane(lanes.hot_color ?? [], beat, base.hot_color),
    color_cycle_rate: sampleFloatLane(lanes.color_cycle_rate ?? [], beat, base.color_cycle_rate),
    inner_alpha: sampleFloatLane(lanes.inner_alpha ?? [], beat, base.inner_alpha),
  };
}

function applyLfoValueToState(
  state: ChromaticBulgeGridShaderState,
  lane: LaneId,
  value: number,
): ChromaticBulgeGridShaderState {
  switch (lane) {
    case "motion_rate":
      return { ...state, motion_rate: value };
    case "motion_rate_y":
      return { ...state, motion_rate_y: value };
    case "lattice_density":
      return { ...state, lattice_density: value };
    case "circle_radius":
      return { ...state, circle_radius: value };
    case "circle_falloff_start":
      return { ...state, circle_falloff_start: value };
    case "circle_falloff_end":
      return { ...state, circle_falloff_end: value };
    case "bulge_amount":
      return { ...state, bulge_amount: value };
    case "rim_guard":
      return { ...state, rim_guard: value };
    case "rim_exponent":
      return { ...state, rim_exponent: value };
    case "rim_warp":
      return { ...state, rim_warp: value };
    case "dot_size":
      return { ...state, dot_size: value };
    case "outer_dot_scale":
      return { ...state, outer_dot_scale: value };
    case "edge_softness":
      return { ...state, edge_softness: value };
    case "chromatic_aberration":
      return { ...state, chromatic_aberration: value };
    case "color_cycle_rate":
      return { ...state, color_cycle_rate: value };
    case "inner_alpha":
      return { ...state, inner_alpha: value };
    case "cold_color":
    case "hot_color":
      return state;
  }
}

export function sampleLfoShape(shape: ChromaticBulgeGridLfoShape, phase: number): number {
  const points = normalizeShapePoints(shape.points);
  if (!points.length) return 0;
  if (points.length === 1) return points[0].value;
  const normalizedPhase = ((phase % 1) + 1) % 1;
  for (let index = 0; index < points.length - 1; index += 1) {
    const left = points[index];
    const right = points[index + 1];
    if (normalizedPhase <= right.phase) {
      const span = Math.max(0.0001, right.phase - left.phase);
      const t = clamp01((normalizedPhase - left.phase) / span);
      return quadraticBezier(left.value, controlValue(left, right), right.value, t);
    }
  }
  const first = points[0];
  const last = points[points.length - 1];
  const span = Math.max(0.0001, first.phase + 1 - last.phase);
  const t = clamp01((normalizedPhase + 1 - last.phase) / span);
  return lerp(last.value, first.value, t);
}

export function primaryLane(
  clip: ChromaticBulgeGridClip,
): { lane: LaneId; index: number } | null {
  const lane =
    clip.source?.lane ??
    clip.authoring?.tracks[0]?.lane ??
    LANE_ORDER.find((candidate) => ((clip.lanes as Record<string, unknown>)[candidate] as unknown[] | undefined)?.length);
  return lane ? { lane, index: LANE_ORDER.indexOf(lane) } : null;
}

export function totalBeats(timeline: ChromaticBulgeGridClipTimeline): number {
  return timeline.measures * timeline.beats_per_measure;
}

export function createPlacement(clip: ChromaticBulgeGridClip, startBeat: number): ClipPlacement {
  return {
    clip_id: clip.id,
    start_beat: startBeat,
    track: primaryLane(clip)?.index ?? 0,
    repeats: 1,
  };
}

export function defaultStep(lane: LaneId): ClipTweenStep {
  return isColorLane(lane)
    ? {
        to: { kind: "color", value: [1, 1, 1] },
        duration_beats: 1,
        ease: "linear",
      }
    : {
        to: { kind: "float", value: 1 },
        duration_beats: 1,
        ease: "linear",
      };
}

export function syncClipAuthoring(config: TrackVisualizerConfig, _clipId: string): TrackVisualizerConfig {
  return config;
}

export function timelineFromConfig(config: TrackVisualizerConfig): ChromaticBulgeGridClipTimeline {
  return config.timeline ?? defaultVisualizer().timeline!;
}

export function buildTimelineIndex(timeline: ChromaticBulgeGridClipTimeline): TimelineIndex {
  const clipsById = new Map<string, ChromaticBulgeGridClip>();
  for (const clip of timeline.clips) {
    clipsById.set(clip.id, clip);
  }

  const placements: IndexedTimelinePlacement[] = [];
  const placementsByTrack = new Map<number, IndexedTimelinePlacement[]>();

  timeline.arrangement.forEach((placement, placementIndex) => {
    const clip = clipsById.get(placement.clip_id);
    if (!clip) {
      return;
    }
    const indexedPlacement = {
      clip,
      endBeat: placement.start_beat + clip.length_beats * placement.repeats,
      placement,
      placementIndex,
    };
    placements.push(indexedPlacement);
    const trackPlacements = placementsByTrack.get(placement.track);
    if (trackPlacements) {
      trackPlacements.push(indexedPlacement);
    } else {
      placementsByTrack.set(placement.track, [indexedPlacement]);
    }
  });

  return {
    clipsById,
    placements,
    placementsByTrack,
  };
}

export function legacyAutomationToTimeline(
  automation: ChromaticBulgeGridAutomation,
): ChromaticBulgeGridClipTimeline {
  const totalClipBeats = automation.measures * automation.beats_per_measure;
  const clips = LANE_ORDER.flatMap((lane, index) => {
    const clip = {
      id: `legacy_${lane}`,
      name: `Legacy ${lane}`,
      length_beats: totalClipBeats,
      color: palette(index),
      source: undefined,
      authoring: undefined,
      lanes: onlyLane(automation.lanes, lane),
    };
    const value = (clip.lanes as Record<string, unknown>)[lane];
    return Array.isArray(value) && value.length > 0 ? [clip] : [];
  });
  const resolvedClips = clips.length
    ? clips
    : [
        {
          id: "legacy_timeline",
          name: "Legacy Timeline",
          length_beats: totalClipBeats,
          color: palette(0),
          source: undefined,
          authoring: undefined,
          lanes: structuredClone(automation.lanes),
        },
      ];
  return {
    bpm: automation.bpm,
    measures: automation.measures,
    beats_per_measure: automation.beats_per_measure,
    clips: resolvedClips,
    arrangement: resolvedClips.map((clip) => createPlacement(clip, 0)),
  };
}

export function isLegacyClip(clip: ChromaticBulgeGridClip): boolean {
  return !clip.source?.kind;
}

export function clipSummary(clip: ChromaticBulgeGridClip): string {
  if (!clip.source?.kind) return "Legacy step clip";
  return `${clip.source.min.toFixed(2)}-${clip.source.max.toFixed(2)} · ${clip.source.period_beats.toFixed(2)}b`;
}

function laneDefaultSpan(lane: LaneId): number {
  switch (lane) {
    case "lattice_density":
      return 2;
    case "rim_exponent":
      return 0.5;
    case "motion_rate":
    case "motion_rate_y":
    case "color_cycle_rate":
      return 0.25;
    default:
      return 0.15;
  }
}

function normalizeShaderState(
  state: Partial<ChromaticBulgeGridShaderState> | undefined,
): ChromaticBulgeGridShaderState {
  const defaults = defaultShaderState();
  return {
    motion_rate: finiteOr(state?.motion_rate, defaults.motion_rate),
    motion_rate_y: finiteOr(state?.motion_rate_y, defaults.motion_rate_y),
    lattice_density: finiteOr(state?.lattice_density, defaults.lattice_density),
    circle_radius: finiteOr(state?.circle_radius, defaults.circle_radius),
    circle_falloff_start: finiteOr(state?.circle_falloff_start, defaults.circle_falloff_start),
    circle_falloff_end: finiteOr(state?.circle_falloff_end, defaults.circle_falloff_end),
    bulge_amount: finiteOr(state?.bulge_amount, defaults.bulge_amount),
    rim_guard: finiteOr(state?.rim_guard, defaults.rim_guard),
    rim_exponent: finiteOr(state?.rim_exponent, defaults.rim_exponent),
    rim_warp: finiteOr(state?.rim_warp, defaults.rim_warp),
    dot_size: finiteOr(state?.dot_size, defaults.dot_size),
    outer_dot_scale: finiteOr(state?.outer_dot_scale, defaults.outer_dot_scale),
    edge_softness: finiteOr(state?.edge_softness, defaults.edge_softness),
    chromatic_aberration: finiteOr(state?.chromatic_aberration, defaults.chromatic_aberration),
    cold_color: clampColor(state?.cold_color ?? defaults.cold_color),
    hot_color: clampColor(state?.hot_color ?? defaults.hot_color),
    color_cycle_rate: finiteOr(state?.color_cycle_rate, defaults.color_cycle_rate),
    inner_alpha: finiteOr(state?.inner_alpha, defaults.inner_alpha),
  };
}

function normalizeShapePoints(points: LfoPoint[]): LfoPoint[] {
  const next = (points ?? [])
    .map((point) =>
      normalizedPoint({
        phase: clamp01(point.phase),
        value: clamp01(point.value),
        curve_to_next: clamp(point.curve_to_next ?? 0, -1, 1),
      }),
    )
    .sort((left, right) => left.phase - right.phase);
  const normalized = dedupePoints(next).slice(0, 64);
  if (normalized.length) {
    normalized[normalized.length - 1] = normalizedPoint({
      ...normalized[normalized.length - 1],
      curve_to_next: 0,
    });
  }
  return normalized;
}

function dedupePoints(points: LfoPoint[]): LfoPoint[] {
  const deduped: LfoPoint[] = [];
  for (const point of points) {
    const existing = deduped.findIndex((candidate) => Math.abs(candidate.phase - point.phase) < 0.0001);
    if (existing >= 0) deduped[existing] = point;
    else deduped.push(point);
  }
  return deduped;
}

function sampleFloatLane(keyframes: FloatKeyframe[], beat: number, base: number): number {
  if (!keyframes.length) return base;
  const clampedBeat = Math.max(0, beat);
  if (clampedBeat < keyframes[0].beat) return base;
  for (let index = 0; index < keyframes.length - 1; index += 1) {
    const left = keyframes[index];
    const right = keyframes[index + 1];
    if (clampedBeat < right.beat) {
      if (left.interpolation === "hold") return left.value;
      const span = Math.max(0.0001, right.beat - left.beat);
      return lerp(left.value, right.value, clamp01((clampedBeat - left.beat) / span));
    }
  }
  return keyframes[keyframes.length - 1].value;
}

function sampleColorLane(
  keyframes: ColorKeyframe[],
  beat: number,
  base: [number, number, number],
): [number, number, number] {
  if (!keyframes.length) return base;
  const clampedBeat = Math.max(0, beat);
  if (clampedBeat < keyframes[0].beat) return base;
  for (let index = 0; index < keyframes.length - 1; index += 1) {
    const left = keyframes[index];
    const right = keyframes[index + 1];
    if (clampedBeat < right.beat) {
      if (left.interpolation === "hold") return left.value;
      const span = Math.max(0.0001, right.beat - left.beat);
      const t = clamp01((clampedBeat - left.beat) / span);
      return [
        lerp(left.value[0], right.value[0], t),
        lerp(left.value[1], right.value[1], t),
        lerp(left.value[2], right.value[2], t),
      ];
    }
  }
  return keyframes[keyframes.length - 1].value;
}

function onlyLane(lanes: ChromaticBulgeGridAutomationLanes, lane: LaneId): ChromaticBulgeGridAutomationLanes {
  return { [lane]: structuredClone((lanes as Record<string, unknown>)[lane] ?? []) } as ChromaticBulgeGridAutomationLanes;
}

function clampState(state: ChromaticBulgeGridShaderState): ChromaticBulgeGridShaderState {
  return {
    ...state,
    motion_rate: clamp(state.motion_rate, -4, 4),
    motion_rate_y: clamp(state.motion_rate_y, -4, 4),
    lattice_density: clamp(state.lattice_density, 2, 12),
    circle_radius: clamp(state.circle_radius, 0.02, 0.95),
    circle_falloff_start: clamp(state.circle_falloff_start, 0, 1),
    circle_falloff_end: clamp(state.circle_falloff_end, 0, 1.2),
    bulge_amount: clamp(state.bulge_amount, 0, 1.5),
    rim_guard: clamp(state.rim_guard, 0.01, 2),
    rim_exponent: clamp(state.rim_exponent, 0.1, 6),
    rim_warp: clamp(state.rim_warp, 0, 64),
    dot_size: clamp(state.dot_size, 0.02, 1),
    outer_dot_scale: clamp(state.outer_dot_scale, 0.02, 2),
    edge_softness: clamp(state.edge_softness, 0.1, 8),
    chromatic_aberration: clamp(state.chromatic_aberration, 0, 24),
    cold_color: clampColor(state.cold_color),
    hot_color: clampColor(state.hot_color),
    color_cycle_rate: clamp(state.color_cycle_rate, 0, 6),
    inner_alpha: clamp(state.inner_alpha, 0, 1),
  };
}

function clampColor(value: [number, number, number]): [number, number, number] {
  return [clamp01(value[0]), clamp01(value[1]), clamp01(value[2])];
}

function palette(index: number): [number, number, number] {
  const colors: Array<[number, number, number]> = [
    [0.43, 0.86, 0.83],
    [0.9, 0.57, 0.34],
    [0.55, 0.73, 0.96],
    [0.86, 0.69, 0.35],
    [0.69, 0.85, 0.49],
    [0.91, 0.5, 0.59],
  ];
  return colors[index % colors.length];
}

export function shapePath(shape: ChromaticBulgeGridLfoShape, width: number, height: number, paddingY = 0): string {
  const points = normalizeShapePoints(shape.points);
  if (!points.length) return "";
  if (points.length === 1) {
    return `M ${points[0].phase * width} ${valueToEditorY(points[0].value, height, paddingY)}`;
  }
  const first = points[0];
  const commands = [`M ${first.phase * width} ${valueToEditorY(first.value, height, paddingY)}`];
  for (const segment of shapeSegments(shape, width, height, paddingY)) {
    commands.push(
      `Q ${segment.controlX} ${segment.controlY} ${segment.right.phase * width} ${valueToEditorY(segment.right.value, height, paddingY)}`,
    );
  }
  return commands.join(" ");
}

export function shapeSegments(
  shape: ChromaticBulgeGridLfoShape,
  width: number,
  height: number,
  paddingY = 0,
): ShapeSegment[] {
  const points = normalizeShapePoints(shape.points);
  if (points.length < 2) return [];
  return points.slice(0, -1).map((left, index) => {
    const right = points[index + 1];
    const midpointX = ((left.phase + right.phase) * width) / 2;
    const midpointY = valueToEditorY((left.value + right.value) / 2, height, paddingY);
    return {
      index,
      left,
      right,
      midpointX,
      midpointY,
      controlX: midpointX,
      controlY: valueToEditorY(controlValue(left, right), height, paddingY),
    };
  });
}

export function pointLabel(point: LfoPoint): string {
  return `${point.phase.toFixed(3)} / ${point.value.toFixed(3)}`;
}

function point(phase: number, value: number): LfoPoint {
  return { phase, value };
}

function normalizedPoint(point: LfoPoint): LfoPoint {
  const curveToNext = clamp(point.curve_to_next ?? 0, -1, 1);
  return curveToNext === 0
    ? { phase: point.phase, value: point.value }
    : { phase: point.phase, value: point.value, curve_to_next: curveToNext };
}

export function controlValue(left: LfoPoint, right: LfoPoint): number {
  const curveToNext = clamp(left.curve_to_next ?? 0, -1, 1);
  const midpoint = (left.value + right.value) / 2;
  return curveToNext >= 0
    ? lerp(midpoint, 1, curveToNext)
    : lerp(midpoint, 0, Math.abs(curveToNext));
}

export function curveFromControlValue(left: LfoPoint, right: LfoPoint, nextControlValue: number): number {
  const midpoint = (left.value + right.value) / 2;
  const clampedControl = clamp01(nextControlValue);
  if (Math.abs(clampedControl - midpoint) < 0.0001) {
    return 0;
  }
  if (clampedControl > midpoint) {
    return midpoint >= 1 ? 0 : clamp((clampedControl - midpoint) / Math.max(0.0001, 1 - midpoint), 0, 1);
  }
  return -clamp((midpoint - clampedControl) / Math.max(0.0001, midpoint), 0, 1);
}

function quadraticBezier(a: number, control: number, b: number, t: number): number {
  const inverse = 1 - t;
  return inverse * inverse * a + 2 * inverse * t * control + t * t * b;
}

function valueToEditorY(value: number, height: number, paddingY = 0): number {
  const safePadding = Math.max(0, Math.min(height / 2 - 1, paddingY));
  const usableHeight = Math.max(1, height - safePadding * 2);
  return safePadding + (1 - value) * usableHeight;
}

function finiteOr(value: number | undefined, fallback: number): number {
  return Number.isFinite(value) ? Number(value) : fallback;
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function clamp01(value: number): number {
  return clamp(value, 0, 1);
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.max(min, Math.min(max, value));
}
