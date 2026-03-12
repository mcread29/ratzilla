import { PointerEvent } from "react";
import { beatToPx } from "../utils/timelineMath";

export function ArrangementRuler({
  totalTimelineBeats,
  beatsPerMeasure,
  timelineWidth,
  timelineZoom,
  scrollLeft,
  onPointerDown,
}: {
  totalTimelineBeats: number;
  beatsPerMeasure: number;
  timelineWidth: number;
  timelineZoom: number;
  scrollLeft: number;
  onPointerDown: (event: PointerEvent<HTMLDivElement>) => void;
}) {
  return (
    <div className="ruler-row">
      <div
        className="ruler-track"
        style={{
          width: timelineWidth,
          transform: `translateX(${-scrollLeft}px)`,
        }}
        onPointerDown={onPointerDown}
      >
        {Array.from({ length: Math.ceil(totalTimelineBeats) + 1 }, (_, beat) => {
          const isBar = beat % beatsPerMeasure === 0;
          return (
            <div
              key={beat}
              className={`ruler-mark ${isBar ? "bar" : ""}`}
              style={{ left: beatToPx(beat, timelineZoom) }}
            >
              {isBar && beat < totalTimelineBeats ? <span>{beat / beatsPerMeasure + 1}</span> : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
