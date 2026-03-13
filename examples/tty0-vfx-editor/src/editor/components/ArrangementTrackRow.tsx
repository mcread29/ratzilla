import { MouseEvent } from "react";
import { LaneId, TrackVisualizerConfig } from "../../types";
import { IndexedTimelinePlacement } from "../../vfx";
import { TRACK_HEIGHT } from "../constants";
import { beatToPx, pxToBeat } from "../utils/timelineMath";
import { PlacementBlock } from "./PlacementBlock";

export function ArrangementTrackRow({
  lane,
  trackIndex,
  lanePlacements,
  timelineWidth,
  scrollLeft,
  timelineZoom,
  timelineTool,
  totalTimelineBeats,
  currentBeat,
  leadInBeatOffset,
  selectedPlacementSet,
  onBackgroundPointerDown,
  onPlacementPointerDown,
}: {
  lane: LaneId;
  trackIndex: number;
  lanePlacements: IndexedTimelinePlacement[];
  timelineWidth: number;
  scrollLeft: number;
  timelineZoom: number;
  timelineTool: "select" | "pencil";
  totalTimelineBeats: number;
  currentBeat: number;
  leadInBeatOffset: number;
  selectedPlacementSet: Set<number>;
  onBackgroundPointerDown: (lane: LaneId, event: React.PointerEvent<HTMLDivElement>) => void;
  onPlacementPointerDown: (
    entry: IndexedTimelinePlacement,
    lane: LaneId,
    beatAtCursor: number,
    hitEdge: boolean,
    event: MouseEvent<HTMLDivElement>,
  ) => void;
}) {
  void trackIndex;
  void timelineTool;
  void totalTimelineBeats;
  return (
    <div key={lane} className="track-row">
      <div
        className="track-lane"
        style={{
          height: TRACK_HEIGHT,
          width: timelineWidth,
          transform: `translateX(${-scrollLeft}px)`,
        }}
        onPointerDown={(event) => onBackgroundPointerDown(lane, event)}
      >
        {lanePlacements.map((entry) => {
          const placement = entry.placement;
          const left = beatToPx(placement.start_beat + leadInBeatOffset, timelineZoom);
          const width = beatToPx(entry.clip.length_beats * placement.repeats, timelineZoom);
          const isSelected = selectedPlacementSet.has(entry.placementIndex);
          const isActive =
            currentBeat >= placement.start_beat + leadInBeatOffset &&
            currentBeat < placement.start_beat + leadInBeatOffset + entry.clip.length_beats * placement.repeats;
          return (
            <PlacementBlock
              key={`${entry.placement.clip_id}-${entry.placementIndex}`}
              entry={entry}
              placement={placement}
              left={left}
              width={width}
              isSelected={isSelected}
              isActive={isActive}
              timelineZoom={timelineZoom}
              onPointerDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                const rect = (event.currentTarget as HTMLDivElement).getBoundingClientRect();
                const hitEdge = rect.right - event.clientX < 12;
                const beatAtCursor = pxToBeat(event.clientX - rect.left, timelineZoom);
                onPlacementPointerDown(entry, lane, beatAtCursor, hitEdge, event);
              }}
            />
          );
        })}
        <div className="track-playhead" style={{ left: beatToPx(currentBeat, timelineZoom) }} />
      </div>
    </div>
  );
}
