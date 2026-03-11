import { ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import {
  ClipPlacement,
  ClipTweenStep,
  LaneId,
  LoadedRecord,
  ProjectHandle,
  RecordSummary,
  TrackVisualizerConfig,
} from "./types";
import {
  createPlacement,
  defaultStep,
  defaultVisualizer,
  isColorLane,
  LANE_ORDER,
  normalizedConfig,
  primaryLane,
  resolveChromaticBulgeGrid,
  syncClipAuthoring,
  timelineFromConfig,
  totalBeats,
} from "./vfx";
import {
  importRecordFromJson,
  isTauri,
  listRecords,
  loadRecord,
  pickProjectRoot,
  revealRecordFile,
  saveRecordVisualizer,
} from "./platform";
import { FRAGMENT_SHADER, VERTEX_SHADER } from "./shaders";

type DragState =
  | { kind: "move"; placementIndex: number; offsetBeats: number }
  | { kind: "resize"; placementIndex: number };

const TRACK_HEIGHT = 28;

export default function App() {
  const [project, setProject] = useState<ProjectHandle | null>(null);
  const [records, setRecords] = useState<RecordSummary[]>([]);
  const [loaded, setLoaded] = useState<LoadedRecord | null>(null);
  const [draft, setDraft] = useState<TrackVisualizerConfig>(defaultVisualizer());
  const [selectedClipId, setSelectedClipId] = useState<string | null>(null);
  const [selectedPlacementIndex, setSelectedPlacementIndex] = useState<number | null>(null);
  const [selectedStepIndex, setSelectedStepIndex] = useState<number | null>(null);
  const [message, setMessage] = useState<string>("Open a tty0 project root or import a record JSON.");
  const [playbackTime, setPlaybackTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [dragState, setDragState] = useState<DragState | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const arrangementRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const timeline = useMemo(() => timelineFromConfig(draft), [draft]);
  const totalTimelineBeats = totalBeats(timeline);
  const selectedClip = timeline.clips.find((clip) => clip.id === selectedClipId) ?? timeline.clips[0] ?? null;
  const selectedTrack = selectedClip?.authoring?.tracks[0];
  const selectedStep = selectedTrack?.steps?.[selectedStepIndex ?? 0] ?? null;
  const dirty = loaded ? JSON.stringify(normalizedConfig(draft)) !== JSON.stringify(normalizedConfig(loaded.visualizer)) : false;

  useEffect(() => {
    if (!selectedClip && timeline.clips[0]) {
      setSelectedClipId(timeline.clips[0].id);
    }
  }, [selectedClip, timeline.clips]);

  useEffect(() => {
    let frame = 0;
    const tick = () => {
      if (audioRef.current && isPlaying) {
        setPlaybackTime(audioRef.current.currentTime);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [isPlaying]);

  async function handleOpenProject() {
    const handle = await pickProjectRoot();
    if (!handle) {
      return;
    }
    const nextRecords = await listRecords(handle.root);
    setProject(handle);
    setRecords(nextRecords);
    setMessage(`Opened ${handle.root}`);
  }

  async function handleLoadRecord(recordId: string) {
    if (!project) return;
    const next = await loadRecord(project.root, recordId);
    setLoaded(next);
    setDraft(normalizedConfig(next.visualizer));
    setSelectedClipId(timelineFromConfig(next.visualizer).clips[0]?.id ?? null);
    setSelectedPlacementIndex(null);
    setSelectedStepIndex(null);
    setPlaybackTime(0);
    setIsPlaying(false);
    setMessage(next.has_legacy_automation ? "Loaded record and migrated legacy automation into timeline draft." : `Loaded ${next.record_id}`);
  }

  async function handleImportRecord(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    const next = await importRecordFromJson(file);
    setLoaded(next);
    setDraft(normalizedConfig(next.visualizer));
    setSelectedClipId(timelineFromConfig(next.visualizer).clips[0]?.id ?? null);
    setMessage(`Imported ${next.record_id} in browser mode.`);
  }

  async function handleSave() {
    if (!loaded) return;
    const normalized = normalizedConfig(draft);
    if (project && isTauri()) {
      const result = await saveRecordVisualizer(project.root, loaded.record_id, normalized);
      setLoaded({ ...loaded, visualizer: normalized });
      setMessage(result);
      return;
    }
    const blob = new Blob([JSON.stringify(normalized, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${loaded.record_id}.visualizer.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    setMessage("Exported visualizer JSON.");
  }

  function updateDraft(mutator: (current: TrackVisualizerConfig) => TrackVisualizerConfig) {
    setDraft((current) => normalizedConfig(mutator(structuredClone(current))));
  }

  function currentBeat(): number {
    return Math.max(0, playbackTime) * timeline.bpm / 60;
  }

  function addClip() {
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clipId = `clip_${nextTimeline.clips.length + 1}`;
      nextTimeline.clips.push({
        id: clipId,
        name: `Clip ${nextTimeline.clips.length + 1}`,
        length_beats: 4,
        color: [0.43, 0.86, 0.83],
        authoring: {
          tracks: [{ lane: "motion_rate", steps: [] }],
        },
        lanes: {},
      });
      current.timeline = nextTimeline;
      setSelectedClipId(clipId);
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
      setSelectedClipId(nextTimeline.clips[0]?.id ?? null);
      return current;
    });
  }

  function addPlacement(lane: LaneId) {
    if (!selectedClip) return;
    const primary = primaryLane(selectedClip);
    if (!primary || primary.lane !== lane) {
      setMessage(`Selected clip belongs on ${primary?.lane ?? "its authored"} lane.`);
      return;
    }
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.arrangement.push(createPlacement(selectedClip, Math.floor(currentBeat() * 4) / 4));
      current.timeline = nextTimeline;
      return current;
    });
  }

  function updateSelectedPlacement(next: ClipPlacement) {
    if (selectedPlacementIndex == null) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      nextTimeline.arrangement[selectedPlacementIndex] = next;
      current.timeline = nextTimeline;
      return current;
    });
  }

  function updateSelectedStep(nextStep: ClipTweenStep) {
    if (!selectedClip || !selectedTrack || selectedStepIndex == null) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.authoring?.tracks[0]) return current;
      clip.authoring.tracks[0].steps[selectedStepIndex] = nextStep;
      current.timeline = nextTimeline;
      return syncClipAuthoring(current, clip.id);
    });
  }

  function addStep() {
    if (!selectedClip || !selectedTrack) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.authoring?.tracks[0]) return current;
      clip.authoring.tracks[0].steps.push(defaultStep(clip.authoring.tracks[0].lane));
      current.timeline = nextTimeline;
      setSelectedStepIndex(clip.authoring.tracks[0].steps.length - 1);
      return syncClipAuthoring(current, clip.id);
    });
  }

  function deleteStep() {
    if (!selectedClip || !selectedTrack || selectedStepIndex == null) return;
    updateDraft((current) => {
      const nextTimeline = timelineFromConfig(current);
      const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
      if (!clip?.authoring?.tracks[0]) return current;
      clip.authoring.tracks[0].steps.splice(selectedStepIndex, 1);
      current.timeline = nextTimeline;
      setSelectedStepIndex(Math.max(0, selectedStepIndex - 1));
      return syncClipAuthoring(current, clip.id);
    });
  }

  const resolved = resolveChromaticBulgeGrid(draft, {
    currentTimeSecs: playbackTime,
    visualTimeSecs: playbackTime,
    isPlaying,
    timelinePreview: true,
  });

  return (
    <div className="app-shell">
      <audio
        ref={audioRef}
        src={loaded?.audio_url ?? undefined}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onEnded={() => {
          setIsPlaying(false);
          setPlaybackTime(0);
        }}
      />
      <header className="topbar">
        <div>
          <h1>tty0 VFX Editor</h1>
          <p>{project?.root ?? "Browser import/export mode"}</p>
        </div>
        <div className="topbar-actions">
          <button onClick={handleOpenProject} disabled={!isTauri()}>
            Open Project
          </button>
          <button onClick={() => fileInputRef.current?.click()}>Import JSON</button>
          <button onClick={handleSave} disabled={!loaded}>
            Save
          </button>
          <button
            onClick={() => loaded && setDraft(normalizedConfig(loaded.visualizer))}
            disabled={!loaded || !dirty}
          >
            Revert
          </button>
          <button
            onClick={() => {
              if (project && loaded) void revealRecordFile(project.root, loaded.record_id);
            }}
            disabled={!project || !loaded || !isTauri()}
          >
            Reveal File
          </button>
        </div>
        <div className="transport">
          <button
            onClick={() => {
              if (!audioRef.current) return;
              if (audioRef.current.paused) {
                void audioRef.current.play();
              } else {
                audioRef.current.pause();
              }
            }}
            disabled={!loaded?.audio_url}
          >
            {isPlaying ? "Pause" : "Play"}
          </button>
          <button
            onClick={() => {
              if (!audioRef.current) return;
              audioRef.current.pause();
              audioRef.current.currentTime = 0;
              setPlaybackTime(0);
            }}
            disabled={!loaded?.audio_url}
          >
            Stop
          </button>
          <input
            type="range"
            min={0}
            max={audioRef.current?.duration || (timeline.measures * timeline.beats_per_measure * 60) / timeline.bpm || 1}
            step={0.01}
            value={playbackTime}
            onChange={(event) => {
              const nextTime = Number(event.target.value);
              setPlaybackTime(nextTime);
              if (audioRef.current) {
                audioRef.current.currentTime = nextTime;
              }
            }}
          />
          <span>
            beat {resolved.currentBeat.toFixed(2)} / measure {(resolved.currentBeat / timeline.beats_per_measure + 1).toFixed(2)}
          </span>
          <span>{dirty ? "dirty" : "saved"}</span>
        </div>
        <input
          ref={fileInputRef}
          type="file"
          accept=".json,application/json"
          hidden
          onChange={handleImportRecord}
        />
      </header>

      <div className="message-bar">{message}</div>

      <main className="workspace">
        <aside className="sidebar">
          <section className="panel">
            <h2>Records</h2>
            <div className="list">
              {records.map((record) => (
                <button
                  key={record.record_id}
                  className={`list-item ${loaded?.record_id === record.record_id ? "selected" : ""}`}
                  onClick={() => void handleLoadRecord(record.record_id)}
                >
                  <strong>{record.record_id}</strong>
                  <span>{record.title}</span>
                </button>
              ))}
            </div>
          </section>
          <section className="panel">
            <div className="panel-header">
              <h2>Clips</h2>
              <div className="inline-actions">
                <button onClick={addClip}>Add</button>
                <button onClick={duplicateClip} disabled={!selectedClip}>Copy</button>
                <button onClick={deleteClip} disabled={!selectedClip}>Delete</button>
              </div>
            </div>
            <div className="list">
              {timeline.clips.map((clip) => (
                <button
                  key={clip.id}
                  className={`clip-card ${selectedClipId === clip.id ? "selected" : ""}`}
                  onClick={() => setSelectedClipId(clip.id)}
                >
                  <span
                    className="clip-swatch"
                    style={{ background: `rgb(${clip.color.map((channel) => Math.round(channel * 255)).join(" ")})` }}
                  />
                  <div>
                    <strong>{clip.name}</strong>
                    <span>{clip.length_beats.toFixed(2)} beats</span>
                  </div>
                </button>
              ))}
            </div>
          </section>
        </aside>

        <section className="editor-column">
          <section className="panel arrangement-panel">
            <div className="panel-header">
              <h2>Song Timeline</h2>
              <span>
                {timeline.bpm.toFixed(1)} BPM • {timeline.measures} bars • {timeline.beats_per_measure}/4
              </span>
            </div>
            <div
              ref={arrangementRef}
              className="arrangement-grid"
              onPointerMove={(event) => {
                if (!dragState || !arrangementRef.current) return;
                const rect = arrangementRef.current.getBoundingClientRect();
                const beat = ((event.clientX - rect.left) / rect.width) * totalTimelineBeats;
                if (dragState.kind === "move") {
                  const placement = timeline.arrangement[dragState.placementIndex];
                  updateSelectedPlacement({
                    ...placement,
                    start_beat: Math.max(0, Math.min(totalTimelineBeats, Math.round((beat - dragState.offsetBeats) * 4) / 4)),
                  });
                } else {
                  const placement = timeline.arrangement[dragState.placementIndex];
                  const clip = timeline.clips.find((candidate) => candidate.id === placement.clip_id);
                  if (!clip) return;
                  const widthBeats = Math.max(clip.length_beats, beat - placement.start_beat);
                  updateSelectedPlacement({
                    ...placement,
                    repeats: Math.max(1, Math.round(widthBeats / clip.length_beats)),
                  });
                }
              }}
              onPointerUp={() => setDragState(null)}
              onPointerLeave={() => setDragState(null)}
            >
              {LANE_ORDER.map((lane, index) => (
                <div key={lane} className="track-row">
                  <button className="track-label" onClick={() => addPlacement(lane)}>
                    {lane}
                  </button>
                  <div className="track-lane" style={{ height: TRACK_HEIGHT }}>
                    {timeline.arrangement
                      .map((placement, placementIndex) => ({ placement, placementIndex }))
                      .filter(({ placement }) => placement.track === index)
                      .map(({ placement, placementIndex }) => {
                        const clip = timeline.clips.find((candidate) => candidate.id === placement.clip_id);
                        if (!clip) return null;
                        const left = (placement.start_beat / totalTimelineBeats) * 100;
                        const width = ((clip.length_beats * placement.repeats) / totalTimelineBeats) * 100;
                        return (
                          <div
                            key={`${placement.clip_id}-${placementIndex}`}
                            className={`placement ${selectedPlacementIndex === placementIndex ? "selected" : ""}`}
                            style={{
                              left: `${left}%`,
                              width: `${width}%`,
                              background: `rgb(${clip.color.map((channel) => Math.round(channel * 255)).join(" ")})`,
                            }}
                            onPointerDown={(event) => {
                              event.stopPropagation();
                              const rect = (event.currentTarget as HTMLDivElement).getBoundingClientRect();
                              const hitEdge = rect.right - event.clientX < 12;
                              setSelectedPlacementIndex(placementIndex);
                              const beatAtCursor = ((event.clientX - rect.left) / rect.width) * (clip.length_beats * placement.repeats);
                              setDragState(
                                hitEdge
                                  ? { kind: "resize", placementIndex }
                                  : { kind: "move", placementIndex, offsetBeats: beatAtCursor },
                              );
                            }}
                          >
                            <span>{clip.name}</span>
                            <div className="resize-handle" />
                          </div>
                        );
                      })}
                  </div>
                </div>
              ))}
              <div className="playhead" style={{ left: `${(resolved.currentBeat / totalTimelineBeats) * 100}%` }} />
            </div>
          </section>

          <section className="panel clip-editor-panel">
            <div className="panel-header">
              <h2>Clip Editor</h2>
              {selectedClip ? <span>{selectedClip.name}</span> : <span>No clip selected</span>}
            </div>
            {selectedClip ? (
              <div className="clip-editor">
                <div className="step-list">
                  <label>
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
                          return syncClipAuthoring(current, clip.id);
                        })
                      }
                    />
                  </label>
                  <label>
                    Lane
                    <select
                      value={selectedTrack?.lane}
                      onChange={(event) =>
                        updateDraft((current) => {
                          const nextTimeline = timelineFromConfig(current);
                          const clip = nextTimeline.clips.find((candidate) => candidate.id === selectedClip.id);
                          if (!clip) return current;
                          clip.authoring = {
                            tracks: [
                              {
                                lane: event.target.value as LaneId,
                                steps: clip.authoring?.tracks[0]?.steps ?? [],
                              },
                            ],
                          };
                          current.timeline = nextTimeline;
                          return syncClipAuthoring(current, clip.id);
                        })
                      }
                    >
                      {LANE_ORDER.map((lane) => (
                        <option key={lane} value={lane}>
                          {lane}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="inline-actions">
                    <button onClick={addStep}>Add Step</button>
                    <button onClick={deleteStep} disabled={selectedStepIndex == null}>
                      Delete Step
                    </button>
                  </div>
                  <div className="list compact">
                    {(selectedTrack?.steps ?? []).map((step, index) => (
                      <button
                        key={`${index}-${step.duration_beats}`}
                        className={`list-item ${selectedStepIndex === index ? "selected" : ""}`}
                        onClick={() => setSelectedStepIndex(index)}
                      >
                        <strong>{index + 1}</strong>
                        <span>{step.ease} • {step.duration_beats.toFixed(2)} beats</span>
                      </button>
                    ))}
                  </div>
                </div>
                <div className="step-inspector">
                  <h3>Selected Step</h3>
                  {selectedStep ? (
                    <>
                      <label>
                        Duration
                        <input
                          type="number"
                          step={0.25}
                          value={selectedStep.duration_beats}
                          onChange={(event) =>
                            updateSelectedStep({
                              ...selectedStep,
                              duration_beats: Math.max(0, Number(event.target.value)),
                            })
                          }
                        />
                      </label>
                      <label>
                        Ease
                        <select
                          value={selectedStep.ease}
                          onChange={(event) =>
                            updateSelectedStep({
                              ...selectedStep,
                              ease: event.target.value as ClipTweenStep["ease"],
                            })
                          }
                        >
                          {["hold", "linear", "sine_in", "sine_out", "sine_in_out"].map((ease) => (
                            <option key={ease} value={ease}>
                              {ease}
                            </option>
                          ))}
                        </select>
                      </label>
                      {selectedStep.to.kind === "float" ? (
                        <label>
                          Target Value
                          <input
                            type="number"
                            step={0.01}
                            value={selectedStep.to.value}
                            onChange={(event) =>
                              updateSelectedStep({
                                ...selectedStep,
                                to: { kind: "float", value: Number(event.target.value) },
                              })
                            }
                          />
                        </label>
                      ) : (
                        selectedStep.to.value.map((channel, channelIndex) => (
                          <label key={channelIndex}>
                            Color {channelIndex + 1}
                            <input
                              type="number"
                              step={0.01}
                              min={0}
                              max={1}
                              value={channel}
                              onChange={(event) => {
                                const nextColor = Array.from(
                                  selectedStep.to.value as [number, number, number],
                                ) as [number, number, number];
                                nextColor[channelIndex] = Number(event.target.value);
                                updateSelectedStep({
                                  ...selectedStep,
                                  to: { kind: "color", value: nextColor },
                                });
                              }}
                            />
                          </label>
                        ))
                      )}
                    </>
                  ) : (
                    <p>Select a tween step to edit.</p>
                  )}
                </div>
              </div>
            ) : (
              <p>Select or create a clip.</p>
            )}
          </section>
        </section>

        <aside className="preview-column">
          <section className="panel preview-panel">
            <h2>Preview</h2>
            <PreviewCanvas uniforms={resolved.uniforms} time={playbackTime} />
            <div className="preview-stats">
              <div>
                <strong>{loaded?.record_title ?? "No record loaded"}</strong>
                <span>{loaded?.audio_url ? "Audio mounted" : "Audio missing or browser import mode"}</span>
              </div>
              <dl>
                <div>
                  <dt>circle_radius</dt>
                  <dd>{resolved.uniforms.circle_radius.toFixed(3)}</dd>
                </div>
                <div>
                  <dt>bulge_amount</dt>
                  <dd>{resolved.uniforms.bulge_amount.toFixed(3)}</dd>
                </div>
                <div>
                  <dt>chrom_ab</dt>
                  <dd>{resolved.uniforms.chromatic_aberration.toFixed(3)}</dd>
                </div>
              </dl>
            </div>
          </section>
        </aside>
      </main>
    </div>
  );
}

function PreviewCanvas({
  uniforms,
  time,
}: {
  uniforms: ReturnType<typeof resolveChromaticBulgeGrid>["uniforms"];
  time: number;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

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
    const render = () => {
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
      setUniform1f(gl, program, "u_time", time);
      setUniform1f(gl, program, "u_motion_rate", uniforms.motion_rate);
      setUniform1f(gl, program, "u_lattice_density", uniforms.lattice_density);
      setUniform1f(gl, program, "u_circle_radius", uniforms.circle_radius);
      setUniform1f(gl, program, "u_circle_falloff_start", uniforms.circle_falloff_start);
      setUniform1f(gl, program, "u_circle_falloff_end", uniforms.circle_falloff_end);
      setUniform1f(gl, program, "u_bulge_amount", uniforms.bulge_amount);
      setUniform1f(gl, program, "u_rim_guard", uniforms.rim_guard);
      setUniform1f(gl, program, "u_rim_exponent", uniforms.rim_exponent);
      setUniform1f(gl, program, "u_rim_warp", uniforms.rim_warp);
      setUniform1f(gl, program, "u_spacing_max_px", uniforms.spacing_max_px);
      setUniform1f(gl, program, "u_spacing_min_px", uniforms.spacing_min_px);
      setUniform1f(gl, program, "u_dot_size", uniforms.dot_size);
      setUniform1f(gl, program, "u_outer_dot_scale", uniforms.outer_dot_scale);
      setUniform1f(gl, program, "u_edge_softness", uniforms.edge_softness);
      setUniform1f(gl, program, "u_chromatic_aberration", uniforms.chromatic_aberration);
      setUniform1f(gl, program, "u_scroll_base", uniforms.scroll_base);
      setUniform1f(gl, program, "u_scroll_motion_scale", uniforms.scroll_motion_scale);
      setUniform1f(gl, program, "u_scroll_motion_floor", uniforms.scroll_motion_floor);
      setUniform1f(gl, program, "u_scroll_motion_ceiling", uniforms.scroll_motion_ceiling);
      setUniform3f(gl, program, "u_cold_color", uniforms.cold_color);
      setUniform3f(gl, program, "u_hot_color", uniforms.hot_color);
      setUniform1f(gl, program, "u_color_cycle_rate", uniforms.color_cycle_rate);
      setUniform1f(gl, program, "u_inner_alpha", uniforms.inner_alpha);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    };
    render();
    return () => {
      gl.deleteProgram(program);
      if (vao) gl.deleteVertexArray(vao);
    };
  }, [time, uniforms]);

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
