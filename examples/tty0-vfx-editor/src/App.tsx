import { ChangeEvent, MouseEvent, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  ChromaticBulgeGridShaderState,
  ClipPlacement,
  ClipTweenStep,
  LaneId,
  LfoPoint,
  TrackVisualizerConfig,
} from "./types";
import {
  clipSummary,
  defaultLfoClip,
  createPlacement,
  defaultStep,
  defaultVisualizer,
  isColorLane,
  isLegacyClip,
  LANE_ORDER,
  normalizedConfig,
  pointLabel,
  primaryLane,
  resolveNormalizedChromaticBulgeGrid,
  shapePath,
  syncClipAuthoring,
  timelineFromConfig,
  totalBeats,
} from "./vfx";
import {
  importRecordFromJson,
} from "./platform";
import { FRAGMENT_SHADER, VERTEX_SHADER } from "./shaders";

type DragState =
  | { kind: "move"; placementIndex: number; offsetBeats: number }
  | { kind: "resize"; placementIndex: number }
  | { kind: "scrub" };

type TimelineTool = "select" | "pencil";

type PlacementClipboardEntry = {
  clipId: string;
  offsetBeats: number;
  repeats: number;
};

const TRACK_HEIGHT = 28;
const TIMELINE_SNAP_DIVISION = 4;
const TIMELINE_SNAP_THRESHOLD_PX = 12;
const PLAYHEAD_SNAP_DIVISION = 1;
const SHAPE_EDITOR_WIDTH = 320;
const SHAPE_EDITOR_HEIGHT = 180;
const SHAPE_EDITOR_VERTICAL_PADDING = 10;
const SHAPE_EDITOR_BOUND_INSET = 2;
const HIDDEN_EDITOR_LANES = new Set<LaneId>(["color_cycle_rate"]);
const EDITOR_LANES = LANE_ORDER.filter((lane) => !HIDDEN_EDITOR_LANES.has(lane));

type LoadedDocument = {
  audioUrl: string | null;
  name: string;
  visualizer: TrackVisualizerConfig;
};

export default function App() {
  const [loaded, setLoaded] = useState<LoadedDocument | null>(null);
  const [draft, setDraft] = useState<TrackVisualizerConfig>(defaultVisualizer());
  const [selectedLane, setSelectedLane] = useState<LaneId>("motion_rate");
  const [selectedClipId, setSelectedClipId] = useState<string | null>(null);
  const [selectedPointIndex, setSelectedPointIndex] = useState<number | null>(null);
  const [selectedPlacementIndices, setSelectedPlacementIndices] = useState<number[]>([]);
  const [placementSelectionAnchor, setPlacementSelectionAnchor] = useState<number | null>(null);
  const [selectedStepIndices, setSelectedStepIndices] = useState<number[]>([]);
  const [stepSelectionAnchor, setStepSelectionAnchor] = useState<number | null>(null);
  const [message, setMessage] = useState<string>("Load a visualizer JSON or start a new effect.");
  const [playbackTime, setPlaybackTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [previewReady, setPreviewReady] = useState(false);
  const [audioReady, setAudioReady] = useState(false);
  const [playPending, setPlayPending] = useState(false);
  const [clipEditorHeight, setClipEditorHeight] = useState<number | null>(null);
  const [timelineZoom, setTimelineZoom] = useState(32);
  const [timelineTool, setTimelineTool] = useState<TimelineTool>("select");
  const [dragState, setDragState] = useState<DragState | null>(null);
  const [shapeDragIndex, setShapeDragIndex] = useState<number | null>(null);
  const [placementClipboard, setPlacementClipboard] = useState<PlacementClipboardEntry[]>([]);
  const [importedAudioUrl, setImportedAudioUrl] = useState<string | null>(null);
  const playbackTimeRef = useRef(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const arrangementRef = useRef<HTMLDivElement | null>(null);
  const clipFieldsRef = useRef<HTMLDivElement | null>(null);
  const shapeSvgRef = useRef<SVGSVGElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const audioInputRef = useRef<HTMLInputElement | null>(null);

  const normalizedDraft = useMemo(() => normalizedConfig(draft), [draft]);
  const timeline = useMemo(() => timelineFromConfig(normalizedDraft), [normalizedDraft]);
  const totalTimelineBeats = totalBeats(timeline);
  const timelineWidth = Math.max(960, totalTimelineBeats * timelineZoom);
  const laneClips = useMemo(
    () => timeline.clips.filter((clip) => primaryLane(clip)?.lane === selectedLane),
    [timeline.clips, selectedLane],
  );
  const selectedClip =
    laneClips.find((clip) => clip.id === selectedClipId) ??
    timeline.clips.find((clip) => clip.id === selectedClipId) ??
    laneClips[0] ??
    null;
  const selectedShape = selectedClip?.source?.kind === "lfo" ? selectedClip.source.shape : null;
  const selectedTrack = selectedClip ? editableTrack(selectedClip, selectedLane) : null;
  const selectedSteps = selectedTrack?.steps ?? [];
  const selectedPlacementSet = useMemo(() => new Set(selectedPlacementIndices), [selectedPlacementIndices]);
  const selectedStepSet = useMemo(() => new Set(selectedStepIndices), [selectedStepIndices]);
  const selectedStep =
    selectedStepIndices.length === 1
      ? selectedTrack?.steps?.[selectedStepIndices[0] ?? 0] ?? null
      : null;
  const leadSelectedStep =
    selectedStepIndices.length > 0
      ? selectedTrack?.steps?.[selectedStepIndices[0] ?? 0] ?? null
      : null;
  const batchStepTargetKind =
    selectedStepIndices.length > 0
      ? selectedTrack?.steps?.[selectedStepIndices[0] ?? 0]?.to.kind ?? null
      : null;
  const normalizedLoaded = useMemo(
    () => (loaded ? normalizedConfig(loaded.visualizer) : null),
    [loaded],
  );
  const dirty = useMemo(
    () => (normalizedLoaded ? JSON.stringify(normalizedDraft) !== JSON.stringify(normalizedLoaded) : false),
    [normalizedDraft, normalizedLoaded],
  );
  const effectiveAudioUrl = importedAudioUrl ?? loaded?.audioUrl ?? null;
  const baseState = draft.params.shader_states?.playing;
  const selectedLaneMeta = laneMeta(selectedLane);
  const audioDuration = audioRef.current?.duration;
  const totalDurationSeconds =
    typeof audioDuration === "number" && Number.isFinite(audioDuration) && audioDuration > 0
      ? audioDuration
      : (totalTimelineBeats * 60) / timeline.bpm;
  const documentName = loaded?.name ?? "tty0-visualizer.json";
  const uiCurrentBeat = useMemo(
    () => Math.max(0, playbackTime) * timeline.bpm / 60,
    [playbackTime, timeline.bpm],
  );

  useEffect(() => {
    setSelectedStepIndices(selectedTrack?.steps?.length ? [0] : []);
    setStepSelectionAnchor(selectedTrack?.steps?.length ? 0 : null);
  }, [selectedClipId, selectedLane]);

  useEffect(() => {
    setSelectedPointIndex(selectedShape?.points.length ? 0 : null);
  }, [selectedClip?.id]);

  useEffect(() => {
    return () => {
      if (importedAudioUrl) {
        URL.revokeObjectURL(importedAudioUrl);
      }
    };
  }, [importedAudioUrl]);

  useEffect(() => {
    setAudioReady(!effectiveAudioUrl);
    setPlayPending(false);
    if (audioRef.current) {
      audioRef.current.pause();
    }
  }, [effectiveAudioUrl]);

  useLayoutEffect(() => {
    const element = clipFieldsRef.current;
    if (!element) return;

    const updateHeight = () => {
      setClipEditorHeight(Math.ceil(element.getBoundingClientRect().height));
    };

    updateHeight();
    const observer = new ResizeObserver(updateHeight);
    observer.observe(element);
    return () => observer.disconnect();
  }, [selectedClip?.id, selectedLane, draft]);

  useEffect(() => {
    let frame = 0;
    let lastPublishedTime = playbackTimeRef.current;
    const tick = () => {
      if (audioRef.current && isPlaying) {
        const nextTime = audioRef.current.currentTime;
        playbackTimeRef.current = nextTime;
        if (Math.abs(nextTime - lastPublishedTime) >= 1 / 30) {
          lastPublishedTime = nextTime;
          setPlaybackTime(nextTime);
        }
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [isPlaying]);

  useEffect(() => {
    if (!playPending || !previewReady || !audioReady || !audioRef.current) return;
    const run = async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (!audioRef.current) return;
      void audioRef.current.play();
      setPlayPending(false);
    };
    void run();
  }, [audioReady, playPending, previewReady]);

  useEffect(() => {
    const next = selectedPlacementIndices.filter((index) => index >= 0 && index < timeline.arrangement.length);
    if (next.length === selectedPlacementIndices.length) {
      return;
    }
    setSelectedPlacementIndices(next);
    setPlacementSelectionAnchor(next[0] ?? null);
  }, [selectedPlacementIndices, timeline.arrangement.length]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (
        target?.closest("input, textarea, select") ||
        target?.isContentEditable
      ) {
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "c" && selectedPlacementIndices.length) {
        event.preventDefault();
        copySelectedPlacements();
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "v" && placementClipboard.length) {
        event.preventDefault();
        pasteCopiedPlacements();
        return;
      }
      if ((event.key === "Delete" || event.key === "Backspace") && selectedPlacementIndices.length) {
        event.preventDefault();
        deleteSelectedPlacements();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [placementClipboard, selectedPlacementIndices]);

  function loadDocument(
    visualizer: TrackVisualizerConfig,
    options: {
      audioUrl?: string | null;
      name: string;
      status: string;
    },
  ) {
    const normalized = normalizedConfig(visualizer);
    const nextTimeline = timelineFromConfig(normalized);
    setLoaded({
      audioUrl: options.audioUrl ?? null,
      name: options.name,
      visualizer: normalized,
    });
    setDraft(normalized);
    setSelectedClipId(nextTimeline.clips[0]?.id ?? null);
    clearPlacementSelection();
    setSelectedStepIndices([]);
    setStepSelectionAnchor(null);
    setSelectedLane(visibleLane(primaryLane(nextTimeline.clips[0])?.lane));
    playbackTimeRef.current = 0;
    setPlaybackTime(0);
    setIsPlaying(false);
    setImportedAudioUrl(null);
    setMessage(options.status);
  }

  async function handleImportRecord(event: ChangeEvent<HTMLInputElement>) {
    const input = event.target;
    const file = input.files?.[0];
    if (!file) return;
    try {
      const imported = await importRecordFromJson(file);
      if (imported.kind === "record") {
        loadDocument(imported.record.visualizer, {
          audioUrl: imported.record.audio_url ?? null,
          name: `${imported.record.record_id}.visualizer.json`,
          status: imported.record.has_legacy_automation
            ? `Loaded ${imported.record.record_id} and migrated legacy automation into the timeline draft.`
            : `Loaded ${imported.record.record_id} from record JSON.`,
        });
      } else {
        loadDocument(imported.visualizer, {
          name: imported.sourceName,
          status: `Loaded ${imported.sourceName}.`,
        });
      }
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Load failed.");
    } finally {
      input.value = "";
    }
  }

  function handleNewVisualizer() {
    const visualizer = defaultVisualizer();
    setLoaded({
      audioUrl: null,
      name: "tty0-visualizer.json",
      visualizer,
    });
    setDraft(visualizer);
    setSelectedLane("motion_rate");
    setSelectedClipId(timelineFromConfig(visualizer).clips[0]?.id ?? null);
    clearPlacementSelection();
    setSelectedStepIndices([]);
    setStepSelectionAnchor(null);
    playbackTimeRef.current = 0;
    setPlaybackTime(0);
    setIsPlaying(false);
    setImportedAudioUrl(null);
    setMessage("Started a new visualizer draft.");
  }

  function handleImportAudio(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    if (importedAudioUrl) {
      URL.revokeObjectURL(importedAudioUrl);
    }
    const url = URL.createObjectURL(file);
    setImportedAudioUrl(url);
    setMessage(`Mounted audio file ${file.name}.`);
  }

  function updateSelectedClipShape(mutator: (points: LfoPoint[]) => void) {
    if (!selectedClip || !selectedClip.source || selectedClip.source.kind !== "lfo") return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.source || clip.source.kind !== "lfo") return current;
      mutator(clip.source.shape.points);
      current.timeline = nextTimeline;
      return current;
    });
  }

  async function handleSave() {
    const normalized = normalizedConfig(draft);
    const blob = new Blob([JSON.stringify(normalized, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = jsonFilename(loaded?.name);
    anchor.click();
    URL.revokeObjectURL(url);
    setLoaded((current) => ({
      audioUrl: current?.audioUrl ?? null,
      name: jsonFilename(current?.name),
      visualizer: normalized,
    }));
    setMessage(`Saved ${jsonFilename(loaded?.name)}.`);
  }

  function updateDraft(mutator: (current: TrackVisualizerConfig) => TrackVisualizerConfig) {
    setDraft((current) => normalizedConfig(mutator(structuredClone(current))));
  }

  function currentBeat(): number {
    return uiCurrentBeat;
  }

  function seekToBeat(beat: number) {
    const nextBeat = Math.max(0, Math.min(totalTimelineBeats, beat));
    const nextTime = nextBeat * 60 / timeline.bpm;
    playbackTimeRef.current = nextTime;
    setPlaybackTime(nextTime);
    if (audioRef.current) {
      audioRef.current.currentTime = nextTime;
    }
  }

  function beatToPx(beat: number): number {
    return Math.max(0, beat) * timelineZoom;
  }

  function pxToBeat(px: number): number {
    return Math.max(0, px / timelineZoom);
  }

  function clampBeat(beat: number): number {
    return Math.max(0, Math.min(totalTimelineBeats, beat));
  }

  function snapBeatToGrid(beat: number): number {
    return Math.round(clampBeat(beat) * TIMELINE_SNAP_DIVISION) / TIMELINE_SNAP_DIVISION;
  }

  function snapPlaybackBeat(beat: number): number {
    return Math.round(clampBeat(beat) * PLAYHEAD_SNAP_DIVISION) / PLAYHEAD_SNAP_DIVISION;
  }

  function placementLengthBeats(placement: ClipPlacement): number {
    const clip = timeline.clips.find((candidate) => candidate.id === placement.clip_id);
    return clip ? clip.length_beats * placement.repeats : 0;
  }

  function snapPlacementStart(beat: number, placementIndex: number): number {
    const thresholdBeats = pxToBeat(TIMELINE_SNAP_THRESHOLD_PX);
    const clampedBeat = clampBeat(beat);
    let snappedBeat = snapBeatToGrid(clampedBeat);
    let closestDistance = thresholdBeats;

    for (const [index, placement] of timeline.arrangement.entries()) {
      if (index === placementIndex) continue;
      const startDistance = Math.abs(placement.start_beat - clampedBeat);
      if (startDistance <= closestDistance) {
        snappedBeat = placement.start_beat;
        closestDistance = startDistance;
      }

      const endBeat = placement.start_beat + placementLengthBeats(placement);
      const endDistance = Math.abs(endBeat - clampedBeat);
      if (endDistance <= closestDistance) {
        snappedBeat = endBeat;
        closestDistance = endDistance;
      }
    }

    return clampBeat(snappedBeat);
  }

  function arrangementBeatFromPointer(clientX: number): number {
    if (!arrangementRef.current) return 0;
    const rect = arrangementRef.current.getBoundingClientRect();
    return pxToBeat(clientX - rect.left + arrangementRef.current.scrollLeft - 160);
  }

  function setPlacementSelection(indices: number[], anchor: number | null = indices[0] ?? null) {
    const next = [...new Set(indices)].filter((index) => index >= 0).sort((a, b) => a - b);
    setSelectedPlacementIndices(next);
    setPlacementSelectionAnchor(anchor ?? next[0] ?? null);
  }

  function clearPlacementSelection() {
    setPlacementSelection([]);
  }

  function setSinglePlacementSelection(index: number | null) {
    setPlacementSelection(index == null ? [] : [index], index);
  }

  function selectPlacement(
    placementIndex: number,
    lane: LaneId,
    clipId: string,
    event: MouseEvent<HTMLDivElement>,
  ) {
    if (event.shiftKey && placementSelectionAnchor != null) {
      const start = Math.min(placementSelectionAnchor, placementIndex);
      const end = Math.max(placementSelectionAnchor, placementIndex);
      setPlacementSelection(
        Array.from({ length: end - start + 1 }, (_, offset) => start + offset),
        placementSelectionAnchor,
      );
    } else if (event.metaKey || event.ctrlKey) {
      const next = selectedPlacementSet.has(placementIndex)
        ? selectedPlacementIndices.filter((index) => index !== placementIndex)
        : [...selectedPlacementIndices, placementIndex];
      setPlacementSelection(next, placementIndex);
    } else if (!selectedPlacementSet.has(placementIndex) || selectedPlacementIndices.length > 1) {
      setSinglePlacementSelection(placementIndex);
    }
    setSelectedLane(lane);
    setSelectedClipId(clipId);
    setSelectedStepIndices([]);
    setStepSelectionAnchor(null);
  }

  function handleTogglePlayback() {
    if (!audioRef.current) return;
    if (!audioRef.current.paused) {
      audioRef.current.pause();
      setPlayPending(false);
      return;
    }
    if (!previewReady || !audioReady) {
      setPlayPending(true);
      setMessage("Preparing preview and audio before playback.");
      return;
    }
    void audioRef.current.play();
  }

  function addClip() {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const base = current.params.shader_states?.playing ?? defaultVisualizer().params.shader_states!.playing;
      const clipId = `clip_${nextTimeline.clips.length + 1}`;
      nextTimeline.clips.push({
        id: clipId,
        name: `Clip ${nextTimeline.clips.length + 1}`,
        length_beats: 4,
        color: [0.43, 0.86, 0.83],
        source: defaultLfoClip(selectedLane, base),
        authoring: undefined,
        lanes: {},
      });
      current.timeline = nextTimeline;
      setSelectedClipId(clipId);
      setSelectedStepIndices([]);
      setStepSelectionAnchor(null);
      return current;
    });
  }

  function duplicateClip() {
    if (!selectedClip) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const newClip = structuredClone(selectedClip);
      newClip.id = `${selectedClip.id}_copy_${Date.now().toString(36)}`;
      newClip.name = `${selectedClip.name} Copy`;
      nextTimeline.clips.push(newClip);
      current.timeline = nextTimeline;
      setSelectedClipId(newClip.id);
      setSelectedLane(visibleLane(primaryLane(newClip)?.lane ?? selectedLane));
      return current;
    });
  }

  function deleteClip() {
    if (!selectedClip) return;
    if (timeline.arrangement.some((placement) => placement.clip_id === selectedClip.id)) {
      setMessage("Delete blocked: clip is still placed on the song timeline.");
      return;
    }
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.clips = nextTimeline.clips.filter((clip) => clip.id !== selectedClip.id);
      current.timeline = nextTimeline;
      setSelectedClipId(
        nextTimeline.clips.find((clip) => primaryLane(clip)?.lane === selectedLane)?.id ??
          nextTimeline.clips[0]?.id ??
          null,
      );
      return current;
    });
  }

  function placeSelectedClipAtBeat(beat: number) {
    if (!selectedClip) return;
    const primary = primaryLane(selectedClip);
    if (!primary) {
      setMessage("Selected clip has no authored lane.");
      return;
    }
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.arrangement.push(createPlacement(selectedClip, snapBeatToGrid(beat)));
      current.timeline = nextTimeline;
      setSinglePlacementSelection(nextTimeline.arrangement.length - 1);
      setSelectedLane(primary.lane);
      setSelectedClipId(selectedClip.id);
      return current;
    });
  }

  function addPlacement() {
    placeSelectedClipAtBeat(currentBeat());
  }

  function deleteSelectedPlacements() {
    if (!selectedPlacementIndices.length) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const indices = [...new Set(selectedPlacementIndices)]
        .filter((index) => index >= 0 && index < nextTimeline.arrangement.length)
        .sort((a, b) => b - a);
      indices.forEach((index) => {
        nextTimeline.arrangement.splice(index, 1);
      });
      current.timeline = nextTimeline;
      clearPlacementSelection();
      return current;
    });
  }

  function updatePlacementAtIndex(index: number, next: ClipPlacement) {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      if (!nextTimeline.arrangement[index]) {
        return current;
      }
      nextTimeline.arrangement[index] = next;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function copySelectedPlacements() {
    if (!selectedPlacementIndices.length) return;
    const placements = [...new Set(selectedPlacementIndices)]
      .map((index) => timeline.arrangement[index])
      .filter(Boolean)
      .sort((left, right) => left.start_beat - right.start_beat);
    if (!placements.length) return;
    const startBeat = placements[0].start_beat;
    setPlacementClipboard(
      placements.map((placement) => ({
        clipId: placement.clip_id,
        offsetBeats: placement.start_beat - startBeat,
        repeats: placement.repeats,
      })),
    );
    setMessage(`Copied ${placements.length} placement${placements.length === 1 ? "" : "s"}.`);
  }

  function pasteCopiedPlacements(atBeat = currentBeat()) {
    if (!placementClipboard.length) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const pastedIndices: number[] = [];
      for (const entry of placementClipboard) {
        const clip = nextTimeline.clips.find((candidate) => candidate.id === entry.clipId);
        if (!clip) {
          continue;
        }
        nextTimeline.arrangement.push({
          ...createPlacement(clip, snapBeatToGrid(atBeat + entry.offsetBeats)),
          repeats: entry.repeats,
        });
        pastedIndices.push(nextTimeline.arrangement.length - 1);
      }
      current.timeline = nextTimeline;
      setPlacementSelection(pastedIndices, pastedIndices[0] ?? null);
      if (pastedIndices.length) {
        const clip = nextTimeline.clips.find((candidate) => candidate.id === nextTimeline.arrangement[pastedIndices[0]].clip_id);
        setSelectedClipId(clip?.id ?? null);
        if (clip) {
          setSelectedLane(visibleLane(primaryLane(clip)?.lane ?? selectedLane));
        }
        setMessage(`Pasted ${pastedIndices.length} placement${pastedIndices.length === 1 ? "" : "s"}.`);
      }
      return current;
    });
  }

  function setSingleStepSelection(index: number | null) {
    setSelectedStepIndices(index == null ? [] : [index]);
    setStepSelectionAnchor(index);
  }

  function selectStep(index: number, event: MouseEvent<HTMLButtonElement>) {
    if (!selectedTrack) return;
    if (event.shiftKey && stepSelectionAnchor != null) {
      const start = Math.min(stepSelectionAnchor, index);
      const end = Math.max(stepSelectionAnchor, index);
      setSelectedStepIndices(Array.from({ length: end - start + 1 }, (_, offset) => start + offset));
      return;
    }
    if (event.metaKey || event.ctrlKey) {
      setSelectedStepIndices((current) => {
        const next = current.includes(index)
          ? current.filter((value) => value !== index)
          : [...current, index].sort((a, b) => a - b);
        return next;
      });
      setStepSelectionAnchor(index);
      return;
    }
    setSingleStepSelection(index);
  }

  function updateSelectedSteps(mutator: (step: ClipTweenStep, index: number) => ClipTweenStep) {
    if (!selectedClip || !selectedTrack || !selectedStepIndices.length) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip) return current;
      const track = ensureClipTrack(clip, selectedLane);
      if (!track) return current;
      selectedStepIndices.forEach((index) => {
        if (track.steps[index]) {
          track.steps[index] = mutator(track.steps[index], index);
        }
      });
      current.timeline = nextTimeline;
      return syncClipAuthoring(current, clip.id);
    });
  }

  function addStep() {
    if (!selectedClip) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip) return current;
      const track = ensureClipTrack(clip, selectedLane);
      if (!track) return current;
      track.steps.push(defaultStep(track.lane));
      current.timeline = nextTimeline;
      setSingleStepSelection(track.steps.length - 1);
      return syncClipAuthoring(current, clip.id);
    });
  }

  function copySelectedSteps() {
    if (!selectedClip || !selectedTrack || !selectedStepIndices.length) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip) return current;
      const track = ensureClipTrack(clip, selectedLane);
      if (!track) return current;
      const indices = [...new Set(selectedStepIndices)]
        .filter((index) => index >= 0 && index < track.steps.length)
        .sort((a, b) => a - b);
      if (!indices.length) return current;
      const insertionIndex = indices[indices.length - 1] + 1;
      const copies = indices.map((index) => structuredClone(track.steps[index]));
      track.steps.splice(insertionIndex, 0, ...copies);
      current.timeline = nextTimeline;
      const nextSelection = Array.from({ length: copies.length }, (_, offset) => insertionIndex + offset);
      setSelectedStepIndices(nextSelection);
      setStepSelectionAnchor(nextSelection[0] ?? null);
      return syncClipAuthoring(current, clip.id);
    });
  }

  function deleteStep() {
    if (!selectedClip || !selectedTrack || !selectedStepIndices.length) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip) return current;
      const track = ensureClipTrack(clip, selectedLane);
      if (!track) return current;
      const indices = [...new Set(selectedStepIndices)]
        .filter((index) => index >= 0 && index < track.steps.length)
        .sort((a, b) => b - a);
      if (!indices.length) return current;
      indices.forEach((index) => {
        track.steps.splice(index, 1);
      });
      current.timeline = nextTimeline;
      const nextIndex =
        track.steps.length > 0 ? Math.min(indices[indices.length - 1], track.steps.length - 1) : null;
      setSingleStepSelection(nextIndex);
      return syncClipAuthoring(current, clip.id);
    });
  }

  return (
    <div className="app-shell">
      <audio
        ref={audioRef}
        src={effectiveAudioUrl ?? undefined}
        preload="auto"
        onCanPlay={() => setAudioReady(true)}
        onCanPlayThrough={() => setAudioReady(true)}
        onPlay={() => setIsPlaying(true)}
        onPause={() => {
          setIsPlaying(false);
          playbackTimeRef.current = audioRef.current?.currentTime ?? playbackTimeRef.current;
          setPlaybackTime(playbackTimeRef.current);
        }}
        onEnded={() => {
          setIsPlaying(false);
          playbackTimeRef.current = 0;
          setPlaybackTime(0);
        }}
      />
      <main className="workspace">
        <section className="left-column">
          <section className="workspace-row timeline-row">
            <aside className="timeline-sidebar">
              <section className="panel property-sidebar-panel">
                <div className="property-sidebar-header">
                  <h2>{selectedLaneMeta.label}</h2>
                  <code className="property-variable-name">{selectedLane}</code>
                </div>
                <p className="empty-copy">{selectedLaneMeta.description}</p>
                <div className="base-panel">
                  {baseState ? (
                    isColorLane(selectedLane) ? (
                      <ColorEditor
                        label="Base Value"
                        value={getBaseColor(baseState, selectedLane as "cold_color" | "hot_color")}
                        onChange={(nextColor) =>
                          updateDraft((current) => {
                            const state = current.params.shader_states?.playing;
                            if (!state) return current;
                            state[selectedLane] = nextColor as never;
                            return current;
                          })
                        }
                      />
                    ) : (
                      <label>
                        Base Value
                        <input
                          type="number"
                          step={0.01}
                          value={Number(baseState[selectedLane])}
                          onChange={(event) =>
                            updateDraft((current) => {
                              const state = current.params.shader_states?.playing;
                              if (!state) return current;
                              state[selectedLane] = Number(event.target.value) as never;
                              return current;
                            })
                          }
                        />
                      </label>
                    )
                  ) : null}
                </div>
              </section>

              <section className="panel utility-panel">
                <div className="session-summary">
                  <strong>{loaded?.name ?? "Untitled effect"}</strong>
                  <span>{dirty ? "Unsaved changes" : loaded ? "Saved draft" : "New draft"}</span>
                </div>
                <p className="empty-copy">{message}</p>
                <div className="action-grid">
                  <button onClick={handleNewVisualizer}>New Effect</button>
                  <button onClick={() => fileInputRef.current?.click()}>Load JSON</button>
                  <button onClick={() => audioInputRef.current?.click()}>Import Audio</button>
                  <button onClick={handleSave}>Save JSON</button>
                  <button
                    onClick={() => {
                      if (!loaded) return;
                      setDraft(normalizedConfig(loaded.visualizer));
                      clearPlacementSelection();
                      setSelectedStepIndices([]);
                      setStepSelectionAnchor(null);
                      setMessage(`Reverted ${loaded.name}.`);
                    }}
                    disabled={!loaded || !dirty}
                  >
                    Revert
                  </button>
                </div>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".json,application/json"
                  hidden
                  onChange={handleImportRecord}
                />
                <input
                  ref={audioInputRef}
                  type="file"
                  accept="audio/*"
                  hidden
                  onChange={handleImportAudio}
                />
              </section>
            </aside>

            <section className="panel arrangement-panel">
              <div className="inline-actions arrangement-toolbar">
                <label className="compact-field">
                  BPM
                  <input
                    type="number"
                    step={0.1}
                    value={timeline.bpm}
                    onChange={(event) =>
                      updateDraft((current) => {
                        const nextTimeline = timelineFromConfig(current);
                        nextTimeline.bpm = Math.max(1, Number(event.target.value));
                        current.timeline = nextTimeline;
                        return current;
                      })
                    }
                  />
                </label>
                <label className="compact-field">
                  Measures
                  <input
                    type="number"
                    step={1}
                    min={1}
                    value={timeline.measures}
                    onChange={(event) =>
                      updateDraft((current) => {
                        const nextTimeline = timelineFromConfig(current);
                        nextTimeline.measures = Math.max(1, Math.round(Number(event.target.value)));
                        current.timeline = nextTimeline;
                        return current;
                      })
                    }
                  />
                </label>
                <label className="compact-field">
                  Beats/Measure
                  <input
                    type="number"
                    step={1}
                    min={1}
                    value={timeline.beats_per_measure}
                    onChange={(event) =>
                      updateDraft((current) => {
                        const nextTimeline = timelineFromConfig(current);
                        nextTimeline.beats_per_measure = Math.max(
                          1,
                          Math.round(Number(event.target.value)),
                        );
                        current.timeline = nextTimeline;
                        return current;
                      })
                    }
                  />
                </label>
                <label className="compact-field">
                  Zoom
                  <input
                    type="range"
                    min={12}
                    max={96}
                    step={2}
                    value={timelineZoom}
                    onChange={(event) => setTimelineZoom(Number(event.target.value))}
                  />
                </label>
                <div className="tool-toggle" role="group" aria-label="Timeline tool">
                  <button
                    className={timelineTool === "select" ? "active" : ""}
                    onClick={() => setTimelineTool("select")}
                    type="button"
                  >
                    Select
                  </button>
                  <button
                    className={timelineTool === "pencil" ? "active" : ""}
                    onClick={() => setTimelineTool("pencil")}
                    type="button"
                  >
                    Pencil
                  </button>
                </div>
                <span>{selectedPlacementIndices.length ? `${selectedPlacementIndices.length} selected` : `track ${selectedLaneMeta.label}`}</span>
                <button onClick={addPlacement} disabled={!selectedClip}>
                  Place Selected Clip
                </button>
                <button onClick={copySelectedPlacements} disabled={!selectedPlacementIndices.length}>
                  Copy Placement{selectedPlacementIndices.length === 1 ? "" : "s"}
                </button>
                <button onClick={() => pasteCopiedPlacements()} disabled={!placementClipboard.length}>
                  Paste Placement{placementClipboard.length === 1 ? "" : "s"}
                </button>
                <button onClick={deleteSelectedPlacements} disabled={!selectedPlacementIndices.length}>
                  Delete Placement{selectedPlacementIndices.length === 1 ? "" : "s"}
                </button>
              </div>
              <div
                ref={arrangementRef}
                className="arrangement-grid"
                style={{ ["--timeline-grid" as never]: `${timelineZoom}px` }}
                onPointerMove={(event) => {
                  if (!dragState || !arrangementRef.current) return;
                  const beat = arrangementBeatFromPointer(event.clientX);
                  if (dragState.kind === "scrub") {
                    seekToBeat(snapPlaybackBeat(beat));
                  } else if (dragState.kind === "move") {
                    const placement = timeline.arrangement[dragState.placementIndex];
                    if (!placement) return;
                    updatePlacementAtIndex(dragState.placementIndex, {
                      ...placement,
                      start_beat: snapPlacementStart(beat - dragState.offsetBeats, dragState.placementIndex),
                    });
                  } else {
                    const placement = timeline.arrangement[dragState.placementIndex];
                    if (!placement) return;
                    const clip = timeline.clips.find((candidate) => candidate.id === placement.clip_id);
                    if (!clip) return;
                    const widthBeats = Math.max(clip.length_beats, beat - placement.start_beat);
                    updatePlacementAtIndex(dragState.placementIndex, {
                      ...placement,
                      repeats: Math.max(1, Math.round(widthBeats / clip.length_beats)),
                    });
                  }
                }}
                onPointerUp={() => setDragState(null)}
                onPointerLeave={() => setDragState(null)}
              >
                <div className="ruler-row">
                  <div className="ruler-spacer" />
                  <div
                    className="ruler-track"
                    style={{ width: timelineWidth }}
                    onPointerDown={(event) => {
                      event.preventDefault();
                      const beat = arrangementBeatFromPointer(event.clientX);
                      clearPlacementSelection();
                      setSelectedClipId(null);
                      setSelectedStepIndices([]);
                      setStepSelectionAnchor(null);
                      setDragState({ kind: "scrub" });
                      seekToBeat(snapPlaybackBeat(beat));
                    }}
                  >
                    {Array.from({ length: Math.ceil(totalTimelineBeats) + 1 }, (_, beat) => {
                      const isBar = beat % timeline.beats_per_measure === 0;
                      return (
                        <div
                          key={beat}
                          className={`ruler-mark ${isBar ? "bar" : ""}`}
                          style={{ left: beatToPx(beat) }}
                        >
                          {isBar ? <span>{beat / timeline.beats_per_measure + 1}</span> : null}
                        </div>
                      );
                    })}
                  </div>
                </div>
                {EDITOR_LANES.map((lane) => {
                  const trackIndex = LANE_ORDER.indexOf(lane);
                  return (
                    <div key={lane} className="track-row">
                      <button
                        className={`track-label ${selectedLane === lane ? "selected" : ""}`}
                        onClick={() => {
                          setSelectedLane(lane);
                          clearPlacementSelection();
                        }}
                      >
                        {laneMeta(lane).trackLabel}
                      </button>
                      <div
                        className="track-lane"
                        style={{ height: TRACK_HEIGHT, width: timelineWidth }}
                        onPointerDown={(event) => {
                          if (event.target !== event.currentTarget || !arrangementRef.current) {
                            return;
                          }
                          event.preventDefault();
                          const beat = arrangementBeatFromPointer(event.clientX);
                          if (timelineTool === "pencil") {
                            placeSelectedClipAtBeat(beat);
                            return;
                          }
                          setSelectedLane(lane);
                          clearPlacementSelection();
                          setSelectedClipId(null);
                          setSelectedStepIndices([]);
                          setStepSelectionAnchor(null);
                          setDragState({ kind: "scrub" });
                          seekToBeat(snapPlaybackBeat(beat));
                        }}
                      >
                        {timeline.arrangement
                          .map((placement, placementIndex) => ({ placement, placementIndex }))
                          .filter(({ placement }) => placement.track === trackIndex)
                          .map(({ placement, placementIndex }) => {
                            const clip = timeline.clips.find((candidate) => candidate.id === placement.clip_id);
                            if (!clip) return null;
                            const left = beatToPx(placement.start_beat);
                            const width = beatToPx(clip.length_beats * placement.repeats);
                            const isSelected = selectedPlacementSet.has(placementIndex);
                            const isActive =
                              uiCurrentBeat >= placement.start_beat &&
                              uiCurrentBeat < placement.start_beat + clip.length_beats * placement.repeats;
                            return (
                              <div
                                key={`${placement.clip_id}-${placementIndex}`}
                                className={`placement ${isSelected ? "selected" : ""} ${isActive ? "active" : ""}`}
                                title={clip.name}
                                style={{
                                  left,
                                  width,
                                  background: placementBackground(clip.color, isSelected),
                                }}
                                onPointerDown={(event) => {
                                  event.preventDefault();
                                  event.stopPropagation();
                                  selectPlacement(placementIndex, lane, clip.id, event);
                                  if (event.metaKey || event.ctrlKey || event.shiftKey) {
                                    return;
                                  }
                                  const rect = (event.currentTarget as HTMLDivElement).getBoundingClientRect();
                                  const hitEdge = rect.right - event.clientX < 12;
                                  const beatAtCursor = pxToBeat(event.clientX - rect.left);
                                  setDragState(
                                    hitEdge
                                      ? { kind: "resize", placementIndex }
                                      : { kind: "move", placementIndex, offsetBeats: beatAtCursor },
                                  );
                                }}
                              >
                                <span className="placement-label">
                                  {formatPlacementLabel(clip.name, width)}
                                </span>
                                <div className="resize-handle" />
                              </div>
                            );
                          })}
                        <div className="track-playhead" style={{ left: beatToPx(uiCurrentBeat) }} />
                      </div>
                    </div>
                  );
                })}
              </div>
              <div className="timeline-footer">
                <input
                  className="timeline-scrubber"
                  type="range"
                  min={0}
                  max={Math.max(totalDurationSeconds, 0.01)}
                  step={0.01}
                  value={Math.min(playbackTime, totalDurationSeconds)}
                  onChange={(event) => {
                    const nextTime = Number(event.target.value);
                    playbackTimeRef.current = nextTime;
                    setPlaybackTime(nextTime);
                    if (audioRef.current) {
                      audioRef.current.currentTime = nextTime;
                    }
                  }}
                />
                <div className="timeline-transport">
                  <div className="transport-actions">
                    <button
                      className="icon-button"
                      onClick={handleTogglePlayback}
                      disabled={!effectiveAudioUrl}
                      type="button"
                      title={playPending ? "Preparing playback" : isPlaying ? "Pause playback" : "Play playback"}
                      aria-label={playPending ? "Preparing playback" : isPlaying ? "Pause playback" : "Play playback"}
                    >
                      <TransportIcon name={playPending ? "loading" : isPlaying ? "pause" : "play"} />
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => {
                        if (!audioRef.current) return;
                        audioRef.current.pause();
                        audioRef.current.currentTime = 0;
                        setPlaybackTime(0);
                      }}
                      disabled={!effectiveAudioUrl}
                      type="button"
                      title="Stop playback"
                      aria-label="Stop playback"
                    >
                      <TransportIcon name="stop" />
                    </button>
                  </div>
                  <div className="transport-stat">
                    <span>Measure</span>
                    <strong>{formatMeasurePosition(uiCurrentBeat, timeline.beats_per_measure)}</strong>
                  </div>
                  <div className="transport-stat">
                    <span>Time</span>
                    <strong>
                      {formatDuration(playbackTime)} / {formatDuration(totalDurationSeconds)}
                    </strong>
                  </div>
                  <div className="transport-stat">
                    <span>State</span>
                    <strong>{dirty ? "Unsaved" : "Saved"}</strong>
                  </div>
                </div>
              </div>
            </section>
          </section>

          <section className="workspace-row clip-row">
            <section className="panel library-panel clips-panel">
              <div className="panel-header">
                <h2>Clips</h2>
                <div className="inline-actions">
                  <button className="icon-button" onClick={addClip} type="button" title="Add clip" aria-label="Add clip">
                    <ClipActionIcon name="add" />
                  </button>
                  <button
                    className="icon-button"
                    onClick={duplicateClip}
                    disabled={!selectedClip}
                    type="button"
                    title="Copy clip"
                    aria-label="Copy clip"
                  >
                    <ClipActionIcon name="copy" />
                  </button>
                  <button
                    className="icon-button"
                    onClick={deleteClip}
                    disabled={!selectedClip}
                    type="button"
                    title="Delete clip"
                    aria-label="Delete clip"
                  >
                    <ClipActionIcon name="delete" />
                  </button>
                </div>
              </div>
              <p className="empty-copy">Current track: {selectedLaneMeta.label}</p>
              <div className="list">
                {laneClips.map((clip) => (
                  <button
                    key={clip.id}
                    className={`clip-card ${selectedClipId === clip.id ? "selected" : ""}`}
                    onClick={() => {
                      setSelectedClipId(clip.id);
                      setSelectedLane(visibleLane(primaryLane(clip)?.lane ?? selectedLane));
                      clearPlacementSelection();
                    }}
                  >
                    <span
                      className="clip-swatch"
                      style={{ background: `rgb(${clip.color.map((channel) => Math.round(channel * 255)).join(" ")})` }}
                    />
                    <div>
                      <strong>{clip.name}</strong>
                      <span>{clipSummary(clip)}</span>
                    </div>
                  </button>
                ))}
                {!laneClips.length ? <p className="empty-copy">No clips for this track yet.</p> : null}
              </div>
            </section>

            <section className="panel clip-editor-panel">
              {selectedClip ? (
                <div className="clip-editor">
                  <div ref={clipFieldsRef} className="step-list lfo-clip-fields">
                    <p className="empty-copy">Editing track: {selectedLaneMeta.label}</p>
                    <div className="clip-field-grid">
                      <label className="field-span-2">
                        Name
                        <input
                          value={selectedClip.name}
                          onChange={(event) =>
                            updateDraft((current) => {
                              const nextTimeline = timelineFromConfig(current);
                              const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                              if (!clip) return current;
                              clip.name = event.target.value;
                              current.timeline = nextTimeline;
                              return current;
                            })
                          }
                        />
                      </label>
                      <label>
                        Length Beats
                        <input
                          type="number"
                          step={0.25}
                          value={selectedClip.length_beats}
                          onChange={(event) =>
                            updateDraft((current) => {
                              const nextTimeline = timelineFromConfig(current);
                              const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                              if (!clip) return current;
                              clip.length_beats = Math.max(0.25, Number(event.target.value));
                              current.timeline = nextTimeline;
                              return current;
                            })
                          }
                        />
                      </label>
                      {isLegacyClip(selectedClip) ? (
                        <p className="empty-copy field-span-2">Legacy step clip detected. This layout preserves playback, but LFO editing is only available for LFO clips.</p>
                      ) : (
                        <>
                          <label>
                            Lane
                            <select
                              value={selectedClip.source?.lane ?? selectedLane}
                              onChange={(event) =>
                                updateDraft((current) => {
                                  const nextTimeline = timelineFromConfig(current);
                                  const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                                  if (!clip?.source || clip.source.kind !== "lfo") return current;
                                  clip.source.lane = event.target.value as LaneId;
                                  current.timeline = nextTimeline;
                                  setSelectedLane(event.target.value as LaneId);
                                  return current;
                                })
                              }
                            >
                              {EDITOR_LANES.filter((lane) => !isColorLane(lane)).map((lane) => (
                                <option key={lane} value={lane}>
                                  {laneMeta(lane).label}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label>
                            Min
                            <input
                              type="number"
                              step={0.01}
                              value={selectedClip.source?.min ?? 0}
                              onChange={(event) =>
                                updateDraft((current) => {
                                  const nextTimeline = timelineFromConfig(current);
                                  const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                                  if (!clip?.source || clip.source.kind !== "lfo") return current;
                                  clip.source.min = Number(event.target.value);
                                  current.timeline = nextTimeline;
                                  return current;
                                })
                              }
                            />
                          </label>
                          <label>
                            Max
                            <input
                              type="number"
                              step={0.01}
                              value={selectedClip.source?.max ?? 0}
                              onChange={(event) =>
                                updateDraft((current) => {
                                  const nextTimeline = timelineFromConfig(current);
                                  const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                                  if (!clip?.source || clip.source.kind !== "lfo") return current;
                                  clip.source.max = Number(event.target.value);
                                  current.timeline = nextTimeline;
                                  return current;
                                })
                              }
                            />
                          </label>
                          <label className="field-span-2">
                            Period Beats
                            <input
                              type="number"
                              step={0.25}
                              value={selectedClip.source?.period_beats ?? 4}
                              onChange={(event) =>
                                updateDraft((current) => {
                                  const nextTimeline = timelineFromConfig(current);
                                  const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                                  if (!clip?.source || clip.source.kind !== "lfo") return current;
                                  clip.source.period_beats = Math.max(0.25, Number(event.target.value));
                                  current.timeline = nextTimeline;
                                  return current;
                                })
                              }
                            />
                          </label>
                        </>
                      )}
                    </div>
                  </div>
                  <div className="shape-point-column" style={clipEditorHeight ? { height: `${clipEditorHeight}px` } : undefined}>
                    {selectedShape ? (
                      <div className="shape-point-list vertical">
                        {selectedShape.points.map((point, index) => (
                          <button
                            key={`${index}-${point.phase}-${point.value}`}
                            className={`list-item ${selectedPointIndex === index ? "selected" : ""}`}
                            onClick={() => setSelectedPointIndex(index)}
                          >
                            <strong>Point {index + 1}</strong>
                            <span>{pointLabel(point)}</span>
                          </button>
                        ))}
                      </div>
                    ) : (
                      <p>Select a shape to edit.</p>
                    )}
                  </div>
                  <div className="step-inspector lfo-shape-inspector" style={clipEditorHeight ? { height: `${clipEditorHeight}px` } : undefined}>
                    {selectedShape ? (
                      <div className="shape-editor-frame">
                        <svg
                          ref={shapeSvgRef}
                          className="shape-editor"
                          viewBox={`0 0 ${SHAPE_EDITOR_WIDTH} ${SHAPE_EDITOR_HEIGHT}`}
                          onPointerDown={(event) => {
                            const svg = shapeSvgRef.current;
                            if (!svg || !selectedShape) return;
                            event.currentTarget.setPointerCapture(event.pointerId);
                            if (event.target instanceof SVGCircleElement) {
                              return;
                            }
                            const point = pointerToPoint(svg, event.clientX, event.clientY);
                            const insertedIndex = selectedShape.points.length;
                            updateSelectedClipShape((points) => {
                              points.push(point);
                            });
                            setSelectedPointIndex(insertedIndex);
                          }}
                          onPointerMove={(event) => {
                            const svg = shapeSvgRef.current;
                            if (shapeDragIndex == null || !selectedShape || !svg) return;
                            const point = pointerToPoint(svg, event.clientX, event.clientY);
                            updateSelectedClipShape((points) => {
                              if (points[shapeDragIndex]) {
                                points[shapeDragIndex] = point;
                              }
                            });
                          }}
                          onPointerUp={(event) => {
                            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                              event.currentTarget.releasePointerCapture(event.pointerId);
                            }
                            setShapeDragIndex(null);
                          }}
                          onPointerCancel={() => setShapeDragIndex(null)}
                          onLostPointerCapture={() => setShapeDragIndex(null)}
                        >
                          <rect x="0" y="0" width={SHAPE_EDITOR_WIDTH} height={SHAPE_EDITOR_HEIGHT} />
                          <path
                            className="shape-grid shape-grid-bound"
                            d={`M ${SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_VERTICAL_PADDING} L ${SHAPE_EDITOR_WIDTH - SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_VERTICAL_PADDING}`}
                          />
                          <path
                            className="shape-grid shape-grid-bound"
                            d={`M ${SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_HEIGHT - SHAPE_EDITOR_VERTICAL_PADDING} L ${SHAPE_EDITOR_WIDTH - SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_HEIGHT - SHAPE_EDITOR_VERTICAL_PADDING}`}
                          />
                          <path className="shape-grid" d={`M 0 ${SHAPE_EDITOR_HEIGHT / 2} L ${SHAPE_EDITOR_WIDTH} ${SHAPE_EDITOR_HEIGHT / 2}`} />
                          <path className="shape-grid" d={`M ${SHAPE_EDITOR_WIDTH / 2} 0 L ${SHAPE_EDITOR_WIDTH / 2} ${SHAPE_EDITOR_HEIGHT}`} />
                          <path
                            className="shape-curve"
                            d={shapePath(selectedShape, SHAPE_EDITOR_WIDTH, SHAPE_EDITOR_HEIGHT, SHAPE_EDITOR_VERTICAL_PADDING)}
                          />
                          {selectedShape.points.map((point, index) => (
                            <circle
                              key={`${index}-${point.phase}-${point.value}`}
                              className={selectedPointIndex === index ? "shape-point selected" : "shape-point"}
                              cx={point.phase * SHAPE_EDITOR_WIDTH}
                              cy={shapeEditorY(point.value)}
                              r={5}
                              onPointerDown={(event) => {
                                event.stopPropagation();
                                event.currentTarget.setPointerCapture(event.pointerId);
                                setSelectedPointIndex(index);
                                setShapeDragIndex(index);
                              }}
                            />
                          ))}
                        </svg>
                      </div>
                    ) : null}
                  </div>
                </div>
              ) : (
                <p>Select or create a clip.</p>
              )}
            </section>
          </section>
        </section>

        <aside className="preview-column">
          <section className="panel preview-panel">
            <PreviewCanvas
              config={normalizedDraft}
              audioRef={audioRef}
              playbackTimeRef={playbackTimeRef}
              isPlaying={isPlaying}
              onReady={() => setPreviewReady(true)}
            />
          </section>
        </aside>
      </main>
    </div>
  );
}

function editableTrack(
  clip: { authoring?: { tracks: Array<{ lane: LaneId; steps: ClipTweenStep[] }> } },
  fallbackLane: LaneId,
) {
  return clip.authoring?.tracks[0] ?? { lane: visibleLane(primaryLane(clip as never)?.lane ?? fallbackLane), steps: [] };
}

function visibleLane(lane: LaneId | null | undefined): LaneId {
  if (!lane || HIDDEN_EDITOR_LANES.has(lane)) {
    return "motion_rate";
  }
  return lane;
}

function laneMeta(lane: LaneId): { label: string; trackLabel: string; description: string } {
  switch (lane) {
    case "motion_rate":
      return {
        label: "Motion X",
        trackLabel: "Motion X",
        description: "Controls horizontal movement of the dot field. Positive values move right and negative values move left.",
      };
    case "motion_rate_y":
      return {
        label: "Motion Y",
        trackLabel: "Motion Y",
        description: "Controls vertical movement of the dot field. Positive values move down and negative values move up.",
      };
    case "lattice_density":
      return {
        label: "Lattice Density",
        trackLabel: "Density",
        description: "Changes how tightly packed the dot grid is.",
      };
    case "circle_radius":
      return {
        label: "Bulge Radius",
        trackLabel: "Bulge Radius",
        description: "Sets how large the bulge appears on screen.",
      };
    case "circle_falloff_start":
      return {
        label: "Bulge Falloff Start",
        trackLabel: "Falloff Start",
        description: "Determines where the bulge begins fading from its center toward the edge.",
      };
    case "circle_falloff_end":
      return {
        label: "Bulge Falloff End",
        trackLabel: "Falloff End",
        description: "Determines where the bulge fully blends back into the surrounding field.",
      };
    case "bulge_amount":
      return {
        label: "Bulge Amount",
        trackLabel: "Bulge Amount",
        description: "Controls how strongly the center distorts the dot field.",
      };
    case "rim_guard":
      return {
        label: "Rim Guard",
        trackLabel: "Rim Guard",
        description: "Stabilizes the edge of the bulge to keep the rim from blowing out.",
      };
    case "rim_exponent":
      return {
        label: "Rim Exponent",
        trackLabel: "Rim Exponent",
        description: "Shapes how concentrated the rim emphasis is near the edge of the bulge.",
      };
    case "rim_warp":
      return {
        label: "Rim Warp",
        trackLabel: "Rim Warp",
        description: "Pushes dots around the outer ring of the bulge for a stronger lens edge.",
      };
    case "dot_size":
      return {
        label: "Dot Size",
        trackLabel: "Dot Size",
        description: "Controls the base size of every dot in the field.",
      };
    case "outer_dot_scale":
      return {
        label: "Outer Dot Scale",
        trackLabel: "Outer Scale",
        description: "Scales dots outside the bulge relative to the dots at the center.",
      };
    case "edge_softness":
      return {
        label: "Edge Softness",
        trackLabel: "Edge Softness",
        description: "Softens or hardens the edge of each dot.",
      };
    case "chromatic_aberration":
      return {
        label: "Chromatic Aberration",
        trackLabel: "Chromatic Shift",
        description: "Offsets red and blue channels near the bulge for a split-color fringe.",
      };
    case "cold_color":
      return {
        label: "Surrounding Dots Color",
        trackLabel: "Outer Color",
        description: "Sets the color of the dots surrounding the bulge.",
      };
    case "hot_color":
      return {
        label: "Bulge Center Color",
        trackLabel: "Center Color",
        description: "Sets the color of the dots at the center of the bulge.",
      };
    case "color_cycle_rate":
      return {
        label: "Legacy Color Cycle",
        trackLabel: "Legacy Cycle",
        description: "Legacy control from the earlier shader version. The current shader no longer cycles between colors.",
      };
    case "inner_alpha":
      return {
        label: "Bulge Center Alpha",
        trackLabel: "Center Alpha",
        description: "Controls how opaque the dots stay inside the bulge.",
      };
  }
}

function getBaseColor(
  state: ChromaticBulgeGridShaderState,
  lane: Extract<LaneId, "cold_color" | "hot_color">,
): [number, number, number] {
  return Array.from(state[lane]) as [number, number, number];
}

function ColorEditor({
  label,
  value,
  onChange,
}: {
  label: string;
  value: [number, number, number];
  onChange: (value: [number, number, number]) => void;
}) {
  return (
    <div className="color-editor">
      <label>
        {label}
        <input
          className="color-picker"
          type="color"
          value={colorToHex(value)}
          onChange={(event) => onChange(hexToColor(event.target.value))}
        />
      </label>
      <div className="color-channel-grid">
        {(["R", "G", "B"] as const).map((channel, index) => (
          <label key={channel}>
            {channel}
            <input
              type="number"
              min={0}
              max={1}
              step={0.01}
              value={value[index]}
              onChange={(event) => {
                const next = [...value] as [number, number, number];
                next[index] = clampColorChannel(Number(event.target.value));
                onChange(next);
              }}
            />
          </label>
        ))}
      </div>
    </div>
  );
}

function colorToHex(value: [number, number, number]): string {
  return `#${value
    .map((channel) => Math.round(clampColorChannel(channel) * 255).toString(16).padStart(2, "0"))
    .join("")}`;
}

function hexToColor(value: string): [number, number, number] {
  const normalized = value.replace("#", "");
  if (normalized.length !== 6) {
    return [1, 1, 1];
  }
  return [
    parseInt(normalized.slice(0, 2), 16) / 255,
    parseInt(normalized.slice(2, 4), 16) / 255,
    parseInt(normalized.slice(4, 6), 16) / 255,
  ];
}

function clampColorChannel(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(1, value));
}

function placementBackground(color: [number, number, number], selected: boolean): string {
  const fill = selected ? mixColor(color, [0.96, 0.73, 0.26], 0.42) : color;
  return `rgb(${fill.map((channel) => Math.round(clampColorChannel(channel) * 255)).join(" ")})`;
}

function mixColor(
  left: [number, number, number],
  right: [number, number, number],
  amount: number,
): [number, number, number] {
  const t = Math.max(0, Math.min(1, amount));
  return [
    left[0] + (right[0] - left[0]) * t,
    left[1] + (right[1] - left[1]) * t,
    left[2] + (right[2] - left[2]) * t,
  ];
}

function formatPlacementLabel(name: string, widthPx: number): string {
  if (widthPx < 30) {
    return "•";
  }
  if (widthPx < 76) {
    const initials = name
      .split(/\s+/)
      .filter(Boolean)
      .map((part) => part[0])
      .join("")
      .slice(0, 3)
      .toUpperCase();
    return initials || name.slice(0, 2).toUpperCase();
  }
  return name;
}

function jsonFilename(name: string | null | undefined): string {
  const trimmed = (name ?? "").trim();
  if (!trimmed) {
    return "tty0-visualizer.json";
  }
  return trimmed.toLowerCase().endsWith(".json") ? trimmed : `${trimmed}.json`;
}

function pointerToPoint(svg: SVGSVGElement, clientX: number, clientY: number): LfoPoint {
  const svgPoint = svg.createSVGPoint();
  svgPoint.x = clientX;
  svgPoint.y = clientY;
  const inverse = svg.getScreenCTM()?.inverse();
  if (!inverse) {
    return { phase: 0, value: 0 };
  }
  const local = svgPoint.matrixTransform(inverse);
  const phase = Math.max(0, Math.min(1, local.x / SHAPE_EDITOR_WIDTH));
  const value = Math.max(
    0,
    Math.min(
      1,
      1 - ((local.y - SHAPE_EDITOR_VERTICAL_PADDING) / (SHAPE_EDITOR_HEIGHT - SHAPE_EDITOR_VERTICAL_PADDING * 2)),
    ),
  );
  return { phase, value };
}

function shapeEditorY(value: number): number {
  return SHAPE_EDITOR_VERTICAL_PADDING + (1 - value) * (SHAPE_EDITOR_HEIGHT - SHAPE_EDITOR_VERTICAL_PADDING * 2);
}

function formatMeasurePosition(currentBeat: number, beatsPerMeasure: number): string {
  const safeBeatsPerMeasure = Math.max(1, Math.round(beatsPerMeasure));
  const unitsPerBeat = 1;
  const totalUnits = Math.max(0, Math.round(currentBeat * unitsPerBeat));
  const unitsPerMeasure = safeBeatsPerMeasure * unitsPerBeat;
  const wholeMeasures = Math.floor(totalUnits / unitsPerMeasure);
  const remainderUnits = totalUnits % unitsPerMeasure;
  const displayedMeasure = wholeMeasures + 1;
  const displayedQuarter = remainderUnits + 1;
  return `${displayedMeasure} ${displayedQuarter}/${unitsPerMeasure}`;
}

function formatDuration(seconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const remainderSeconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${remainderSeconds.toString().padStart(2, "0")}`;
  }

  return `${minutes}:${remainderSeconds.toString().padStart(2, "0")}`;
}

function ensureClipTrack(
  clip: { authoring?: { tracks: Array<{ lane: LaneId; steps: ClipTweenStep[] }> } } | undefined,
  fallbackLane: LaneId,
) {
  if (!clip) {
    return null;
  }
  if (!clip.authoring) {
    clip.authoring = { tracks: [] };
  }
  if (!clip.authoring.tracks[0]) {
    clip.authoring.tracks[0] = {
      lane: primaryLane(clip as never)?.lane ?? fallbackLane,
      steps: [],
    };
  }
  return clip.authoring.tracks[0];
}

function TransportIcon({ name }: { name: "loading" | "pause" | "play" | "stop" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "play" ? <path d="M6 4.5 15 10 6 15.5Z" fill="currentColor" stroke="none" /> : null}
      {name === "pause" ? (
        <>
          <path d="M6.5 4.5v11" {...commonProps} />
          <path d="M13.5 4.5v11" {...commonProps} />
        </>
      ) : null}
      {name === "stop" ? <rect x="5.5" y="5.5" width="9" height="9" rx="1.5" fill="currentColor" /> : null}
      {name === "loading" ? (
        <>
          <path d="M10 3.5a6.5 6.5 0 1 1-4.6 1.9" {...commonProps} />
          <path d="M5.4 5.4 4 2.8l2.8 1.4" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

function ClipActionIcon({ name }: { name: "add" | "copy" | "delete" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "add" ? (
        <>
          <path d="M10 4.5v11" {...commonProps} />
          <path d="M4.5 10h11" {...commonProps} />
        </>
      ) : null}
      {name === "copy" ? (
        <>
          <rect x="7" y="5" width="8" height="10" rx="1.5" {...commonProps} />
          <path d="M5 12.5H4.5A1.5 1.5 0 0 1 3 11V6.5A1.5 1.5 0 0 1 4.5 5H9" {...commonProps} />
        </>
      ) : null}
      {name === "delete" ? (
        <>
          <path d="M5.5 6.5h9" {...commonProps} />
          <path d="M8 3.8h4" {...commonProps} />
          <path d="M7 6.5v8" {...commonProps} />
          <path d="M10 6.5v8" {...commonProps} />
          <path d="M13 6.5v8" {...commonProps} />
          <path d="M6.5 6.5 7 15a1.5 1.5 0 0 0 1.5 1.4h3a1.5 1.5 0 0 0 1.5-1.4l.5-8.5" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

function PreviewCanvas({
  config,
  audioRef,
  playbackTimeRef,
  isPlaying,
  onReady,
}: {
  config: TrackVisualizerConfig;
  audioRef: { current: HTMLAudioElement | null };
  playbackTimeRef: { current: number };
  isPlaying: boolean;
  onReady: () => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const configRef = useRef(config);
  const isPlayingRef = useRef(isPlaying);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    isPlayingRef.current = isPlaying;
  }, [isPlaying]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const gl = canvas.getContext("webgl2");
    if (!gl) return;
    const program = createProgram(gl, VERTEX_SHADER, FRAGMENT_SHADER);
    if (!program) return;
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    gl.useProgram(program);

    let frame = 0;
    let announcedReady = false;
    const render = () => {
      const audio = audioRef.current;
      const currentTime = audio ? audio.currentTime : playbackTimeRef.current;
      const currentIsPlaying = audio ? !audio.paused && !audio.ended : isPlayingRef.current;
      const currentUniforms = resolveNormalizedChromaticBulgeGrid(configRef.current, {
        currentTimeSecs: currentTime,
        visualTimeSecs: currentTime,
        isPlaying: currentIsPlaying,
        timelinePreview: true,
      }).uniforms;
      const dpr = window.devicePixelRatio || 1;
      const width = Math.max(1, Math.floor(canvas.clientWidth * dpr));
      const height = Math.max(1, Math.floor(canvas.clientHeight * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      gl.viewport(0, 0, width, height);
      gl.clearColor(0, 0, 0, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      setUniform2f(gl, program, "u_resolution", width, height);
      setUniform1f(gl, program, "u_time", currentTime);
      setUniform2f(gl, program, "u_motion_rate", currentUniforms.motion_rate, currentUniforms.motion_rate_y);
      setUniform1f(gl, program, "u_lattice_density", currentUniforms.lattice_density);
      setUniform1f(gl, program, "u_circle_radius", currentUniforms.circle_radius);
      setUniform1f(gl, program, "u_circle_falloff_start", currentUniforms.circle_falloff_start);
      setUniform1f(gl, program, "u_circle_falloff_end", currentUniforms.circle_falloff_end);
      setUniform1f(gl, program, "u_bulge_amount", currentUniforms.bulge_amount);
      setUniform1f(gl, program, "u_rim_guard", currentUniforms.rim_guard);
      setUniform1f(gl, program, "u_rim_exponent", currentUniforms.rim_exponent);
      setUniform1f(gl, program, "u_rim_warp", currentUniforms.rim_warp);
      setUniform1f(gl, program, "u_dot_size", currentUniforms.dot_size);
      setUniform1f(gl, program, "u_outer_dot_scale", currentUniforms.outer_dot_scale);
      setUniform1f(gl, program, "u_edge_softness", currentUniforms.edge_softness);
      setUniform1f(gl, program, "u_chromatic_aberration", currentUniforms.chromatic_aberration);
      setUniform3f(gl, program, "u_cold_color", currentUniforms.cold_color);
      setUniform3f(gl, program, "u_hot_color", currentUniforms.hot_color);
      setUniform1f(gl, program, "u_inner_alpha", currentUniforms.inner_alpha);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      if (!announcedReady) {
        announcedReady = true;
        onReady();
      }
      frame = requestAnimationFrame(render);
    };
    frame = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(frame);
      gl.deleteProgram(program);
      if (vao) gl.deleteVertexArray(vao);
    };
  }, [audioRef, onReady, playbackTimeRef]);

  return <canvas ref={canvasRef} className="preview-canvas" />;
}

function createProgram(
  gl: WebGL2RenderingContext,
  vertexSource: string,
  fragmentSource: string,
): WebGLProgram | null {
  const vertex = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragment = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);
  if (!vertex || !fragment) return null;
  const program = gl.createProgram();
  if (!program) return null;
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error(gl.getProgramInfoLog(program));
    return null;
  }
  return program;
}

function compileShader(
  gl: WebGL2RenderingContext,
  type: number,
  source: string,
): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error(gl.getShaderInfoLog(shader));
    return null;
  }
  return shader;
}

function setUniform1f(gl: WebGL2RenderingContext, program: WebGLProgram, name: string, value: number) {
  const location = gl.getUniformLocation(program, name);
  if (location) gl.uniform1f(location, value);
}

function setUniform2f(gl: WebGL2RenderingContext, program: WebGLProgram, name: string, x: number, y: number) {
  const location = gl.getUniformLocation(program, name);
  if (location) gl.uniform2f(location, x, y);
}

function setUniform3f(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  name: string,
  value: [number, number, number],
) {
  const location = gl.getUniformLocation(program, name);
  if (location) gl.uniform3f(location, value[0], value[1], value[2]);
}
