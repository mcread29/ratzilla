import { LaneId, TrackVisualizerConfig } from "../types";

export type DragState =
  | { kind: "move"; placementIndex: number; offsetBeats: number }
  | { kind: "resize"; placementIndex: number }
  | { kind: "scrub" };

export type TimelineTool = "select" | "pencil";
export type ShapeInteractionMode = "add" | "move" | "delete";

export type PlacementClipboardEntry = {
  clipId: string;
  offsetBeats: number;
  repeats: number;
};

export type ClipDropIndicator = {
  clipId: string;
  position: "before" | "after";
};

export type LoadedDocument = {
  audioUrl: string | null;
  name: string;
  savedSnapshot: string;
  visualizer: TrackVisualizerConfig;
};

export type LaneMeta = {
  label: string;
  trackLabel: string;
  description: string;
};

export type VisibleLaneId = LaneId;
