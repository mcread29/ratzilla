import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ClipPlacement, LaneId, TrackVisualizerConfig } from "../../types";
import { IndexedTimelinePlacement, LANE_ORDER, buildTimelineIndex, totalBeats } from "../../vfx";
import {
  RULER_HEIGHT,
  TIMELINE_LABEL_WIDTH,
  TIMELINE_SNAP_THRESHOLD_PX,
  TIMELINE_TOP_SCROLLBAR_HEIGHT,
} from "../constants";
import { DragState, TimelineTool } from "../editor-types";
import { usePlaybackDisplayTime } from "../hooks/usePlaybackDisplayTime";
import { EDITOR_LANES, laneMeta } from "../utils/lanes";
import { clampBeat, pxToBeat, snapBeatToGrid, snapPlaybackBeat } from "../utils/timelineMath";
import { ArrangementOverviewScrollbar } from "./ArrangementOverviewScrollbar";
import { ArrangementRuler } from "./ArrangementRuler";
import { ArrangementTrackRow } from "./ArrangementTrackRow";

type ScrollbarDrag =
  | { kind: "move"; pointerId: number; startClientX: number; startStartBeat: number; spanBeats: number }
  | { kind: "resize-left"; pointerId: number; startClientX: number; startStartBeat: number; endBeat: number }
  | { kind: "resize-right"; pointerId: number; startClientX: number; startBeat: number; startEndBeat: number };

export function ArrangementGrid({
  audioRef,
  indexedPlacements,
  isPlaying,
  maxTimelineZoom,
  minTimelineZoom,
  onViewportWidthChange,
  onTimelineZoomChange,
  onCommitPlacement,
  onPlaceSelectedClipAtBeat,
  onResetSelection,
  onSeekToBeat,
  onSelectLane,
  onSelectPlacement,
  playbackTimeRef,
  placementsByTrack,
  selectedLane,
  selectedPlacementSet,
  timeline,
  timelineTool,
  timelineWidth,
  timelineZoom,
}: {
  audioRef: { current: HTMLAudioElement | null };
  indexedPlacements: IndexedTimelinePlacement[];
  isPlaying: boolean;
  maxTimelineZoom: number;
  minTimelineZoom: number;
  onViewportWidthChange: (width: number) => void;
  onTimelineZoomChange: (zoom: number | ((current: number) => number)) => void;
  onCommitPlacement: (index: number, next: ClipPlacement) => void;
  onPlaceSelectedClipAtBeat: (beat: number) => void;
  onResetSelection: () => void;
  onSeekToBeat: (beat: number) => void;
  onSelectLane: (lane: LaneId) => void;
  onSelectPlacement: (placementIndex: number, lane: LaneId, clipId: string, event: React.MouseEvent<HTMLDivElement>) => void;
  playbackTimeRef: { current: number };
  placementsByTrack: Map<number, IndexedTimelinePlacement[]>;
  selectedLane: LaneId;
  selectedPlacementSet: Set<number>;
  timeline: NonNullable<TrackVisualizerConfig["timeline"]>;
  timelineTool: TimelineTool;
  timelineWidth: number;
  timelineZoom: number;
}) {
  const arrangementRef = useRef<HTMLDivElement | null>(null);
  const scrollbarDragRef = useRef<ScrollbarDrag | null>(null);
  const [dragState, setDragState] = useState<DragState | null>(null);
  const [previewPlacement, setPreviewPlacement] = useState<{ placement: ClipPlacement; placementIndex: number } | null>(null);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [scrollRegionWidth, setScrollRegionWidth] = useState(0);
  const [displayTime] = usePlaybackDisplayTime(audioRef, playbackTimeRef, isPlaying);
  const totalTimelineBeats = totalBeats(timeline);
  const currentBeat = Math.max(0, displayTime) * timeline.bpm / 60;
  const visibleTimelineWidth = Math.max(1, scrollRegionWidth - TIMELINE_LABEL_WIDTH);
  const maxScrollLeft = Math.max(0, timelineWidth - visibleTimelineWidth);
  const visibleStartBeat = timelineZoom <= 0 ? 0 : scrollLeft / timelineZoom;
  const visibleBeatSpan = timelineZoom <= 0 ? totalTimelineBeats : Math.min(totalTimelineBeats, visibleTimelineWidth / timelineZoom);
  const visibleEndBeat = Math.min(totalTimelineBeats, visibleStartBeat + visibleBeatSpan);
  const overviewPxPerBeat = totalTimelineBeats <= 0 ? 0 : visibleTimelineWidth / totalTimelineBeats;
  const scrollbarThumbWidth = totalTimelineBeats <= 0 ? visibleTimelineWidth : visibleBeatSpan * overviewPxPerBeat;
  const scrollbarThumbLeft = totalTimelineBeats <= 0 ? 0 : visibleStartBeat * overviewPxPerBeat;

  useLayoutEffect(() => {
    const element = arrangementRef.current;
    if (!element) return;

    const updateWidth = () => {
      onViewportWidthChange(element.clientWidth);
      setScrollRegionWidth(element.clientWidth);
    };
    updateWidth();

    const observer = new ResizeObserver(updateWidth);
    observer.observe(element);
    return () => observer.disconnect();
  }, [onViewportWidthChange]);

  useEffect(() => {
    if (previewPlacement && !timeline.arrangement[previewPlacement.placementIndex]) {
      setPreviewPlacement(null);
    }
  }, [previewPlacement, timeline.arrangement]);

  useEffect(() => {
    setScrollLeft((current) => Math.min(current, maxScrollLeft));
  }, [maxScrollLeft]);

  function arrangementBeatFromPointer(clientX: number): number {
    if (!arrangementRef.current) return 0;
    const rect = arrangementRef.current.getBoundingClientRect();
    return pxToBeat(clientX - rect.left + scrollLeft - TIMELINE_LABEL_WIDTH, timelineZoom);
  }

  function clampScrollLeft(next: number): number {
    return Math.max(0, Math.min(maxScrollLeft, next));
  }

  function viewportFromOverview(offset: number): number {
    if (overviewPxPerBeat <= 0) {
      return 0;
    }
    return Math.max(0, Math.min(totalTimelineBeats, offset / overviewPxPerBeat));
  }

  function applyViewport(startBeat: number, endBeat: number) {
    const minVisibleBeats = Math.min(totalTimelineBeats, visibleTimelineWidth / maxTimelineZoom);
    const maxVisibleBeats = Math.min(totalTimelineBeats, visibleTimelineWidth / Math.max(minTimelineZoom, 0.0001));
    const spanBeats = Math.max(minVisibleBeats, Math.min(maxVisibleBeats, endBeat - startBeat));
    const maxStartBeat = Math.max(0, totalTimelineBeats - spanBeats);
    const clampedStartBeat = Math.max(0, Math.min(maxStartBeat, startBeat));
    const nextZoom = Math.max(minTimelineZoom, Math.min(maxTimelineZoom, visibleTimelineWidth / Math.max(spanBeats, 0.0001)));
    onTimelineZoomChange(nextZoom);
    setScrollLeft(clampedStartBeat * nextZoom);
  }

  function snapPlacementStart(beat: number, placementIndex: number): number {
    const thresholdBeats = pxToBeat(TIMELINE_SNAP_THRESHOLD_PX, timelineZoom);
    const clamped = clampBeat(beat, totalTimelineBeats);
    let snapped = snapBeatToGrid(clamped, totalTimelineBeats);
    let closestDistance = thresholdBeats;

    for (const entry of indexedPlacements) {
      if (entry.placementIndex === placementIndex) {
        continue;
      }
      const startDistance = Math.abs(entry.placement.start_beat - clamped);
      if (startDistance <= closestDistance) {
        snapped = entry.placement.start_beat;
        closestDistance = startDistance;
      }
      const endDistance = Math.abs(entry.endBeat - clamped);
      if (endDistance <= closestDistance) {
        snapped = entry.endBeat;
        closestDistance = endDistance;
      }
    }

    return clampBeat(snapped, totalTimelineBeats);
  }

  function commitPreviewPlacement() {
    if (!previewPlacement) {
      setDragState(null);
      return;
    }
    onCommitPlacement(previewPlacement.placementIndex, previewPlacement.placement);
    setPreviewPlacement(null);
    setDragState(null);
  }

  function renderedPlacement(entry: IndexedTimelinePlacement) {
    if (previewPlacement?.placementIndex === entry.placementIndex) {
      return previewPlacement.placement;
    }
    return entry.placement;
  }

  return (
    <div className="arrangement-grid">
      <div className="arrangement-body">
        <div className="arrangement-label-overlay">
          <div className="ruler-spacer" style={{ height: TIMELINE_TOP_SCROLLBAR_HEIGHT + RULER_HEIGHT }} />
          {EDITOR_LANES.map((lane, index) => (
            <button
              key={lane}
              className={`track-label ${selectedLane === lane ? "selected" : ""}`}
              style={{
                top: TIMELINE_TOP_SCROLLBAR_HEIGHT + RULER_HEIGHT + index * 28 - scrollTop,
                height: 28,
              }}
              onClick={() => onSelectLane(lane)}
              type="button"
            >
              {laneMeta(lane).trackLabel}
            </button>
          ))}
        </div>
        <ArrangementOverviewScrollbar
          visibleTimelineWidth={visibleTimelineWidth}
          overviewPxPerBeat={overviewPxPerBeat}
          visibleBeatSpan={visibleBeatSpan}
          visibleStartBeat={visibleStartBeat}
          visibleEndBeat={visibleEndBeat}
          totalTimelineBeats={totalTimelineBeats}
          scrollbarThumbWidth={scrollbarThumbWidth}
          scrollbarThumbLeft={scrollbarThumbLeft}
          scrollbarDragRef={scrollbarDragRef}
          viewportFromOverview={viewportFromOverview}
          applyViewport={applyViewport}
        />
        <div
          ref={arrangementRef}
          className="arrangement-scroll-region"
          style={{ ["--timeline-grid" as never]: `${timelineZoom}px` }}
          onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
          onWheel={(event) => {
            const delta = Math.abs(event.deltaX) > 0 ? event.deltaX : event.shiftKey ? event.deltaY : 0;
            if (delta === 0) {
              return;
            }
            event.preventDefault();
            setScrollLeft((current) => clampScrollLeft(current + delta));
          }}
          onPointerMove={(event) => {
            if (!dragState) return;
            const beat = arrangementBeatFromPointer(event.clientX);
            if (dragState.kind === "scrub") {
              onSeekToBeat(snapPlaybackBeat(beat, totalTimelineBeats));
              return;
            }
            const sourceEntry = indexedPlacements.find((entry) => entry.placementIndex === dragState.placementIndex);
            if (!sourceEntry) return;
            if (dragState.kind === "move") {
              setPreviewPlacement({
                placement: {
                  ...sourceEntry.placement,
                  start_beat: snapPlacementStart(beat - dragState.offsetBeats, dragState.placementIndex),
                },
                placementIndex: dragState.placementIndex,
              });
              return;
            }
            const widthBeats = Math.max(sourceEntry.clip.length_beats, beat - sourceEntry.placement.start_beat);
            setPreviewPlacement({
              placement: {
                ...sourceEntry.placement,
                repeats: Math.max(1, Math.round(widthBeats / sourceEntry.clip.length_beats)),
              },
              placementIndex: dragState.placementIndex,
            });
          }}
          onPointerUp={commitPreviewPlacement}
          onPointerLeave={commitPreviewPlacement}
        >
          <div className="arrangement-surface">
            <ArrangementRuler
              totalTimelineBeats={totalTimelineBeats}
              beatsPerMeasure={timeline.beats_per_measure}
              timelineWidth={timelineWidth}
              timelineZoom={timelineZoom}
              scrollLeft={scrollLeft}
              onPointerDown={(event) => {
                event.preventDefault();
                onResetSelection();
                setDragState({ kind: "scrub" });
                onSeekToBeat(snapPlaybackBeat(arrangementBeatFromPointer(event.clientX), totalTimelineBeats));
              }}
            />
            {EDITOR_LANES.map((lane) => {
              const trackIndex = LANE_ORDER.indexOf(lane);
              const lanePlacements = placementsByTrack.get(trackIndex) ?? [];
              return (
                <ArrangementTrackRow
                  key={lane}
                  lane={lane}
                  trackIndex={trackIndex}
                  lanePlacements={lanePlacements.map((entry) => ({
                    ...entry,
                    placement: renderedPlacement(entry),
                  }))}
                  timelineWidth={timelineWidth}
                  scrollLeft={scrollLeft}
                  timelineZoom={timelineZoom}
                  timelineTool={timelineTool}
                  totalTimelineBeats={totalTimelineBeats}
                  currentBeat={currentBeat}
                  selectedPlacementSet={selectedPlacementSet}
                  onBackgroundPointerDown={(laneValue, event) => {
                    if (event.target !== event.currentTarget) {
                      return;
                    }
                    event.preventDefault();
                    const beat = arrangementBeatFromPointer(event.clientX);
                    if (timelineTool === "pencil") {
                      onPlaceSelectedClipAtBeat(beat);
                      return;
                    }
                    onSelectLane(laneValue);
                    onResetSelection();
                    setDragState({ kind: "scrub" });
                    onSeekToBeat(snapPlaybackBeat(beat, totalTimelineBeats));
                  }}
                  onPlacementPointerDown={(entry, laneValue, beatAtCursor, hitEdge, event) => {
                    onSelectPlacement(entry.placementIndex, laneValue, entry.clip.id, event);
                    if (event.metaKey || event.ctrlKey || event.shiftKey) {
                      return;
                    }
                    setDragState(
                      hitEdge
                        ? { kind: "resize", placementIndex: entry.placementIndex }
                        : { kind: "move", placementIndex: entry.placementIndex, offsetBeats: beatAtCursor },
                    );
                  }}
                />
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
