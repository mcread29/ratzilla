import { ChangeEvent, DragEvent, MouseEvent, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { importRecordFromJson } from "../../platform";
import { ClipPlacement, ColorValue, LaneId, LfoPoint, TrackVisualizerConfig } from "../../types";
import {
  audioTimeFromTransportTime,
  buildTimelineIndex,
  createPlacement,
  defaultLfoClip,
  defaultVisualizer,
  leadInBeats,
  leadInSeconds,
  normalizeClipLfoShape,
  normalizedConfig,
  primaryLane,
  songBeatFromTransportTime,
  timelineFromConfig,
  totalBeats,
  totalDisplayBeats,
  transportTimeFromAudioTime,
  transportTimeFromDisplayBeat,
} from "../../vfx";
import {
  CLIP_LENGTH_BAR_OPTIONS,
  DEFAULT_TIMELINE_VIEWPORT_WIDTH,
  MAX_VISIBLE_MEASURES_AT_MAX_ZOOM,
  TIMELINE_LABEL_WIDTH,
} from "../constants";
import { ClipDropIndicator, LoadedDocument, PlacementClipboardEntry, TimelineTool } from "../editor-types";
import { jsonFilename, serializeVisualizer } from "../utils/formatting";
import { EDITOR_LANES, laneMeta, visibleLane } from "../utils/lanes";
import { snapBeatToGrid } from "../utils/timelineMath";

const AUDIO_HANDOFF_DEBUG = false;

export function useVfxEditorController() {
  type MessageTone = "info" | "success" | "warning" | "error";

  const [loaded, setLoaded] = useState<LoadedDocument | null>(null);
  const [draft, setDraft] = useState<TrackVisualizerConfig>(normalizedConfig(defaultVisualizer()));
  const [selectedLane, setSelectedLane] = useState<LaneId>("motion_rate");
  const [selectedClipId, setSelectedClipId] = useState<string | null>(null);
  const [selectedPointIndex, setSelectedPointIndex] = useState<number | null>(null);
  const [selectedPlacementIndices, setSelectedPlacementIndices] = useState<number[]>([]);
  const [placementSelectionAnchor, setPlacementSelectionAnchor] = useState<number | null>(null);
  const [message, setMessage] = useState<string>("Load a visualizer JSON or start a new effect.");
  const [messageTone, setMessageTone] = useState<MessageTone>("info");
  const [isPlaying, setIsPlaying] = useState(false);
  const [previewReady, setPreviewReady] = useState(false);
  const [audioReady, setAudioReady] = useState(false);
  const [audioDuration, setAudioDuration] = useState<number | null>(null);
  const [playPending, setPlayPending] = useState(false);
  const [arrangementViewportWidth, setArrangementViewportWidth] = useState(DEFAULT_TIMELINE_VIEWPORT_WIDTH);
  const [timelineZoom, setTimelineZoom] = useState(32);
  const [timelineTool, setTimelineTool] = useState<TimelineTool>("select");
  const [placementClipboard, setPlacementClipboard] = useState<PlacementClipboardEntry[]>([]);
  const [importedAudioUrl, setImportedAudioUrl] = useState<string | null>(null);
  const [draggedClipId, setDraggedClipId] = useState<string | null>(null);
  const [clipDropIndicator, setClipDropIndicator] = useState<ClipDropIndicator | null>(null);

  const playbackTimeRef = useRef(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const outputPrimeContextRef = useRef<AudioContext | null>(null);
  const outputPrimeDoneRef = useRef(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const audioInputRef = useRef<HTMLInputElement | null>(null);
  const transportFrameRef = useRef<number | null>(null);
  const transportOriginTimeRef = useRef(0);
  const transportStartedAtRef = useRef(0);
  const audioStartedRef = useRef(false);
  const audioStartPendingRef = useRef(false);
  const audioTransportAnchorRef = useRef(0);
  const audioHandoffLoggedRef = useRef(false);
  const audioFirstProgressLoggedRef = useRef(false);

  const timeline = useMemo(() => timelineFromConfig(draft), [draft]);
  const timelineIndex = useMemo(() => buildTimelineIndex(timeline), [timeline]);
  const totalTimelineBeats = totalBeats(timeline);
  const totalVisibleBeats = totalDisplayBeats(timeline);
  const minPlacementBeat = -leadInBeats(timeline);
  const timelineViewportWidth = Math.max(1, arrangementViewportWidth - TIMELINE_LABEL_WIDTH);
  const minTimelineZoom = Math.max(0.25, timelineViewportWidth / Math.max(totalVisibleBeats, 1));
  const maxTimelineZoom = Math.max(
    minTimelineZoom,
    timelineViewportWidth / Math.max(MAX_VISIBLE_MEASURES_AT_MAX_ZOOM * timeline.beats_per_measure, 1),
  );
  const timelineWidth = Math.ceil(Math.max(timelineViewportWidth, totalVisibleBeats * timelineZoom));
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
  const selectedPlacementSet = useMemo(() => new Set(selectedPlacementIndices), [selectedPlacementIndices]);
  const draftSnapshot = useMemo(() => serializeVisualizer(draft), [draft]);
  const dirty = useMemo(() => (loaded ? draftSnapshot !== loaded.savedSnapshot : false), [draftSnapshot, loaded]);
  const clipLengthOptions = useMemo(
    () =>
      CLIP_LENGTH_BAR_OPTIONS.map((option) => ({
        label: option.label,
        beats: timeline.beats_per_measure * option.bars,
      })),
    [timeline.beats_per_measure],
  );
  const effectiveAudioUrl = importedAudioUrl ?? loaded?.audioUrl ?? null;
  const baseState = draft.params.shader_states?.playing;
  const selectedLaneMeta = laneMeta(selectedLane);
  const leadInDuration = leadInSeconds(timeline);
  const selectedClipBeatValue = selectedClip?.length_beats ?? 0;
  const selectedClipBeatOption =
    clipLengthOptions.find((option) => Math.abs(option.beats - selectedClipBeatValue) < 0.0001)?.beats.toString() ?? "";
  const totalDurationSeconds =
    typeof audioDuration === "number" && Number.isFinite(audioDuration) && audioDuration > 0
      ? audioDuration + leadInDuration
      : leadInDuration + (totalTimelineBeats * 60) / timeline.bpm;

  useEffect(() => {
    setSelectedPointIndex(selectedShape?.points.length ? 0 : null);
  }, [selectedClip?.id, selectedShape?.points.length]);

  useEffect(() => {
    return () => {
      if (importedAudioUrl) {
        URL.revokeObjectURL(importedAudioUrl);
      }
    };
  }, [importedAudioUrl]);

  function clampTransportTime(nextTime: number): number {
    return Math.max(0, Math.min(totalDurationSeconds, nextTime));
  }

  async function primeAudioOutputPath(): Promise<void> {
    try {
      const AudioContextCtor = window.AudioContext;
      if (!AudioContextCtor) {
        return;
      }
      const context = outputPrimeContextRef.current ?? new AudioContextCtor();
      outputPrimeContextRef.current = context;
      if (outputPrimeDoneRef.current && context.state === "running") {
        return;
      }
      if (context.state === "suspended") {
        await context.resume();
      }
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = "sine";
      oscillator.frequency.value = 440;
      gain.gain.value = 0.00001;
      oscillator.connect(gain);
      gain.connect(context.destination);
      oscillator.onended = () => {
        oscillator.disconnect();
        gain.disconnect();
      };
      const startAt = context.currentTime;
      const stopAt = startAt + 0.05;
      oscillator.start(startAt);
      oscillator.stop(stopAt);
      outputPrimeDoneRef.current = true;
    } catch (error) {
      if (AUDIO_HANDOFF_DEBUG) {
        console.debug("[tty0-audio]", "output-prime-failed", {
          performanceNowMs: Number(performance.now().toFixed(1)),
          message: error instanceof Error ? error.message : String(error),
        });
      }
    }
  }

  function logAudioDebug(message: string, extra?: Record<string, unknown>) {
    if (!AUDIO_HANDOFF_DEBUG) return;
    const audio = audioRef.current;
    console.debug("[tty0-audio]", message, {
      performanceNowMs: Number(performance.now().toFixed(1)),
      playbackTime: Number(playbackTimeRef.current.toFixed(3)),
      audioCurrentTime: audio ? Number(audio.currentTime.toFixed(3)) : null,
      readyState: audio?.readyState ?? null,
      paused: audio?.paused ?? null,
      waiting: audio ? audio.readyState < HTMLMediaElement.HAVE_FUTURE_DATA : null,
      ...extra,
    });
  }

  function stopTransportFrame() {
    if (transportFrameRef.current != null) {
      cancelAnimationFrame(transportFrameRef.current);
      transportFrameRef.current = null;
    }
  }

  function syncAudioToTransport(nextTime: number, forceCurrentTime = false) {
    const audio = audioRef.current;
    if (!audio) return;
    const targetAudioTime = audioTimeFromTransportTime(nextTime, timeline);
    if (forceCurrentTime || !isPlaying || !audioStartedRef.current || nextTime < leadInDuration) {
      if (Math.abs(audio.currentTime - targetAudioTime) > 0.05) {
        audio.currentTime = targetAudioTime;
      }
    }
    if (nextTime < leadInDuration && !audio.paused) {
      audio.pause();
    }
  }

  function pausePlayback() {
    stopTransportFrame();
    const audio = audioRef.current;
    const nextTime =
      audio && audioStartedRef.current ? audio.currentTime + audioTransportAnchorRef.current : playbackTimeRef.current;
    if (audio && !audio.paused) {
      audio.pause();
    }
    audioStartedRef.current = false;
    audioStartPendingRef.current = false;
    audioTransportAnchorRef.current = 0;
    audioHandoffLoggedRef.current = false;
    audioFirstProgressLoggedRef.current = false;
    playbackTimeRef.current = clampTransportTime(nextTime);
    setIsPlaying(false);
    setPlayPending(false);
  }

  function stopPlayback(nextTime = 0) {
    stopTransportFrame();
    const audio = audioRef.current;
    if (audio && !audio.paused) {
      audio.pause();
    }
    audioStartedRef.current = false;
    audioStartPendingRef.current = false;
    audioTransportAnchorRef.current = 0;
    audioHandoffLoggedRef.current = false;
    audioFirstProgressLoggedRef.current = false;
    playbackTimeRef.current = clampTransportTime(nextTime);
    setIsPlaying(false);
    setPlayPending(false);
  }

  async function startAudioAtTransportTime(nextTime: number): Promise<boolean> {
    const audio = audioRef.current;
    if (!audio) {
      return false;
    }
    if (audioStartedRef.current) {
      return false;
    }
    if (audioStartPendingRef.current) {
      return false;
    }
    audioStartPendingRef.current = true;
    const targetAudioTime = audioTimeFromTransportTime(nextTime, timeline);
    if (Math.abs(audio.currentTime - targetAudioTime) > 0.05) {
      logAudioDebug("seek-before-play", {
        nextTime: Number(nextTime.toFixed(3)),
        targetAudioTime: Number(targetAudioTime.toFixed(3)),
      });
      audio.currentTime = targetAudioTime;
    }
    audioTransportAnchorRef.current = nextTime - audio.currentTime;
    logAudioDebug("audio-play-request", {
      nextTime: Number(nextTime.toFixed(3)),
      targetAudioTime: Number(targetAudioTime.toFixed(3)),
      anchor: Number(audioTransportAnchorRef.current.toFixed(3)),
    });
    try {
      await audio.play();
      audioStartPendingRef.current = false;
      audioStartedRef.current = true;
      logAudioDebug("audio-play-resolved", {
        anchor: Number(audioTransportAnchorRef.current.toFixed(3)),
      });
      return true;
    } catch (error) {
      audioStartPendingRef.current = false;
      pausePlayback();
      const message = error instanceof Error ? error.message : "browser blocked playback or transport failed";
      setStatus(message, "error");
      toast.error("Playback failed.", {
        description: message,
      });
      return false;
    }
  }

  function runTransportFrame(now: number) {
    let nextTime = transportOriginTimeRef.current + (now - transportStartedAtRef.current) / 1000;
    const audio = audioRef.current;
    if (audio && audioStartedRef.current && !audio.paused && !audio.ended) {
      const audioTime = audio.currentTime + audioTransportAnchorRef.current;
      nextTime = Math.max(nextTime, audioTime);
      if (!audioFirstProgressLoggedRef.current && audio.currentTime > 0.02) {
        audioFirstProgressLoggedRef.current = true;
        logAudioDebug("audio-current-time-advanced", {
          anchoredTransportTime: Number(audioTime.toFixed(3)),
        });
      }
    }
    nextTime = clampTransportTime(nextTime);
    playbackTimeRef.current = nextTime;
    if (nextTime >= totalDurationSeconds - 1 / 60) {
      stopPlayback(0);
      return;
    }
    if (nextTime >= leadInDuration && !audioStartedRef.current && !audioStartPendingRef.current) {
      if (!audioHandoffLoggedRef.current) {
        audioHandoffLoggedRef.current = true;
        logAudioDebug("lead-in-boundary-crossed", {
          leadInDuration: Number(leadInDuration.toFixed(3)),
          totalDurationSeconds: Number(totalDurationSeconds.toFixed(3)),
        });
      }
      void startAudioAtTransportTime(nextTime);
    }
    transportFrameRef.current = requestAnimationFrame(runTransportFrame);
  }

  async function startPlayback(startTime = playbackTimeRef.current) {
    const normalizedStart = clampTransportTime(startTime >= totalDurationSeconds ? 0 : startTime);
    stopTransportFrame();
    playbackTimeRef.current = normalizedStart;
    syncAudioToTransport(normalizedStart, true);
    audioStartedRef.current = false;
    setIsPlaying(true);
    setPlayPending(false);
    transportOriginTimeRef.current = normalizedStart;
    transportStartedAtRef.current = performance.now();
    if (normalizedStart >= leadInDuration) {
      const started = await startAudioAtTransportTime(normalizedStart);
      if (!started) {
        return;
      }
      const audio = audioRef.current;
      if (audio) {
        playbackTimeRef.current = clampTransportTime(audio.currentTime + audioTransportAnchorRef.current);
        transportOriginTimeRef.current = playbackTimeRef.current;
        transportStartedAtRef.current = performance.now();
      }
    }
    transportFrameRef.current = requestAnimationFrame(runTransportFrame);
  }

  useEffect(() => {
    stopPlayback(0);
    setAudioReady(!effectiveAudioUrl);
    setAudioDuration(null);
    audioTransportAnchorRef.current = 0;
    audioHandoffLoggedRef.current = false;
    audioFirstProgressLoggedRef.current = false;
  }, [effectiveAudioUrl]);

  useEffect(() => {
    if (!playPending || !previewReady || !audioReady) return;
    const run = async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      void startPlayback();
    };
    void run();
  }, [audioReady, playPending, previewReady]);

  useEffect(() => {
    if (isPlaying) {
      return;
    }
    playbackTimeRef.current = clampTransportTime(playbackTimeRef.current);
  }, [isPlaying, leadInDuration, totalDurationSeconds]);

  useEffect(() => () => stopTransportFrame(), []);

  useEffect(() => {
    const next = selectedPlacementIndices.filter((index) => index >= 0 && index < timeline.arrangement.length);
    if (next.length === selectedPlacementIndices.length) {
      return;
    }
    setSelectedPlacementIndices(next);
    setPlacementSelectionAnchor(next[0] ?? null);
  }, [selectedPlacementIndices, timeline.arrangement.length]);

  useEffect(() => {
    setTimelineZoom((current) => Math.max(current, minTimelineZoom));
  }, [minTimelineZoom]);

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
    setStatus(`Copied ${placements.length} placement${placements.length === 1 ? "" : "s"}.`, "success");
    toast.success(`Copied ${placements.length} placement${placements.length === 1 ? "" : "s"}.`);
  }

  function setStatus(nextMessage: string, tone: MessageTone = "info") {
    setMessage(nextMessage);
    setMessageTone(tone);
  }

  function currentBeat(): number {
    return Math.max(minPlacementBeat, songBeatFromTransportTime(playbackTimeRef.current, timeline));
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

  function updateDraft(mutator: (current: TrackVisualizerConfig) => TrackVisualizerConfig) {
    setDraft((current) => mutator(structuredClone(current)));
  }

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select") || target?.isContentEditable) {
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
    const savedSnapshot = serializeVisualizer(normalized);
    setLoaded({
      audioUrl: options.audioUrl ?? null,
      name: options.name,
      savedSnapshot,
      visualizer: normalized,
    });
    setDraft(normalized);
    setSelectedClipId(nextTimeline.clips[0]?.id ?? null);
    clearPlacementSelection();
    setSelectedLane(visibleLane(primaryLane(nextTimeline.clips[0])?.lane));
    stopPlayback(0);
    setImportedAudioUrl(null);
    setStatus(options.status, "success");
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
      const failureMessage = error instanceof Error ? error.message : "Load failed.";
      setStatus(failureMessage, "error");
      toast.error("Load failed.", {
        description: failureMessage,
      });
    } finally {
      input.value = "";
    }
  }

  function handleNewVisualizer() {
    const visualizer = normalizedConfig(defaultVisualizer());
    setLoaded({
      audioUrl: null,
      name: "tty0-visualizer.json",
      savedSnapshot: serializeVisualizer(visualizer),
      visualizer,
    });
    setDraft(visualizer);
    setSelectedLane("motion_rate");
    setSelectedClipId(timelineFromConfig(visualizer).clips[0]?.id ?? null);
    clearPlacementSelection();
    stopPlayback(0);
    setImportedAudioUrl(null);
    setStatus("Started a new visualizer draft.", "success");
  }

  function handleImportAudio(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    if (importedAudioUrl) {
      URL.revokeObjectURL(importedAudioUrl);
    }
    const url = URL.createObjectURL(file);
    setImportedAudioUrl(url);
    setStatus(`Mounted audio file ${file.name}.`, "success");
    toast.success("Imported audio.", {
      description: file.name,
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
    setDraft(normalized);
    setLoaded((current) => ({
      audioUrl: current?.audioUrl ?? null,
      name: jsonFilename(current?.name),
      savedSnapshot: serializeVisualizer(normalized),
      visualizer: normalized,
    }));
    setStatus(`Saved ${jsonFilename(loaded?.name)}.`, "success");
    toast.success("Saved JSON.", {
      description: jsonFilename(loaded?.name),
    });
  }

  function revertToLoaded() {
    if (!loaded) return;
    setDraft(loaded.visualizer);
    clearPlacementSelection();
    setStatus(`Reverted ${loaded.name}.`, "warning");
  }

  function commitSelectedClipShape(points: LfoPoint[]) {
    if (!selectedClip || !selectedClip.source || selectedClip.source.kind !== "lfo") return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.source || clip.source.kind !== "lfo") return current;
      clip.source.shape.points = normalizeClipLfoShape({ interpolation: "linear", points }).points;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function updateSelectedClipBeatValue(rawValue: number) {
    const nextValue = Math.max(0.25, rawValue);
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip?.id);
      if (!clip) return current;
      clip.length_beats = nextValue;
      if (clip.source?.kind === "lfo") {
        clip.source.period_beats = nextValue;
      }
      current.timeline = nextTimeline;
      return current;
    });
  }

  function seekToTime(nextTime: number) {
    const clampedTime = clampTransportTime(nextTime);
    playbackTimeRef.current = clampedTime;
    transportOriginTimeRef.current = clampedTime;
    transportStartedAtRef.current = performance.now();
    if (!isPlaying) {
      audioStartedRef.current = false;
      audioStartPendingRef.current = false;
      audioTransportAnchorRef.current = clampedTime - (audioRef.current?.currentTime ?? 0);
      const audio = audioRef.current;
      if (audio) {
        const targetAudioTime = audioTimeFromTransportTime(clampedTime, timeline);
        if (Math.abs(audio.currentTime - targetAudioTime) > 0.05) {
          audio.currentTime = targetAudioTime;
        }
        audioTransportAnchorRef.current = clampedTime - audio.currentTime;
      }
      return;
    }
    syncAudioToTransport(clampedTime, true);
    if (clampedTime < leadInDuration) {
      audioStartedRef.current = false;
      audioStartPendingRef.current = false;
      audioTransportAnchorRef.current = 0;
      return;
    }
    const audio = audioRef.current;
    if (audio && !audio.paused) {
      audioStartedRef.current = true;
      audioTransportAnchorRef.current = clampedTime - audio.currentTime;
      return;
    }
    void startAudioAtTransportTime(clampedTime);
  }

  function seekToBeat(displayBeat: number) {
    const nextBeat = Math.max(0, Math.min(totalVisibleBeats, displayBeat));
    seekToTime(transportTimeFromDisplayBeat(nextBeat, timeline));
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
  }

  function handleTogglePlayback() {
    if (!effectiveAudioUrl) return;
    if (isPlaying) {
      pausePlayback();
      return;
    }
    if (playbackTimeRef.current < leadInDuration) {
      void primeAudioOutputPath();
    }
    if (!previewReady || !audioReady) {
      setPlayPending(true);
      setStatus("Preparing preview and audio before playback.");
      return;
    }
    void startPlayback();
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
      return current;
    });
    setStatus("Added a new clip.", "success");
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
    setStatus("Duplicated selected clip.", "success");
  }

  function deleteClip() {
    if (!selectedClip) return;
    if (timeline.arrangement.some((placement) => placement.clip_id === selectedClip.id)) {
      setStatus("Delete blocked: clip is still placed on the song timeline.", "warning");
      toast.warning("Clip delete blocked.", {
        description: "Remove timeline placements first.",
      });
      return;
    }
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.clips = nextTimeline.clips.filter((clip) => clip.id !== selectedClip.id);
      current.timeline = nextTimeline;
      setSelectedClipId(
        nextTimeline.clips.find((clip) => primaryLane(clip)?.lane === selectedLane)?.id ?? nextTimeline.clips[0]?.id ?? null,
      );
      return current;
    });
    setStatus("Deleted selected clip.", "warning");
  }

  function placeSelectedClipAtBeat(beat: number) {
    if (!selectedClip) return;
    const primary = primaryLane(selectedClip);
    if (!primary) {
      setStatus("Selected clip has no authored lane.", "warning");
      return;
    }
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.arrangement.push(createPlacement(selectedClip, snapBeatToGrid(beat, totalTimelineBeats, minPlacementBeat)));
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

  function clipDropPosition(event: DragEvent<HTMLButtonElement>): "before" | "after" {
    const rect = event.currentTarget.getBoundingClientRect();
    return event.clientY < rect.top + rect.height / 2 ? "before" : "after";
  }

  function reorderClip(draggedId: string, targetId: string, position: "before" | "after") {
    if (draggedId === targetId) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const fromIndex = nextTimeline.clips.findIndex((clip) => clip.id === draggedId);
      const targetIndex = nextTimeline.clips.findIndex((clip) => clip.id === targetId);
      if (fromIndex < 0 || targetIndex < 0) {
        return current;
      }
      const [draggedClip] = nextTimeline.clips.splice(fromIndex, 1);
      let insertIndex = targetIndex + (position === "after" ? 1 : 0);
      if (fromIndex < targetIndex) {
        insertIndex -= 1;
      }
      nextTimeline.clips.splice(Math.max(0, insertIndex), 0, draggedClip);
      current.timeline = nextTimeline;
      return current;
    });
  }

  function handleClipDragStart(event: DragEvent<HTMLButtonElement>, clipId: string) {
    setDraggedClipId(clipId);
    setClipDropIndicator(null);
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", clipId);
  }

  function handleClipDragOver(event: DragEvent<HTMLButtonElement>, targetId: string) {
    if (!draggedClipId || draggedClipId === targetId) {
      setClipDropIndicator(null);
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setClipDropIndicator({
      clipId: targetId,
      position: clipDropPosition(event),
    });
  }

  function handleClipDrop(event: DragEvent<HTMLButtonElement>, targetId: string) {
    if (!draggedClipId || draggedClipId === targetId) {
      setDraggedClipId(null);
      setClipDropIndicator(null);
      return;
    }
    event.preventDefault();
    reorderClip(draggedClipId, targetId, clipDropPosition(event));
    setDraggedClipId(null);
    setClipDropIndicator(null);
  }

  function clearClipDragState() {
    setDraggedClipId(null);
    setClipDropIndicator(null);
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
          ...createPlacement(clip, snapBeatToGrid(atBeat + entry.offsetBeats, totalTimelineBeats, minPlacementBeat)),
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
        setStatus(`Pasted ${pastedIndices.length} placement${pastedIndices.length === 1 ? "" : "s"}.`, "success");
        toast.success(`Pasted ${pastedIndices.length} placement${pastedIndices.length === 1 ? "" : "s"}.`);
      }
      return current;
    });
  }

  function changeBaseValue(lane: LaneId, value: number | [number, number, number]) {
    updateDraft((current) => {
      const state = current.params.shader_states?.playing;
      if (!state) return current;
      state[lane] = value as never;
      return current;
    });
  }

  function changeClipName(value: string) {
    if (!selectedClip) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip) return current;
      clip.name = value;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeClipStartValue(value: number | ColorValue) {
    if (!selectedClip) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.source || clip.source.kind !== "lfo") return current;
      clip.source.start = value as never;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeClipEndValue(value: number | ColorValue) {
    if (!selectedClip) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.source || clip.source.kind !== "lfo") return current;
      clip.source.end = value as never;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeClipHoldAfter(value: boolean) {
    if (!selectedClip) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip) return current;
      clip.hold_after = value;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeTimelineBpm(value: number) {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.bpm = Math.max(1, value);
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeTimelineMeasures(value: number) {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.measures = Math.max(1, Math.round(value));
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeTimelineBeatsPerMeasure(value: number) {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.beats_per_measure = Math.max(1, Math.round(value));
      current.timeline = nextTimeline;
      return current;
    });
  }

  function changeTimelineLeadInBars(value: number) {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.lead_in_bars = Math.max(0, Math.round(value));
      current.timeline = nextTimeline;
      return current;
    });
  }

  function selectClip(clipId: string, lane: LaneId) {
    setSelectedClipId(clipId);
    setSelectedLane(lane);
    clearPlacementSelection();
  }

  function selectLane(lane: LaneId) {
    setSelectedLane(lane);
    clearPlacementSelection();
  }

  function resetArrangementSelection() {
    clearPlacementSelection();
    setSelectedClipId(null);
    setSelectedPointIndex(null);
  }

  return {
    documentState: {
      loaded,
      draft,
      dirty,
      message,
      messageTone,
      fileInputRef,
      audioInputRef,
      effectiveAudioUrl,
    },
    documentActions: {
      handleImportRecord,
      handleNewVisualizer,
      handleImportAudio,
      handleSave,
      revertToLoaded,
      setMessage: setStatus,
    },
    selectionState: {
      selectedLane,
      selectedLaneMeta,
      selectedClip,
      selectedShape,
      selectedPointIndex,
      selectedPlacementIndices,
      selectedPlacementSet,
      clipDropIndicator,
      draggedClipId,
    },
    selectionActions: {
      setSelectedLane,
      setSelectedPointIndex,
      clearPlacementSelection,
      selectPlacement,
      selectClip,
      selectLane,
      resetArrangementSelection,
    },
    arrangementState: {
      timeline,
      timelineIndex,
      timelineZoom,
      timelineTool,
      minTimelineZoom,
      maxTimelineZoom,
      timelineWidth,
      arrangementViewportWidth,
      totalDurationSeconds,
      playPending,
      isPlaying,
      audioReady,
      previewReady,
      audioDuration,
    },
    arrangementActions: {
      setArrangementViewportWidth,
      setTimelineZoom,
      setTimelineTool,
      updatePlacementAtIndex,
      placeSelectedClipAtBeat,
      seekToBeat,
      seekToTime,
      copySelectedPlacements,
      pasteCopiedPlacements,
      deleteSelectedPlacements,
      addPlacement,
      stopPlayback,
      handleTogglePlayback,
      setPreviewReady,
      setAudioReady,
      setAudioDuration,
    },
    clipState: {
      laneClips,
      clipLengthOptions,
      selectedClipBeatValue,
      selectedClipBeatOption,
      baseState,
      hasSelectedClip: Boolean(selectedClip),
    },
    clipActions: {
      addClip,
      duplicateClip,
      deleteClip,
      handleClipDragStart,
      handleClipDragOver,
      handleClipDrop,
      clearClipDragState,
      changeBaseValue,
      changeClipName,
      updateSelectedClipBeatValue,
      changeClipStartValue,
      changeClipEndValue,
      changeClipHoldAfter,
      changeTimelineBpm,
      changeTimelineMeasures,
      changeTimelineBeatsPerMeasure,
      changeTimelineLeadInBars,
    },
    shapeState: {
      selectedPointIndex,
      selectedShape,
    },
    shapeActions: {
      setSelectedPointIndex,
      commitSelectedClipShape,
    },
    previewState: {
      audioRef,
      playbackTimeRef,
      isPlaying,
      previewReady,
      audioReady,
      effectiveAudioUrl,
      dirty,
      totalDurationSeconds,
    },
    editorState: {
      draft,
      loaded,
      selectedLane,
      selectedClip,
      selectedPlacementIndices,
      selectedPlacementSet,
      timeline,
      timelineIndex,
      timelineWidth,
      timelineZoom,
      minTimelineZoom,
      maxTimelineZoom,
      clipLengthOptions,
      selectedClipBeatOption,
      selectedClipBeatValue,
      laneClips,
      draggedClipId,
      clipDropIndicator,
      baseState,
      selectedLaneMeta,
      totalDurationSeconds,
      timelineTool,
      message,
    },
    constants: {
      editorLanes: EDITOR_LANES,
    },
  };
}
