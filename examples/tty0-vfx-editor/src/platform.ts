import { TrackVisualizerConfig } from "./types";
import { defaultVisualizer } from "./vfx";

type ImportedJsonDocument =
  | {
      kind: "record";
      record: {
        audio_url: string | null;
        has_legacy_automation: boolean;
        record_id: string;
        visualizer: TrackVisualizerConfig;
      };
    }
  | { kind: "visualizer"; visualizer: TrackVisualizerConfig; sourceName: string };

function isTrackVisualizerConfig(value: unknown): value is TrackVisualizerConfig {
  return !!value && typeof value === "object" && "mode" in value && "params" in value;
}

export async function importRecordFromJson(file: File): Promise<ImportedJsonDocument> {
  const rawRecordJson = await file.text();
  const parsed = JSON.parse(rawRecordJson) as {
    id?: string;
    title?: string;
    media_page?: {
      visualizer?: TrackVisualizerConfig | null;
    };
  };

  if (isTrackVisualizerConfig(parsed)) {
    return {
      kind: "visualizer",
      visualizer: parsed,
      sourceName: file.name,
    };
  }

  if (typeof parsed.id !== "string" || typeof parsed.title !== "string") {
    throw new Error("Import failed: expected a tty0 record JSON or exported visualizer JSON.");
  }

  return {
    kind: "record",
    record: {
      record_id: parsed.id,
      audio_url: null,
      visualizer: parsed.media_page?.visualizer ?? defaultVisualizer(),
      has_legacy_automation: !!parsed.media_page?.visualizer?.automation,
    },
  };
}
