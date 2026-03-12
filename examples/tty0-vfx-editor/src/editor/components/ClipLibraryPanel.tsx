import { DragEvent } from "react";
import { ChromaticBulgeGridClip, LaneId } from "../../types";
import { ClipDropIndicator } from "../editor-types";
import { ClipActionIcon } from "./icons";
import { ClipCard } from "./ClipCard";

export function ClipLibraryPanel({
  laneClips,
  selectedClipId,
  selectedLane,
  timelineBeatsPerMeasure,
  draggedClipId,
  clipDropIndicator,
  onSelectClip,
  onAddClip,
  onDuplicateClip,
  onDeleteClip,
  onClipDragStart,
  onClipDragOver,
  onClipDrop,
  onClearClipDragState,
  hasSelectedClip,
}: {
  laneClips: ChromaticBulgeGridClip[];
  selectedClipId: string | null;
  selectedLane: LaneId;
  timelineBeatsPerMeasure: number;
  draggedClipId: string | null;
  clipDropIndicator: ClipDropIndicator | null;
  onSelectClip: (clipId: string, lane: LaneId) => void;
  onAddClip: () => void;
  onDuplicateClip: () => void;
  onDeleteClip: () => void;
  onClipDragStart: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onClipDragOver: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onClipDrop: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onClearClipDragState: () => void;
  hasSelectedClip: boolean;
}) {
  return (
    <section className="panel library-panel clips-panel">
      <div className="panel-header">
        <h2>Clips</h2>
        <div className="inline-actions">
          <button className="icon-button" onClick={onAddClip} type="button" title="Add clip" aria-label="Add clip">
            <ClipActionIcon name="add" />
          </button>
          <button
            className="icon-button"
            onClick={onDuplicateClip}
            disabled={!hasSelectedClip}
            type="button"
            title="Copy clip"
            aria-label="Copy clip"
          >
            <ClipActionIcon name="copy" />
          </button>
          <button
            className="icon-button"
            onClick={onDeleteClip}
            disabled={!hasSelectedClip}
            type="button"
            title="Delete clip"
            aria-label="Delete clip"
          >
            <ClipActionIcon name="delete" />
          </button>
        </div>
      </div>
      <div className="list">
        {laneClips.map((clip) => (
          <ClipCard
            key={clip.id}
            clip={clip}
            selectedClipId={selectedClipId}
            selectedLane={selectedLane}
            timelineBeatsPerMeasure={timelineBeatsPerMeasure}
            draggedClipId={draggedClipId}
            clipDropIndicator={clipDropIndicator}
            onSelect={onSelectClip}
            onDragStart={onClipDragStart}
            onDragOver={onClipDragOver}
            onDrop={onClipDrop}
            onDragEnd={onClearClipDragState}
          />
        ))}
        {!laneClips.length ? <p className="empty-copy">No clips for this track yet.</p> : null}
      </div>
    </section>
  );
}
