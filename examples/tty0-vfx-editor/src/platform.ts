import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { LoadedRecord, ProjectHandle, RecordSummary, TrackVisualizerConfig } from "./types";
import { defaultVisualizer } from "./vfx";

export function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function pickProjectRoot(): Promise<ProjectHandle | null> {
  if (!isTauri()) {
    return null;
  }
  const root = await openDialog({ directory: true, multiple: false });
  if (!root || Array.isArray(root)) {
    return null;
  }
  return invoke<ProjectHandle>("open_project_root", { path: root });
}

export async function listRecords(projectRoot: string): Promise<RecordSummary[]> {
  return invoke<RecordSummary[]>("list_records", { projectRoot });
}

export async function loadRecord(
  projectRoot: string,
  recordId: string,
): Promise<LoadedRecord> {
  return invoke<LoadedRecord>("load_record", { projectRoot, recordId });
}

export async function saveRecordVisualizer(
  projectRoot: string,
  recordId: string,
  visualizer: TrackVisualizerConfig,
): Promise<string> {
  return invoke<string>("save_record_visualizer", {
    projectRoot,
    recordId,
    visualizerJson: JSON.stringify(visualizer),
  });
}

export async function revealRecordFile(
  projectRoot: string,
  recordId: string,
): Promise<void> {
  await invoke("reveal_record_file", { projectRoot, recordId });
}

export async function importRecordFromJson(file: File): Promise<LoadedRecord> {
  const rawRecordJson = await file.text();
  const parsed = JSON.parse(rawRecordJson) as {
    id: string;
    title: string;
    media_page?: {
      visualizer?: TrackVisualizerConfig | null;
    };
  };
  return {
    record_id: parsed.id,
    record_path: file.name,
    record_title: parsed.title,
    audio_url: null,
    visualizer: parsed.media_page?.visualizer ?? defaultVisualizer(),
    raw_record_json: rawRecordJson,
    has_legacy_automation: !!parsed.media_page?.visualizer?.automation,
  };
}
