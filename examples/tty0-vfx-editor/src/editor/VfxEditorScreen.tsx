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
      sourceKind={documentState.loaded?.source.kind ?? null}
      dirty={documentState.dirty}
      recoveredWorkspace={documentState.recoveredWorkspace}
      hasLoadedDocument={Boolean(documentState.loaded)}
      message={documentState.message}
      messageTone={documentState.messageTone}
      fileInputRef={documentState.fileInputRef}
      audioInputRef={documentState.audioInputRef}
      projects={documentState.projects}
      activeProjectId={documentState.activeProjectId}
      mountedAudio={documentState.mountedAudio}
      audioPath={documentState.loaded?.audioPath ?? null}
      saveDialog={documentState.saveDialog}
      onNewVisualizer={documentActions.handleNewVisualizer}
      onImportRecord={documentActions.handleImportRecord}
      onImportAudio={documentActions.handleImportAudio}
      onSave={() => void documentActions.handleSave()}
      onSaveAs={documentActions.handleSaveAs}
      onExportJson={documentActions.handleExportJson}
      onSelectProject={(projectId) => void documentActions.selectProjectById(projectId)}
      onSaveDialogClose={documentActions.closeSaveDialog}
      onSaveDialogNameChange={documentActions.updateSaveDialogName}
      onSubmitSaveDialog={() => void documentActions.submitSaveDialog()}
      onRevert={() => void documentActions.revertToLoaded()}
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
      leadInBars={arrangementState.timeline.lead_in_bars ?? 0}
      timelineTool={arrangementState.timelineTool}
      hasSelectedClip={clipState.hasSelectedClip}
      selectedPlacementCount={selectionState.selectedPlacementIndices.length}
      onChangeBpm={clipActions.changeTimelineBpm}
      onChangeMeasures={clipActions.changeTimelineMeasures}
      onChangeBeatsPerMeasure={clipActions.changeTimelineBeatsPerMeasure}
      onChangeLeadInBars={clipActions.changeTimelineLeadInBars}
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
        leadInBars: arrangementState.timeline.lead_in_bars ?? 0,
        onSeekToTime: arrangementActions.seekToTime,
        onStop: () => arrangementActions.stopPlayback(0),
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
      onChangeClipStartValue={clipActions.changeClipStartValue}
      onChangeClipEndValue={clipActions.changeClipEndValue}
      onChangeClipHoldAfter={clipActions.changeClipHoldAfter}
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
      className={desktopLayout ? "preview-column-resizable" : "preview-column-grid"}
      panelClassName={desktopLayout ? "preview-panel-resizable" : "preview-panel-grid"}
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
        onPlay={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:play", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
        onPlaying={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:playing", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
        onWaiting={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:waiting", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
        onStalled={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:stalled", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
        onSeeking={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:seeking", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
        onSeeked={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:seeked", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
        onError={() => {
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:error", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
            mediaError: audio?.error?.message ?? audio?.error?.code ?? null,
          });
        }}
        onLoadedMetadata={() => {
          const duration = previewState.audioRef.current?.duration;
          arrangementActions.setAudioDuration(typeof duration === "number" && Number.isFinite(duration) ? duration : null);
          const audio = previewState.audioRef.current;
          console.debug("[tty0-audio]", "event:loadedmetadata", {
            performanceNowMs: Number(performance.now().toFixed(1)),
            currentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
            duration: typeof duration === "number" && Number.isFinite(duration) ? Number(duration.toFixed(3)) : null,
            readyState: audio?.readyState ?? null,
            networkState: audio?.networkState ?? null,
          });
        }}
      />
      <main className={desktopLayout ? "workspace-resizable" : "workspace"}>
        {desktopLayout ? (
          <ResizablePanelGroup orientation="horizontal" className="resizable-shell">
            <ResizablePanel defaultSize="60%" minSize="55%" className="resizable-panel-frame">
              <ResizablePanelGroup orientation="vertical" className="resizable-shell">
                <ResizablePanel defaultSize="50%" minSize="30%" maxSize="50%" className="resizable-panel-frame">
                  <section className="workspace-row workspace-row-resizable">
                    <ResizablePanelGroup orientation="horizontal" className="resizable-shell">
                      <ResizablePanel defaultSize="16%" minSize="16%" className="timeline-sidebar">
                        {sessionPanel}
                        {propertyPanel}
                      </ResizablePanel>
                      <ResizableHandle withHandle />
                      <ResizablePanel defaultSize="84%" minSize="40%" className="resizable-panel-frame">
                        {arrangementPanel}
                      </ResizablePanel>
                    </ResizablePanelGroup>
                  </section>
                </ResizablePanel>
                <ResizableHandle />
                <ResizablePanel defaultSize="50%" minSize="50%" className="resizable-panel-frame">
                  <section className="workspace-row workspace-row-resizable">
                    <ResizablePanelGroup orientation="horizontal" className="resizable-shell">
                      <ResizablePanel defaultSize="16%" minSize="16%" className="resizable-panel-frame">
                        {clipLibraryPanel}
                      </ResizablePanel>
                      <ResizableHandle withHandle />
                      <ResizablePanel defaultSize="84%" minSize="34%" className="resizable-panel-frame">
                        {clipEditorPanel}
                      </ResizablePanel>
                    </ResizablePanelGroup>
                  </section>
                </ResizablePanel>
              </ResizablePanelGroup>
            </ResizablePanel>
            <ResizableHandle withHandle />
            <ResizablePanel defaultSize="40%" minSize="18%" maxSize="40%" className="resizable-panel-frame">
              {previewPanel}
            </ResizablePanel>
          </ResizablePanelGroup>
        ) : (
          <>
            <section className="left-column">
              <section className="workspace-row timeline-row">
                <aside className="timeline-sidebar">
                  {sessionPanel}
                  {propertyPanel}
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
