import { useEffect, useState } from "react";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ArrangementPanel } from "./components/ArrangementPanel";
import { ClipEditorPanel } from "./components/ClipEditorPanel";
import { ClipLibraryPanel } from "./components/ClipLibraryPanel";
import { PreviewPanel } from "./components/PreviewPanel";
import { PropertySidebarPanel } from "./components/PropertySidebarPanel";
import { SessionPanel } from "./components/SessionPanel";
import { useVfxEditorController } from "./controller/useVfxEditorController";

export function VfxEditorScreen() {
  const [desktopLayout, setDesktopLayout] = useState(() => window.innerWidth > 1200);
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

  useEffect(() => {
    const mediaQuery = window.matchMedia("(min-width: 1201px)");
    const updateLayout = () => setDesktopLayout(mediaQuery.matches);

    updateLayout();
    mediaQuery.addEventListener("change", updateLayout);
    return () => mediaQuery.removeEventListener("change", updateLayout);
  }, []);

  const sessionPanel = (
    <SessionPanel
      loadedName={documentState.loaded?.name ?? null}
      dirty={documentState.dirty}
      hasLoadedDocument={Boolean(documentState.loaded)}
      message={documentState.message}
      messageTone={documentState.messageTone}
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
  );

  const propertyPanel = (
    <PropertySidebarPanel
      selectedLane={selectionState.selectedLane}
      selectedLaneMeta={selectionState.selectedLaneMeta}
      baseState={clipState.baseState}
      onChangeBaseValue={clipActions.changeBaseValue}
    />
  );

  const arrangementPanel = (
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
  );

  const clipLibraryPanel = (
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
      selectedClipName={selectionState.selectedClip?.name ?? null}
    />
  );

  const clipEditorPanel = (
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
  );

  const previewPanel = (
    <PreviewPanel
      config={documentState.draft}
      audioRef={previewState.audioRef}
      playbackTimeRef={previewState.playbackTimeRef}
      isPlaying={previewState.isPlaying}
      onReady={() => arrangementActions.setPreviewReady(true)}
      timelineIndex={arrangementState.timelineIndex}
    />
  );

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
      <main className={desktopLayout ? "workspace-resizable" : "workspace"}>
        {desktopLayout ? (
          <ResizablePanelGroup orientation="horizontal" className="resizable-shell">
            <ResizablePanel defaultSize={74} minSize={55}>
              <ResizablePanelGroup orientation="vertical" className="resizable-shell">
                <ResizablePanel defaultSize={58} minSize={38}>
                  <section className="workspace-row timeline-row">
                    <ResizablePanelGroup orientation="horizontal" className="resizable-shell">
                      <ResizablePanel defaultSize={28} minSize={18} className="timeline-sidebar">
                        {propertyPanel}
                        {sessionPanel}
                      </ResizablePanel>
                      <ResizableHandle withHandle />
                      <ResizablePanel defaultSize={72} minSize={40}>
                        {arrangementPanel}
                      </ResizablePanel>
                    </ResizablePanelGroup>
                  </section>
                </ResizablePanel>
                <ResizableHandle />
                <ResizablePanel defaultSize={42} minSize={26}>
                  <section className="workspace-row clip-row">
                    <ResizablePanelGroup orientation="horizontal" className="resizable-shell">
                      <ResizablePanel defaultSize={26} minSize={16}>
                        {clipLibraryPanel}
                      </ResizablePanel>
                      <ResizableHandle withHandle />
                      <ResizablePanel defaultSize={74} minSize={34}>
                        {clipEditorPanel}
                      </ResizablePanel>
                    </ResizablePanelGroup>
                  </section>
                </ResizablePanel>
              </ResizablePanelGroup>
            </ResizablePanel>
            <ResizableHandle withHandle />
            <ResizablePanel defaultSize={26} minSize={18} maxSize={40}>
              {previewPanel}
            </ResizablePanel>
          </ResizablePanelGroup>
        ) : (
          <>
            <section className="left-column">
              <section className="workspace-row timeline-row">
                <aside className="timeline-sidebar">
                  {propertyPanel}
                  {sessionPanel}
                </aside>
                {arrangementPanel}
              </section>
              <section className="workspace-row clip-row">
                {clipLibraryPanel}
                {clipEditorPanel}
              </section>
            </section>
            {previewPanel}
          </>
        )}
      </main>
    </div>
  );
}
