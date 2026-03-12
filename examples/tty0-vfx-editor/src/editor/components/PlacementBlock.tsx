import { MouseEvent } from "react";
import { IndexedTimelinePlacement } from "../../vfx";
import { formatPlacementLabel } from "../utils/formatting";
import { placementBackground } from "../utils/color";

export function PlacementBlock({
  entry,
  placement,
  left,
  width,
  isSelected,
  isActive,
  timelineZoom,
  onPointerDown,
}: {
  entry: IndexedTimelinePlacement;
  placement: IndexedTimelinePlacement["placement"];
  left: number;
  width: number;
  isSelected: boolean;
  isActive: boolean;
  timelineZoom: number;
  onPointerDown: (event: MouseEvent<HTMLDivElement>) => void;
}) {
  void timelineZoom;
  return (
    <div
      key={`${entry.placement.clip_id}-${entry.placementIndex}`}
      className={`placement ${isSelected ? "selected" : ""} ${isActive ? "active" : ""}`}
      title={entry.clip.name}
      style={{
        left,
        width,
        background: placementBackground(entry.clip.color, isSelected),
      }}
      onPointerDown={onPointerDown}
    >
      <span className="placement-label">{formatPlacementLabel(entry.clip.name, width)}</span>
      <div className="resize-handle" />
    </div>
  );
}
