import { ClipTweenStep, LaneId } from "../../types";
import { LaneMeta } from "../editor-types";
import { LANE_ORDER, primaryLane } from "../../vfx";

export const HIDDEN_EDITOR_LANES = new Set<LaneId>(["color_cycle_rate"]);
export const EDITOR_LANES = LANE_ORDER.filter((lane) => !HIDDEN_EDITOR_LANES.has(lane));

export function editableTrack(
  clip: { authoring?: { tracks: Array<{ lane: LaneId; steps: ClipTweenStep[] }> } },
  fallbackLane: LaneId,
) {
  return clip.authoring?.tracks[0] ?? { lane: visibleLane(primaryLane(clip as never)?.lane ?? fallbackLane), steps: [] };
}

export function visibleLane(lane: LaneId | null | undefined): LaneId {
  if (!lane || HIDDEN_EDITOR_LANES.has(lane)) {
    return "motion_rate";
  }
  return lane;
}

export function laneMeta(lane: LaneId): LaneMeta {
  switch (lane) {
    case "motion_rate":
      return {
        label: "Motion X",
        trackLabel: "Motion X",
        description: "Controls horizontal movement of the dot field. Positive values move right and negative values move left.",
      };
    case "motion_rate_y":
      return {
        label: "Motion Y",
        trackLabel: "Motion Y",
        description: "Controls vertical movement of the dot field. Positive values move down and negative values move up.",
      };
    case "lattice_density":
      return {
        label: "Lattice Density",
        trackLabel: "Density",
        description: "Changes how tightly packed the dot grid is.",
      };
    case "circle_radius":
      return {
        label: "Bulge Radius",
        trackLabel: "Bulge Radius",
        description: "Sets how large the bulge appears on screen.",
      };
    case "circle_falloff_start":
      return {
        label: "Bulge Falloff Start",
        trackLabel: "Falloff Start",
        description: "Determines where the bulge begins fading from its center toward the edge.",
      };
    case "circle_falloff_end":
      return {
        label: "Bulge Falloff End",
        trackLabel: "Falloff End",
        description: "Determines where the bulge fully blends back into the surrounding field.",
      };
    case "bulge_amount":
      return {
        label: "Bulge Amount",
        trackLabel: "Bulge Amount",
        description: "Controls how strongly the center distorts the dot field.",
      };
    case "rim_guard":
      return {
        label: "Rim Guard",
        trackLabel: "Rim Guard",
        description: "Stabilizes the edge of the bulge to keep the rim from blowing out.",
      };
    case "rim_exponent":
      return {
        label: "Rim Exponent",
        trackLabel: "Rim Exponent",
        description: "Shapes how concentrated the rim emphasis is near the edge of the bulge.",
      };
    case "rim_warp":
      return {
        label: "Rim Warp",
        trackLabel: "Rim Warp",
        description: "Pushes dots around the outer ring of the bulge for a stronger lens edge.",
      };
    case "dot_size":
      return {
        label: "Dot Size",
        trackLabel: "Dot Size",
        description: "Controls the base size of every dot in the field.",
      };
    case "outer_dot_scale":
      return {
        label: "Outer Dot Scale",
        trackLabel: "Outer Scale",
        description: "Scales dots outside the bulge relative to the dots at the center.",
      };
    case "edge_softness":
      return {
        label: "Edge Softness",
        trackLabel: "Edge Softness",
        description: "Softens or hardens the edge of each dot.",
      };
    case "chromatic_aberration":
      return {
        label: "Chromatic Aberration",
        trackLabel: "Chromatic Shift",
        description: "Offsets red and blue channels near the bulge for a split-color fringe.",
      };
    case "cold_color":
      return {
        label: "Surrounding Dots Color",
        trackLabel: "Outer Color",
        description: "Sets the color of the dots surrounding the bulge.",
      };
    case "hot_color":
      return {
        label: "Bulge Center Color",
        trackLabel: "Center Color",
        description: "Sets the color of the dots at the center of the bulge.",
      };
    case "color_cycle_rate":
      return {
        label: "Legacy Color Cycle",
        trackLabel: "Legacy Cycle",
        description: "Legacy control from the earlier shader version. The current shader no longer cycles between colors.",
      };
    case "inner_alpha":
      return {
        label: "Bulge Center Alpha",
        trackLabel: "Center Alpha",
        description: "Controls how opaque the dots stay inside the bulge.",
      };
  }
}

export function ensureClipTrack(
  clip: { authoring?: { tracks: Array<{ lane: LaneId; steps: ClipTweenStep[] }> } } | undefined,
  fallbackLane: LaneId,
) {
  if (!clip) {
    return null;
  }
  if (!clip.authoring) {
    clip.authoring = { tracks: [] };
  }
  if (!clip.authoring.tracks[0]) {
    clip.authoring.tracks[0] = {
      lane: primaryLane(clip as never)?.lane ?? fallbackLane,
      steps: [],
    };
  }
  return clip.authoring.tracks[0];
}
