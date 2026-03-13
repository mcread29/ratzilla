import { useEffect, useRef } from "react";
import { FRAGMENT_SHADER, VERTEX_SHADER } from "../../shaders";
import { TrackVisualizerConfig } from "../../types";
import { buildTimelineIndex, resolveNormalizedChromaticBulgeGrid, timelineFromConfig } from "../../vfx";
import { createProgram, setUniform1f, setUniform2f, setUniform3f } from "../preview/gl";

type MotionState = {
  offsetX: number;
  offsetY: number;
  rateX: number;
  rateY: number;
  time: number;
};

export function PreviewCanvas({
  config,
  audioRef,
  playbackTimeRef,
  isPlaying,
  onReady,
  timelineIndex,
}: {
  config: TrackVisualizerConfig;
  audioRef: { current: HTMLAudioElement | null };
  playbackTimeRef: { current: number };
  isPlaying: boolean;
  onReady: () => void;
  timelineIndex: ReturnType<typeof buildTimelineIndex>;
}) {
  void audioRef;
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const configRef = useRef(config);
  const isPlayingRef = useRef(isPlaying);
  const timelineIndexRef = useRef(timelineIndex);
  const motionStateRef = useRef<MotionState | null>(null);

  useEffect(() => {
    configRef.current = config;
    motionStateRef.current = null;
  }, [config]);

  useEffect(() => {
    isPlayingRef.current = isPlaying;
  }, [isPlaying]);

  useEffect(() => {
    timelineIndexRef.current = timelineIndex;
    motionStateRef.current = null;
  }, [timelineIndex]);

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
    const uniforms = {
      u_bulge_amount: gl.getUniformLocation(program, "u_bulge_amount"),
      u_chromatic_aberration: gl.getUniformLocation(program, "u_chromatic_aberration"),
      u_circle_falloff_end: gl.getUniformLocation(program, "u_circle_falloff_end"),
      u_circle_falloff_start: gl.getUniformLocation(program, "u_circle_falloff_start"),
      u_circle_radius: gl.getUniformLocation(program, "u_circle_radius"),
      u_cold_color: gl.getUniformLocation(program, "u_cold_color"),
      u_dot_size: gl.getUniformLocation(program, "u_dot_size"),
      u_edge_softness: gl.getUniformLocation(program, "u_edge_softness"),
      u_hot_color: gl.getUniformLocation(program, "u_hot_color"),
      u_inner_alpha: gl.getUniformLocation(program, "u_inner_alpha"),
      u_lattice_density: gl.getUniformLocation(program, "u_lattice_density"),
      u_motion_rate: gl.getUniformLocation(program, "u_motion_rate"),
      u_outer_dot_scale: gl.getUniformLocation(program, "u_outer_dot_scale"),
      u_resolution: gl.getUniformLocation(program, "u_resolution"),
      u_rim_exponent: gl.getUniformLocation(program, "u_rim_exponent"),
      u_rim_guard: gl.getUniformLocation(program, "u_rim_guard"),
      u_rim_warp: gl.getUniformLocation(program, "u_rim_warp"),
      u_time: gl.getUniformLocation(program, "u_time"),
    };

    let frame = 0;
    let announcedReady = false;
    const render = () => {
      const timeline = timelineFromConfig(configRef.current);
      const currentTime = playbackTimeRef.current;
      const currentIsPlaying = isPlayingRef.current;
      const currentUniforms = resolveNormalizedChromaticBulgeGrid(
        configRef.current,
        {
          currentTimeSecs: currentTime,
          visualTimeSecs: currentTime,
          isPlaying: currentIsPlaying,
          timelinePreview: true,
        },
        timelineIndexRef.current,
      ).uniforms;
      const motionState = resolveMotionState(
        currentTime,
        currentUniforms.motion_rate,
        currentUniforms.motion_rate_y,
        (time) =>
          resolveNormalizedChromaticBulgeGrid(
            configRef.current,
            {
              currentTimeSecs: time,
              visualTimeSecs: time,
              isPlaying: isPlayingRef.current,
              timelinePreview: true,
            },
            timelineIndexRef.current,
          ).uniforms,
        motionStateRef,
      );
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
      setUniform2f(gl, uniforms.u_resolution, width, height);
      setUniform1f(gl, uniforms.u_time, currentTime);
      setUniform2f(gl, uniforms.u_motion_rate, motionState.offsetX, motionState.offsetY);
      setUniform1f(gl, uniforms.u_lattice_density, currentUniforms.lattice_density);
      setUniform1f(gl, uniforms.u_circle_radius, currentUniforms.circle_radius);
      setUniform1f(gl, uniforms.u_circle_falloff_start, currentUniforms.circle_falloff_start);
      setUniform1f(gl, uniforms.u_circle_falloff_end, currentUniforms.circle_falloff_end);
      setUniform1f(gl, uniforms.u_bulge_amount, currentUniforms.bulge_amount);
      setUniform1f(gl, uniforms.u_rim_guard, currentUniforms.rim_guard);
      setUniform1f(gl, uniforms.u_rim_exponent, currentUniforms.rim_exponent);
      setUniform1f(gl, uniforms.u_rim_warp, currentUniforms.rim_warp);
      setUniform1f(gl, uniforms.u_dot_size, currentUniforms.dot_size);
      setUniform1f(gl, uniforms.u_outer_dot_scale, currentUniforms.outer_dot_scale);
      setUniform1f(gl, uniforms.u_edge_softness, currentUniforms.edge_softness);
      setUniform1f(gl, uniforms.u_chromatic_aberration, currentUniforms.chromatic_aberration);
      setUniform3f(gl, uniforms.u_cold_color, currentUniforms.cold_color);
      setUniform3f(gl, uniforms.u_hot_color, currentUniforms.hot_color);
      setUniform1f(gl, uniforms.u_inner_alpha, currentUniforms.inner_alpha);
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

function resolveMotionState(
  currentTime: number,
  currentRateX: number,
  currentRateY: number,
  sampleUniformsAtTime: (time: number) => { motion_rate: number; motion_rate_y: number },
  motionStateRef: { current: MotionState | null },
): MotionState {
  const safeTime = Math.max(0, currentTime);
  const previous = motionStateRef.current;
  if (!previous || safeTime < previous.time - 0.0001 || safeTime - previous.time > 0.25) {
    const recomputed = recomputeMotionState(safeTime, currentRateX, currentRateY, sampleUniformsAtTime);
    motionStateRef.current = recomputed;
    return recomputed;
  }
  const delta = safeTime - previous.time;
  if (delta <= 0.0001) {
    const stationary = { ...previous, rateX: currentRateX, rateY: currentRateY, time: safeTime };
    motionStateRef.current = stationary;
    return stationary;
  }
  const next = {
    offsetX: previous.offsetX + ((previous.rateX + currentRateX) * 0.5 * delta),
    offsetY: previous.offsetY + ((previous.rateY + currentRateY) * 0.5 * delta),
    rateX: currentRateX,
    rateY: currentRateY,
    time: safeTime,
  };
  motionStateRef.current = next;
  return next;
}

function recomputeMotionState(
  currentTime: number,
  currentRateX: number,
  currentRateY: number,
  sampleUniformsAtTime: (time: number) => { motion_rate: number; motion_rate_y: number },
): MotionState {
  const safeTime = Math.max(0, currentTime);
  if (safeTime <= 0.0001) {
    return {
      offsetX: 0,
      offsetY: 0,
      rateX: currentRateX,
      rateY: currentRateY,
      time: safeTime,
    };
  }

  const steps = Math.min(2048, Math.max(1, Math.ceil(safeTime * 120)));
  let offsetX = 0;
  let offsetY = 0;
  let previousTime = 0;
  let previousUniforms = sampleUniformsAtTime(0);

  for (let index = 1; index <= steps; index += 1) {
    const sampleTime = (safeTime * index) / steps;
    const uniforms =
      index === steps ? { motion_rate: currentRateX, motion_rate_y: currentRateY } : sampleUniformsAtTime(sampleTime);
    const delta = sampleTime - previousTime;
    offsetX += ((previousUniforms.motion_rate + uniforms.motion_rate) * 0.5 * delta);
    offsetY += ((previousUniforms.motion_rate_y + uniforms.motion_rate_y) * 0.5 * delta);
    previousTime = sampleTime;
    previousUniforms = uniforms;
  }

  return {
    offsetX,
    offsetY,
    rateX: currentRateX,
    rateY: currentRateY,
    time: safeTime,
  };
}
