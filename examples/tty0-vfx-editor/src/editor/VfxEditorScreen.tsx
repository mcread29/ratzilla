import { ArrangementPanel } from "./components/ArrangementPanel";
import { ClipEditorPanel } from "./components/ClipEditorPanel";
import { ClipLibraryPanel } from "./components/ClipLibraryPanel";
import { PreviewPanel } from "./components/PreviewPanel";
import { PropertySidebarPanel } from "./components/PropertySidebarPanel";
import { SessionPanel } from "./components/SessionPanel";
import { useVfxEditorController } from "./controller/useVfxEditorController";

export function VfxEditorScreen() {
  const controller = useVfxEditorController();
  const {
    documentState,
    documentActions,
    selectionState,
    selectionActions,
    arrangementState,
    arrangementActions,
    clipState,
    clipActions,
    shapeState,
    shapeActions,
    previewState,
  } = controller;

  return (
    <div className="app-shell">
      <audio
        ref={previewState.audioRef}
        src={documentState.effectiveAudioUrl ?? undefined}
        preload="auto"
        onCanPlay={() => arrangementActions.setAudioReady(true)}
        onCanPlayThrough={() => arrangementActions.setAudioReady(true)}
        onLoadedMetadata={() => {
          const duration = previewState.audioRef.current?.duration;
          arrangementActions.setAudioDuration(typeof duration === "number" && Number.isFinite(duration) ? duration : null);
        }}
        onPlay={() => arrangementActions.setIsPlaying(true)}
        onPause={() => {
          arrangementActions.setIsPlaying(false);
          previewState.playbackTimeRef.current = previewState.audioRef.current?.currentTime ?? previewState.playbackTimeRef.current;
        }}
        onEnded={() => {
          arrangementActions.setIsPlaying(false);
          previewState.playbackTimeRef.current = 0;
        }}
      />
      <main className="workspace">
        <section className="left-column">
          <section className="workspace-row timeline-row">
            <aside className="timeline-sidebar">
              <PropertySidebarPanel
                selectedLane={selectionState.selectedLane}
                selectedLaneMeta={selectionState.selectedLaneMeta}
                baseState={clipState.baseState}
                onChangeBaseValue={clipActions.changeBaseValue}
              />
              <SessionPanel
                loadedName={documentState.loaded?.name ?? null}
                dirty={documentState.dirty}
                hasLoadedDocument={Boolean(documentState.loaded)}
                message={documentState.message}
                fileInputRef={documentState.fileInputRef}
                audioInputRef={documentState.audioInputRef}
                onNewVisualizer={documentActions.handleNewVisualizer}
                onImportRecord={documentActions.handleImportRecord}
                onImportAudio={documentActions.handleImportAudio}
                onSave={() => {
                  void documentActions.handleSave();
                }}
                onRevert={documentActions.revertToLoaded}
              />
            </aside>

            <ArrangementPanel
              bpm={arrangementState.timeline.bpm}
              measures={arrangementState.timeline.measures}
              beatsPerMeasure={arrangementState.timeline.beats_per_measure}
              timelineTool={arrangementState.timelineTool}
              hasSelectedClip={clipState.hasSelectedClip}
              selectedPlacementCount={selectionState.selectedPlacementIndices.length}
              onChangeBpm={clipActions.changeTimelineBpm}
              onChangeMeasures={clipActions.changeTimelineMeasures}
              onChangeBeatsPerMeasure={clipActions.changeTimelineBeatsPerMeasure}
              onChangeTimelineTool={arrangementActions.setTimelineTool}
              onAddPlacement={arrangementActions.addPlacement}
              onCopyPlacements={arrangementActions.copySelectedPlacements}
              onDeletePlacements={arrangementActions.deleteSelectedPlacements}
              gridProps={{
                audioRef: previewState.audioRef,
                indexedPlacements: arrangementState.timelineIndex.placements,
                isPlaying: arrangementState.isPlaying,
                maxTimelineZoom: arrangementState.maxTimelineZoom,
                minTimelineZoom: arrangementState.minTimelineZoom,
                onViewportWidthChange: arrangementActions.setArrangementViewportWidth,
                onTimelineZoomChange: arrangementActions.setTimelineZoom,
                onCommitPlacement: arrangementActions.updatePlacementAtIndex,
                onPlaceSelectedClipAtBeat: arrangementActions.placeSelectedClipAtBeat,
                onResetSelection: selectionActions.resetArrangementSelection,
                onSeekToBeat: arrangementActions.seekToBeat,
                onSelectLane: selectionActions.selectLane,
                onSelectPlacement: selectionActions.selectPlacement,
                playbackTimeRef: previewState.playbackTimeRef,
                placementsByTrack: arrangementState.timelineIndex.placementsByTrack,
                selectedLane: selectionState.selectedLane,
                selectedPlacementSet: selectionState.selectedPlacementSet,
                timeline: arrangementState.timeline,
                timelineTool: arrangementState.timelineTool,
                timelineWidth: arrangementState.timelineWidth,
                timelineZoom: arrangementState.timelineZoom,
              }}
              transportProps={{
                audioRef: previewState.audioRef,
                beatsPerMeasure: arrangementState.timeline.beats_per_measure,
                dirty: documentState.dirty,
                effectiveAudioUrl: documentState.effectiveAudioUrl,
                isPlaying: arrangementState.isPlaying,
                onSeekToTime: (nextTime) => {
                  previewState.playbackTimeRef.current = nextTime;
                  if (previewState.audioRef.current) {
                    previewState.audioRef.current.currentTime = nextTime;
                  }
                },
                onStop: () => {
                  if (!previewState.audioRef.current) return;
                  previewState.audioRef.current.pause();
                  previewState.audioRef.current.currentTime = 0;
                  previewState.playbackTimeRef.current = 0;
                },
                onTogglePlayback: arrangementActions.handleTogglePlayback,
                playPending: arrangementState.playPending,
                playbackTimeRef: previewState.playbackTimeRef,
                timelineBpm: arrangementState.timeline.bpm,
                totalDurationSeconds: arrangementState.totalDurationSeconds,
              }}
            />
          </section>

          <section className="workspace-row clip-row">
            <ClipLibraryPanel
              laneClips={clipState.laneClips}
              selectedClipId={selectionState.selectedClip?.id ?? null}
              selectedLane={selectionState.selectedLane}
              timelineBeatsPerMeasure={arrangementState.timeline.beats_per_measure}
              draggedClipId={selectionState.draggedClipId}
              clipDropIndicator={selectionState.clipDropIndicator}
              onSelectClip={selectionActions.selectClip}
              onAddClip={clipActions.addClip}
              onDuplicateClip={clipActions.duplicateClip}
              onDeleteClip={clipActions.deleteClip}
              onClipDragStart={clipActions.handleClipDragStart}
              onClipDragOver={clipActions.handleClipDragOver}
              onClipDrop={clipActions.handleClipDrop}
              onClearClipDragState={clipActions.clearClipDragState}
              hasSelectedClip={clipState.hasSelectedClip}
            />

            <ClipEditorPanel
              selectedClip={selectionState.selectedClip}
              selectedShape={shapeState.selectedShape}
              selectedPointIndex={shapeState.selectedPointIndex}
              selectedClipBeatOption={clipState.selectedClipBeatOption}
              selectedClipBeatValue={clipState.selectedClipBeatValue}
              clipLengthOptions={clipState.clipLengthOptions}
              onSelectPoint={shapeActions.setSelectedPointIndex}
              onCommitShape={shapeActions.commitSelectedClipShape}
              onChangeClipName={clipActions.changeClipName}
              onChangeClipBeatValue={clipActions.updateSelectedClipBeatValue}
              onChangeClipMin={clipActions.changeClipMin}
              onChangeClipMax={clipActions.changeClipMax}
            />
          </section>
        </section>

        <PreviewPanel
          config={documentState.draft}
          audioRef={previewState.audioRef}
          playbackTimeRef={previewState.playbackTimeRef}
          isPlaying={previewState.isPlaying}
          onReady={() => arrangementActions.setPreviewReady(true)}
          timelineIndex={arrangementState.timelineIndex}
        />
      </main>
    </div>
  );
}
