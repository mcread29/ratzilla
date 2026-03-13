import { PointerEvent } from "react";
import { beatToPx } from "../utils/timelineMath";

export function ArrangementRuler({
  totalTimelineBeats,
  beatsPerMeasure,
  leadInBars,
  timelineWidth,
  timelineZoom,
  scrollLeft,
  onPointerDown,
}: {
  totalTimelineBeats: number;
  beatsPerMeasure: number;
  leadInBars: number;
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
          const barIndex = beat / beatsPerMeasure;
          const barLabel = barIndex - leadInBars + 1;
          return (
            <div
              key={beat}
              className={`ruler-mark ${isBar ? "bar" : ""}`}
              style={{ left: beatToPx(beat, timelineZoom) }}
            >
              {isBar && beat < totalTimelineBeats ? <span>{barLabel}</span> : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
