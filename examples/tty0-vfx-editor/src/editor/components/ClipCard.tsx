import { DragEvent } from "react";
import { ChromaticBulgeGridClip, LaneId } from "../../types";
import { primaryLane } from "../../vfx";
import { ClipDropIndicator } from "../editor-types";
import { clipListRangeLabel, formatClipBarLength } from "../utils/formatting";
import { visibleLane } from "../utils/lanes";
import { Button } from "@/components/ui/button";

export function ClipCard({
  clip,
  selectedClipId,
  selectedLane,
  timelineBeatsPerMeasure,
  draggedClipId,
  clipDropIndicator,
  onSelect,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
}: {
  clip: ChromaticBulgeGridClip;
  selectedClipId: string | null;
  selectedLane: LaneId;
  timelineBeatsPerMeasure: number;
  draggedClipId: string | null;
  clipDropIndicator: ClipDropIndicator | null;
  onSelect: (clipId: string, lane: LaneId) => void;
  onDragStart: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onDragOver: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onDrop: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onDragEnd: () => void;
}) {
  return (
    <Button
      className={[
        "clip-card",
        "h-auto w-full justify-start rounded-xl border border-border/80 bg-card px-3 py-3 text-left shadow-none hover:bg-accent/35",
        selectedClipId === clip.id ? "selected" : "",
        draggedClipId === clip.id ? "dragging" : "",
        clipDropIndicator?.clipId === clip.id ? `drop-${clipDropIndicator.position}` : "",
      ]
        .filter(Boolean)
        .join(" ")}
      draggable
      onClick={() => onSelect(clip.id, visibleLane(primaryLane(clip)?.lane ?? selectedLane))}
      onDragStart={(event) => onDragStart(event, clip.id)}
      onDragOver={(event) => onDragOver(event, clip.id)}
      onDrop={(event) => onDrop(event, clip.id)}
      onDragEnd={onDragEnd}
      variant="ghost"
    >
      <span
        className="clip-swatch"
        style={{ background: `rgb(${clip.color.map((channel) => Math.round(channel * 255)).join(" ")})` }}
      />
      <div className="clip-card-copy">
        <div className="clip-card-title">
          <span>{formatClipBarLength(clip.length_beats, timelineBeatsPerMeasure)}</span>
          <strong>{clip.name}</strong>
        </div>
        <span>{clipListRangeLabel(clip)}</span>
      </div>
    </Button>
  );
}
