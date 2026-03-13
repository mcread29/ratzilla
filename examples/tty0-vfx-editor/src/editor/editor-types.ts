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
  name: string;
  savedSnapshot: string | null;
  audioPath: string | null;
  source:
    | { kind: "local-project"; projectId: string }
    | { kind: "imported-json"; sourceName: string | null }
    | { kind: "new-draft" };
  visualizer: TrackVisualizerConfig;
};

export type MountedAudioState =
  | { kind: "none" }
  | { kind: "path"; path: string }
  | { kind: "imported-file"; fileName: string; objectUrl: string; blobKey: string | null; blob: Blob };

export type LaneMeta = {
  label: string;
  trackLabel: string;
  description: string;
};

export type VisibleLaneId = LaneId;
