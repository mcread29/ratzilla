import { LaneId } from "../../types";
import { buildTimelineIndex } from "../../vfx";
import { TimelineTool } from "../editor-types";
import { ArrangementGrid } from "./ArrangementGrid";
import { ArrangementToolbar } from "./ArrangementToolbar";
import { PlaybackTransport } from "./PlaybackTransport";

export function ArrangementPanel({
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
  gridProps,
  transportProps,
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
  gridProps: {
    audioRef: { current: HTMLAudioElement | null };
    indexedPlacements: ReturnType<typeof buildTimelineIndex>["placements"];
    isPlaying: boolean;
    maxTimelineZoom: number;
    minTimelineZoom: number;
    onViewportWidthChange: (width: number) => void;
    onTimelineZoomChange: (zoom: number | ((current: number) => number)) => void;
    onCommitPlacement: (index: number, next: import("../../types").ClipPlacement) => void;
    onPlaceSelectedClipAtBeat: (beat: number) => void;
    onResetSelection: () => void;
    onSeekToBeat: (beat: number) => void;
    onSelectLane: (lane: LaneId) => void;
    onSelectPlacement: (placementIndex: number, lane: LaneId, clipId: string, event: React.MouseEvent<HTMLDivElement>) => void;
    playbackTimeRef: { current: number };
    placementsByTrack: ReturnType<typeof buildTimelineIndex>["placementsByTrack"];
    selectedLane: LaneId;
    selectedPlacementSet: Set<number>;
    timeline: NonNullable<import("../../types").TrackVisualizerConfig["timeline"]>;
    timelineTool: TimelineTool;
    timelineWidth: number;
    timelineZoom: number;
  };
  transportProps: React.ComponentProps<typeof PlaybackTransport>;
}) {
  return (
    <section className="panel arrangement-panel">
      <ArrangementToolbar
        bpm={bpm}
        measures={measures}
        beatsPerMeasure={beatsPerMeasure}
        timelineTool={timelineTool}
        hasSelectedClip={hasSelectedClip}
        selectedPlacementCount={selectedPlacementCount}
        onChangeBpm={onChangeBpm}
        onChangeMeasures={onChangeMeasures}
        onChangeBeatsPerMeasure={onChangeBeatsPerMeasure}
        onChangeTimelineTool={onChangeTimelineTool}
        onAddPlacement={onAddPlacement}
        onCopyPlacements={onCopyPlacements}
        onDeletePlacements={onDeletePlacements}
      />
      <ArrangementGrid {...gridProps} />
      <PlaybackTransport {...transportProps} />
    </section>
  );
}
