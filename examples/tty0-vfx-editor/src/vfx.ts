import {
  ChromaticBulgeGridAutomation,
  ChromaticBulgeGridAutomationLanes,
  ChromaticBulgeGridClip,
  ChromaticBulgeGridClipAuthoring,
  ChromaticBulgeGridClipTimeline,
  ChromaticBulgeGridShaderState,
  ClipPlacement,
  ClipTweenEase,
  ClipTweenStep,
  ClipTweenValue,
  ColorKeyframe,
  FloatKeyframe,
  LaneId,
  TrackVisualizerConfig,
} from "./types";

export const LANE_ORDER: LaneId[] = [
  "motion_rate",
  "lattice_density",
  "circle_radius",
  "circle_falloff_start",
  "circle_falloff_end",
  "bulge_amount",
  "rim_guard",
  "rim_exponent",
  "rim_warp",
  "spacing_max_px",
  "spacing_min_px",
  "dot_size",
  "outer_dot_scale",
  "edge_softness",
  "chromatic_aberration",
  "scroll_base",
  "scroll_motion_scale",
  "scroll_motion_floor",
  "scroll_motion_ceiling",
  "cold_color",
  "hot_color",
  "color_cycle_rate",
  "inner_alpha",
];

const colorLaneSet = new Set<LaneId>(["cold_color", "hot_color"]);

export function isColorLane(lane: LaneId): boolean {
  return colorLaneSet.has(lane);
}

export function defaultShaderState(): ChromaticBulgeGridShaderState {
  return {
    motion_rate: 1,
    lattice_density: 6,
    circle_radius: 0.24,
    circle_falloff_start: 0.78,
    circle_falloff_end: 1,
    bulge_amount: 0.42,
    rim_guard: 0.55,
    rim_exponent: 1.8,
    rim_warp: 0.18,
    spacing_max_px: 22,
    spacing_min_px: 12,
    dot_size: 0.16,
    outer_dot_scale: 0.33,
    edge_softness: 1,
    chromatic_aberration: 0.28,
    scroll_base: 28,
    scroll_motion_scale: 42,
    scroll_motion_floor: 0.2,
    scroll_motion_ceiling: 2.8,
    cold_color: [1, 1, 1],
    hot_color: [1, 1, 1],
    color_cycle_rate: 0.16,
    inner_alpha: 0.9,
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
          authoring: {
            tracks: [{ lane: "motion_rate", steps: [] }],
          },
          lanes: {},
        },
      ],
      arrangement: [],
    },
  };
}

export function normalizedConfig(config: TrackVisualizerConfig): TrackVisualizerConfig {
  const next = structuredClone(config);
  if (!next.params.shader_states) {
    const base = defaultShaderState();
    next.params.shader_states = { playing: base, idle: base };
  }
  next.params.shader_states.idle = structuredClone(next.params.shader_states.playing);
  if (next.timeline) {
    next.automation = null;
    next.timeline = normalizeTimeline(next.timeline);
  }
  return next;
}

export function normalizeTimeline(
  timeline: ChromaticBulgeGridClipTimeline,
): ChromaticBulgeGridClipTimeline {
  const next = structuredClone(timeline);
  next.bpm = Math.max(1, Number.isFinite(next.bpm) ? next.bpm : 120);
  next.measures = Math.max(1, Math.round(next.measures || 1));
  next.beats_per_measure = Math.max(1, Math.round(next.beats_per_measure || 4));
  next.clips = next.clips.map((clip) => ({
    ...clip,
    length_beats: Math.max(0.0001, clip.length_beats || 1),
    lanes: sortLanes(clip.lanes ?? {}),
  }));
  next.arrangement = next.arrangement.map((placement) => ({
    ...placement,
    start_beat: Math.max(0, placement.start_beat || 0),
    repeats: Math.max(1, placement.repeats || 1),
  }));
  return next;
}

function sortLanes(lanes: ChromaticBulgeGridAutomationLanes): ChromaticBulgeGridAutomationLanes {
  const next = structuredClone(lanes);
  for (const lane of LANE_ORDER) {
    const keyframes = next[lane];
    if (keyframes) {
      keyframes.sort((a, b) => a.beat - b.beat);
    }
  }
  return next;
}

export function compileAuthoringLanes(
  base: ChromaticBulgeGridShaderState,
  clipLengthBeats: number,
  authoring?: ChromaticBulgeGridClipAuthoring,
): ChromaticBulgeGridAutomationLanes {
  const lanes: ChromaticBulgeGridAutomationLanes = {};
  if (!authoring) {
    return lanes;
  }
  for (const track of authoring.tracks) {
    if (isColorLane(track.lane)) {
      let cursor = 0;
      let current = getColorState(base, track.lane as "cold_color" | "hot_color");
      const laneKeyframes: ColorKeyframe[] = [];
      for (const step of track.steps) {
        if (step.to.kind !== "color") continue;
        const { keyframes, endValue, endBeat } = compileColorStep(
          current,
          step.to.value,
          cursor,
          step,
          clipLengthBeats,
        );
        laneKeyframes.push(...keyframes);
        current = endValue;
        cursor = endBeat;
      }
      (lanes as Record<string, unknown>)[track.lane] = laneKeyframes;
    } else {
      let cursor = 0;
      let current = Number(base[track.lane]);
      const laneKeyframes: FloatKeyframe[] = [];
      for (const step of track.steps) {
        if (step.to.kind !== "float") continue;
        const { keyframes, endValue, endBeat } = compileFloatStep(
          current,
          step.to.value,
          cursor,
          step,
          clipLengthBeats,
        );
        laneKeyframes.push(...keyframes);
        current = endValue;
        cursor = endBeat;
      }
      (lanes as Record<string, unknown>)[track.lane] = laneKeyframes;
    }
  }
  return sortLanes(lanes);
}

function compileFloatStep(
  startValue: number,
  endValue: number,
  startBeat: number,
  step: ClipTweenStep,
  clipLengthBeats: number,
): { keyframes: FloatKeyframe[]; endValue: number; endBeat: number } {
  const duration = Math.max(0, step.duration_beats);
  const endBeat = Math.min(startBeat + duration, Math.max(0, clipLengthBeats));
  const keyframes: FloatKeyframe[] = [
    {
      beat: startBeat,
      value: startValue,
      interpolation: step.ease === "hold" ? "hold" : "linear",
    },
  ];
  if (duration <= 0.0001 || endBeat <= startBeat || step.ease === "hold") {
    keyframes.push({ beat: endBeat, value: endValue, interpolation: "hold" });
    return { keyframes, endValue, endBeat };
  }
  const segments = easeSegments(step.ease);
  for (let index = 1; index <= segments; index += 1) {
    const t = index / segments;
    keyframes.push({
      beat: startBeat + (endBeat - startBeat) * t,
      value: lerp(startValue, endValue, easeProgress(step.ease, t)),
      interpolation: index === segments ? "hold" : "linear",
    });
  }
  return { keyframes, endValue, endBeat };
}

function compileColorStep(
  startValue: [number, number, number],
  endValue: [number, number, number],
  startBeat: number,
  step: ClipTweenStep,
  clipLengthBeats: number,
): { keyframes: ColorKeyframe[]; endValue: [number, number, number]; endBeat: number } {
  const duration = Math.max(0, step.duration_beats);
  const endBeat = Math.min(startBeat + duration, Math.max(0, clipLengthBeats));
  const keyframes: ColorKeyframe[] = [
    {
      beat: startBeat,
      value: startValue,
      interpolation: step.ease === "hold" ? "hold" : "linear",
    },
  ];
  if (duration <= 0.0001 || endBeat <= startBeat || step.ease === "hold") {
    keyframes.push({ beat: endBeat, value: endValue, interpolation: "hold" });
    return { keyframes, endValue, endBeat };
  }
  const segments = easeSegments(step.ease);
  for (let index = 1; index <= segments; index += 1) {
    const t = index / segments;
    const p = easeProgress(step.ease, t);
    keyframes.push({
      beat: startBeat + (endBeat - startBeat) * t,
      value: [
        lerp(startValue[0], endValue[0], p),
        lerp(startValue[1], endValue[1], p),
        lerp(startValue[2], endValue[2], p),
      ],
      interpolation: index === segments ? "hold" : "linear",
    });
  }
  return { keyframes, endValue, endBeat };
}

function easeSegments(ease: ClipTweenEase): number {
  switch (ease) {
    case "sine_in":
    case "sine_out":
      return 4;
    case "sine_in_out":
      return 6;
    default:
      return 1;
  }
}

function easeProgress(ease: ClipTweenEase, t: number): number {
  const x = Math.max(0, Math.min(1, t));
  switch (ease) {
    case "hold":
      return x >= 1 ? 1 : 0;
    case "sine_in":
      return 1 - Math.cos((x * Math.PI) / 2);
    case "sine_out":
      return Math.sin((x * Math.PI) / 2);
    case "sine_in_out":
      return -(Math.cos(Math.PI * x) * 0.5) + 0.5;
    default:
      return x;
  }
}

export function syncClipAuthoring(
  config: TrackVisualizerConfig,
  clipId: string,
): TrackVisualizerConfig {
  const next = normalizedConfig(config);
  const base = next.params.shader_states?.playing ?? defaultShaderState();
  if (!next.timeline) {
    return next;
  }
  next.timeline.clips = next.timeline.clips.map((clip) =>
    clip.id === clipId
      ? {
          ...clip,
          lanes: compileAuthoringLanes(base, clip.length_beats, clip.authoring),
        }
      : clip,
  );
  return next;
}

export function legacyAutomationToTimeline(
  automation: ChromaticBulgeGridAutomation,
): ChromaticBulgeGridClipTimeline {
  const totalBeats = automation.measures * automation.beats_per_measure;
  const lanes = LANE_ORDER.filter((lane) => (automation.lanes[lane] ?? []).length > 0);
  const clips =
    lanes.length > 0
      ? lanes.map((lane, index) => ({
          id: `imported_${lane}`,
          name: `Imported ${lane}`,
          length_beats: totalBeats,
          color: palette(index),
          lanes: { [lane]: automation.lanes[lane] } as ChromaticBulgeGridAutomationLanes,
        }))
      : [
          {
            id: "imported_timeline",
            name: "Imported Timeline",
            length_beats: totalBeats,
            color: palette(0),
            lanes: automation.lanes,
          },
        ];
  return {
    bpm: automation.bpm,
    measures: automation.measures,
    beats_per_measure: automation.beats_per_measure,
    clips,
    arrangement: clips.map((clip) => ({
      clip_id: clip.id,
      start_beat: 0,
      track: primaryLane(clip)?.index ?? 0,
      repeats: 1,
    })),
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
  const shaderStates = config.params.shader_states ?? {
    playing: defaultShaderState(),
    idle: defaultShaderState(),
  };
  const bpm = config.timeline?.bpm ?? config.automation?.bpm ?? 120;
  const currentBeat = Math.max(0, playback.currentTimeSecs) * Math.max(1, bpm) / 60;
  const base = playback.isPlaying ? shaderStates.playing : shaderStates.idle;
  if (config.timeline) {
    if (playback.isPlaying || playback.timelinePreview) {
      const resolved = resolveTimelineState(config.timeline, base, currentBeat);
      return { uniforms: resolved ?? base, currentBeat };
    }
    return { uniforms: base, currentBeat };
  }
  if (config.automation && playback.isPlaying) {
    return {
      uniforms: applyLanesToState(base, config.automation.lanes, currentBeat),
      currentBeat,
    };
  }
  return { uniforms: base, currentBeat };
}

function resolveTimelineState(
  timeline: ChromaticBulgeGridClipTimeline,
  base: ChromaticBulgeGridShaderState,
  beat: number,
): ChromaticBulgeGridShaderState | null {
  const active = timeline.arrangement
    .map((placement) => {
      const clip = timeline.clips.find((candidate) => candidate.id === placement.clip_id);
      if (!clip) return null;
      const endBeat = placement.start_beat + clip.length_beats * Math.max(1, placement.repeats);
      if (beat < placement.start_beat || beat >= endBeat) return null;
      const localBeat =
        clip.length_beats <= 0
          ? 0
          : Math.max(0, Math.min(clip.length_beats, (beat - placement.start_beat) % clip.length_beats));
      return { clip, localBeat, track: placement.track, startBeat: placement.start_beat };
    })
    .filter(Boolean)
    .sort((left, right) => {
      const a = left!;
      const b = right!;
      return a.track - b.track || a.startBeat - b.startBeat;
    }) as Array<{
    clip: ChromaticBulgeGridClip;
    localBeat: number;
    track: number;
    startBeat: number;
  }>;
  if (active.length === 0) return null;
  let state = structuredClone(base);
  for (const item of active) {
    state = applyLanesToState(state, item.clip.lanes, item.localBeat);
  }
  return state;
}

function applyLanesToState(
  base: ChromaticBulgeGridShaderState,
  lanes: ChromaticBulgeGridAutomationLanes,
  beat: number,
): ChromaticBulgeGridShaderState {
  const state = structuredClone(base);
  for (const lane of LANE_ORDER) {
    const keyframes = lanes[lane];
    if (!keyframes || keyframes.length === 0) continue;
    if (isColorLane(lane)) {
      (state as unknown as Record<string, unknown>)[lane] = sampleColorLane(
        keyframes as ColorKeyframe[],
        beat,
        getColorState(state, lane as "cold_color" | "hot_color"),
      );
    } else {
      (state as unknown as Record<string, unknown>)[lane] = sampleFloatLane(
        keyframes as FloatKeyframe[],
        beat,
        Number((state as unknown as Record<string, unknown>)[lane]),
      );
    }
  }
  return state;
}

function getColorState(
  state: ChromaticBulgeGridShaderState,
  lane: Extract<LaneId, "cold_color" | "hot_color">,
): [number, number, number] {
  return [...state[lane]] as [number, number, number];
}

function sampleFloatLane(keyframes: FloatKeyframe[], beat: number, base: number): number {
  if (keyframes.length === 0) return base;
  const clampedBeat = Math.max(0, beat);
  if (clampedBeat < keyframes[0].beat) return base;
  for (let index = 0; index < keyframes.length - 1; index += 1) {
    const left = keyframes[index];
    const right = keyframes[index + 1];
    if (clampedBeat < right.beat) {
      if (left.interpolation === "hold") return left.value;
      const span = Math.max(0.0001, right.beat - left.beat);
      const t = Math.max(0, Math.min(1, (clampedBeat - left.beat) / span));
      return lerp(left.value, right.value, t);
    }
  }
  return keyframes[keyframes.length - 1].value;
}

function sampleColorLane(
  keyframes: ColorKeyframe[],
  beat: number,
  base: [number, number, number],
): [number, number, number] {
  if (keyframes.length === 0) return base;
  const clampedBeat = Math.max(0, beat);
  if (clampedBeat < keyframes[0].beat) return base;
  for (let index = 0; index < keyframes.length - 1; index += 1) {
    const left = keyframes[index];
    const right = keyframes[index + 1];
    if (clampedBeat < right.beat) {
      if (left.interpolation === "hold") return left.value;
      const span = Math.max(0.0001, right.beat - left.beat);
      const t = Math.max(0, Math.min(1, (clampedBeat - left.beat) / span));
      return [
        lerp(left.value[0], right.value[0], t),
        lerp(left.value[1], right.value[1], t),
        lerp(left.value[2], right.value[2], t),
      ];
    }
  }
  return keyframes[keyframes.length - 1].value;
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
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

export function primaryLane(
  clip: ChromaticBulgeGridClip,
): { lane: LaneId; index: number } | null {
  const lane =
    clip.authoring?.tracks[0]?.lane ??
    LANE_ORDER.find((candidate) => (clip.lanes[candidate] ?? []).length > 0);
  return lane ? { lane, index: LANE_ORDER.indexOf(lane) } : null;
}

export function totalBeats(timeline: ChromaticBulgeGridClipTimeline): number {
  return timeline.measures * timeline.beats_per_measure;
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

export function timelineFromConfig(config: TrackVisualizerConfig): ChromaticBulgeGridClipTimeline {
  if (config.timeline) return normalizeTimeline(config.timeline);
  if (config.automation) return legacyAutomationToTimeline(config.automation);
  return defaultVisualizer().timeline!;
}

export function createPlacement(
  clip: ChromaticBulgeGridClip,
  startBeat: number,
): ClipPlacement {
  return {
    clip_id: clip.id,
    start_beat: startBeat,
    track: primaryLane(clip)?.index ?? 0,
    repeats: 1,
  };
}
