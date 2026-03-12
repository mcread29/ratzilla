import { TimelineTool } from "../editor-types";
import { TimelineActionIcon, TimelineFieldIcon, TimelineToolIcon } from "./icons";

export function ArrangementToolbar({
  bpm,
  measures,
  beatsPerMeasure,
  timelineTool,
  hasSelectedClip,
  selectedPlacementCount,
  onChangeBpm,
  onChangeMeasures,
  onChangeBeatsPerMeasure,
  onChangeTimelineTool,
  onAddPlacement,
  onCopyPlacements,
  onDeletePlacements,
}: {
  bpm: number;
  measures: number;
  beatsPerMeasure: number;
  timelineTool: TimelineTool;
  hasSelectedClip: boolean;
  selectedPlacementCount: number;
  onChangeBpm: (value: number) => void;
  onChangeMeasures: (value: number) => void;
  onChangeBeatsPerMeasure: (value: number) => void;
  onChangeTimelineTool: (tool: TimelineTool) => void;
  onAddPlacement: () => void;
  onCopyPlacements: () => void;
  onDeletePlacements: () => void;
}) {
  return (
    <div className="inline-actions arrangement-toolbar">
      <div className="arrangement-toolbar-fields">
        <label className="compact-field">
          <span className="compact-field-icon" aria-hidden="true">
            <TimelineFieldIcon name="bpm" />
          </span>
          <input type="number" step={0.1} aria-label="BPM" value={bpm} onChange={(event) => onChangeBpm(Number(event.target.value))} />
        </label>
        <label className="compact-field">
          <span className="compact-field-icon" aria-hidden="true">
            <TimelineFieldIcon name="measures" />
          </span>
          <input
            type="number"
            step={1}
            min={1}
            aria-label="Measures"
            value={measures}
            onChange={(event) => onChangeMeasures(Number(event.target.value))}
          />
        </label>
        <label className="compact-field">
          <span className="compact-field-icon" aria-hidden="true">
            <TimelineFieldIcon name="time" />
          </span>
          <input
            type="number"
            step={1}
            min={1}
            aria-label="Time"
            value={beatsPerMeasure}
            onChange={(event) => onChangeBeatsPerMeasure(Number(event.target.value))}
          />
        </label>
      </div>
      <div className="arrangement-toolbar-actions">
        <div className="tool-toggle" role="group" aria-label="Timeline tool">
          <button
            className={timelineTool === "select" ? "active" : ""}
            onClick={() => onChangeTimelineTool("select")}
            type="button"
            title="Select tool"
            aria-label="Select tool"
            aria-pressed={timelineTool === "select"}
          >
            <TimelineToolIcon name="select" />
          </button>
          <button
            className={timelineTool === "pencil" ? "active" : ""}
            onClick={() => onChangeTimelineTool("pencil")}
            type="button"
            title="Pencil tool"
            aria-label="Pencil tool"
            aria-pressed={timelineTool === "pencil"}
          >
            <TimelineToolIcon name="pencil" />
          </button>
        </div>
        <button
          className="icon-button"
          onClick={onAddPlacement}
          disabled={!hasSelectedClip}
          type="button"
          title="Place selected clip"
          aria-label="Place selected clip"
        >
          <TimelineActionIcon name="place" />
        </button>
        <button
          className="icon-button"
          onClick={onCopyPlacements}
          disabled={!selectedPlacementCount}
          type="button"
          title={`Copy placement${selectedPlacementCount === 1 ? "" : "s"}`}
          aria-label={`Copy placement${selectedPlacementCount === 1 ? "" : "s"}`}
        >
          <TimelineActionIcon name="copy" />
        </button>
        <button
          className="icon-button"
          onClick={onDeletePlacements}
          disabled={!selectedPlacementCount}
          type="button"
          title={`Delete placement${selectedPlacementCount === 1 ? "" : "s"}`}
          aria-label={`Delete placement${selectedPlacementCount === 1 ? "" : "s"}`}
        >
          <TimelineActionIcon name="delete" />
        </button>
      </div>
    </div>
  );
}
