use std::{cell::RefCell, rc::Rc};

use crate::{
    archive::{PlaybackClock, TrackVisualizerConfig, TrackVisualizerMode, TrackVisualizerParams},
    panel_shader_visualizer::{PanelShaderRequest, PanelShaderVisualizerLayer},
};
use ratzilla::{
    error::Error,
    ratatui::{
        layout::Rect,
        style::{Color, Style},
        widgets::Paragraph,
        Frame,
    },
    widgets::{GraphicsCanvas, GraphicsCanvasContext, GraphicsCanvasLayer},
};
use web_sys::{wasm_bindgen::JsCast, window, CanvasRenderingContext2d, HtmlCanvasElement};

const CYAN: Color = Color::Rgb(110, 220, 212);
const RED: Color = Color::Rgb(240, 104, 96);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AudioAnalysisSnapshot {
    pub energy: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub peak: f32,
    pub beat: f32,
    pub progress_ratio: f32,
    pub is_playing: bool,
}

impl AudioAnalysisSnapshot {
    pub fn idle(progress_ratio: f32) -> Self {
        Self {
            progress_ratio: progress_ratio.clamp(0.0, 1.0),
            ..Self::default()
        }
    }
}

#[derive(Default)]
pub struct TrackVisualizerRuntime {
    active_key: Option<String>,
    hex_walker: Option<HexWalkerRelayRuntime>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClampedVisualizerParams {
    motion_rate: f32,
    energy_gain: f32,
    bass_gain: f32,
    mid_gain: f32,
    treble_gain: f32,
    ring_count: usize,
    particle_count: usize,
    lattice_density: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DiplomaticSignalBloomState {
    energy: f32,
    bass: f32,
    mid: f32,
    treble: f32,
    peak: f32,
    beat: f32,
    progress_ratio: f32,
    phase: f64,
    ring_count: usize,
    particle_count: usize,
    lattice_density: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HexWalkerRelayFrame {
    analysis: AudioAnalysisSnapshot,
    phase: f64,
    params: ClampedVisualizerParams,
}

struct HexWalkerRelayRuntime {
    trail_canvas: HtmlCanvasElement,
    trail_context: CanvasRenderingContext2d,
    fade_canvas: HtmlCanvasElement,
    fade_context: CanvasRenderingContext2d,
    walkers: Vec<Walker>,
    width: u32,
    height: u32,
    step_accumulator: f64,
}

#[derive(Clone, Copy, Debug)]
struct Walker {
    x: f64,
    y: f64,
    angle: f64,
}

pub fn render_visualizer(
    frame: &mut Frame,
    area: Rect,
    layer: GraphicsCanvasLayer,
    panel_shader_visualizer: PanelShaderVisualizerLayer,
    runtime: Rc<RefCell<TrackVisualizerRuntime>>,
    record_id: &str,
    config: &TrackVisualizerConfig,
    analysis: Option<AudioAnalysisSnapshot>,
    playback: PlaybackClock,
    viewer_tick: u64,
) {
    if is_shader_backed_mode(config.mode) {
        let resolved = config.resolve_chromatic_bulge_grid(playback);
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(Color::Black)),
            area,
        );
        panel_shader_visualizer.queue(PanelShaderRequest {
            area,
            mode: config.mode,
            record_id: record_id.to_string(),
            playback,
            shader_state: resolved.uniforms,
        });
        return;
    }

    let scene = resolve_scene_state(config, analysis, viewer_tick);
    let record_key = record_id.to_string();
    let config = config.clone();
    let widget = GraphicsCanvas::new(
        layer,
        move |canvas: &GraphicsCanvasContext<'_>| match scene {
            VisualizerScene::DiplomaticSignalBloom(state) => {
                draw_diplomatic_signal_bloom(canvas, &state)
            }
            VisualizerScene::HexWalkerRelay(frame_state) => runtime
                .borrow_mut()
                .draw_hex_walker_relay(canvas, &record_key, &config, &frame_state),
        },
    )
    .style(Style::default().bg(Color::Black))
    .smoothing(true);

    frame.render_widget(widget, area);
}

pub fn decay_snapshot_toward_idle(
    snapshot: AudioAnalysisSnapshot,
    progress_ratio: f32,
) -> AudioAnalysisSnapshot {
    let decay = |value: f32| (value * 0.92).clamp(0.0, 1.0);
    AudioAnalysisSnapshot {
        energy: decay(snapshot.energy),
        bass: decay(snapshot.bass),
        mid: decay(snapshot.mid),
        treble: decay(snapshot.treble),
        peak: decay(snapshot.peak),
        beat: (snapshot.beat * 0.78).clamp(0.0, 1.0),
        progress_ratio: progress_ratio.clamp(0.0, 1.0),
        is_playing: false,
    }
}

enum VisualizerScene {
    DiplomaticSignalBloom(DiplomaticSignalBloomState),
    HexWalkerRelay(HexWalkerRelayFrame),
}

fn is_shader_backed_mode(mode: TrackVisualizerMode) -> bool {
    matches!(mode, TrackVisualizerMode::ChromaticBulgeGrid)
}

impl TrackVisualizerRuntime {
    fn draw_hex_walker_relay(
        &mut self,
        canvas: &GraphicsCanvasContext<'_>,
        record_key: &str,
        config: &TrackVisualizerConfig,
        frame: &HexWalkerRelayFrame,
    ) -> Result<(), Error> {
        if self.active_key.as_deref() != Some(record_key) {
            self.active_key = Some(record_key.to_string());
            self.hex_walker = None;
        }

        let runtime = self
            .hex_walker
            .get_or_insert_with(|| HexWalkerRelayRuntime::new(config, canvas));
        runtime.ensure_canvas_size(config, canvas);
        if frame.analysis.is_playing {
            runtime.advance(frame)?;
        } else {
            runtime.clear();
        }

        let context = canvas.context();
        context.save();
        context.set_fill_style_str("rgb(0, 0, 0)");
        context.fill_rect(0.0, 0.0, canvas.width(), canvas.height());
        context.draw_image_with_html_canvas_element(&runtime.trail_canvas, 0.0, 0.0)?;
        context.restore();
        Ok(())
    }
}

impl HexWalkerRelayRuntime {
    fn new(config: &TrackVisualizerConfig, canvas: &GraphicsCanvasContext<'_>) -> Self {
        let (trail_canvas, trail_context) =
            create_detached_canvas().expect("detached trail canvas should initialize");
        let (fade_canvas, fade_context) =
            create_detached_canvas().expect("fade trail canvas should initialize");
        let mut runtime = Self {
            trail_canvas,
            trail_context,
            fade_canvas,
            fade_context,
            walkers: Vec::new(),
            width: 0,
            height: 0,
            step_accumulator: 0.0,
        };
        runtime.ensure_canvas_size(config, canvas);
        runtime
    }

    fn ensure_canvas_size(
        &mut self,
        config: &TrackVisualizerConfig,
        canvas: &GraphicsCanvasContext<'_>,
    ) {
        let width = canvas.width().max(1.0).round() as u32;
        let height = canvas.height().max(1.0).round() as u32;
        if self.width == width && self.height == height && !self.walkers.is_empty() {
            return;
        }

        self.width = width;
        self.height = height;
        self.trail_canvas.set_width(width);
        self.trail_canvas.set_height(height);
        self.fade_canvas.set_width(width);
        self.fade_canvas.set_height(height);
        self.trail_context.set_fill_style_str("rgb(0, 0, 0)");
        self.trail_context
            .fill_rect(0.0, 0.0, width as f64, height as f64);
        self.fade_context.set_fill_style_str("rgb(0, 0, 0)");
        self.fade_context
            .fill_rect(0.0, 0.0, width as f64, height as f64);
        self.walkers = seed_walkers(width as f64, height as f64, config.params.particle_count);
        self.step_accumulator = 0.0;
    }

    fn advance(&mut self, frame: &HexWalkerRelayFrame) -> Result<(), Error> {
        let width = self.width as f64;
        let height = self.height as f64;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let step_length = 12.0;
        let trail = &self.trail_context;

        let retain_alpha =
            (0.18 + frame.analysis.energy as f64 * 0.26 + frame.analysis.bass as f64 * 0.08)
                .clamp(0.18, 0.52);
        self.fade_context.save();
        let _ = self.fade_context.set_global_composite_operation("copy");
        self.fade_context
            .draw_image_with_html_canvas_element(&self.trail_canvas, 0.0, 0.0)?;
        self.fade_context.restore();

        trail.save();
        let _ = trail.set_global_composite_operation("source-over");
        trail.set_global_alpha(1.0);
        trail.set_fill_style_str("rgb(0, 0, 0)");
        trail.fill_rect(0.0, 0.0, width, height);
        trail.set_global_alpha(retain_alpha);
        trail.draw_image_with_html_canvas_element(&self.fade_canvas, 0.0, 0.0)?;
        trail.set_global_alpha(1.0);

        let _ = trail.set_global_composite_operation("lighter");
        trail.set_line_cap("round");
        trail.set_shadow_blur(4.0 + frame.analysis.energy as f64 * 5.0);
        trail.set_shadow_color(&css_rgba(RED, 0.14 + frame.analysis.treble as f64 * 0.12));

        let speed_scalar =
            ((0.12 + frame.analysis.energy as f64 * 0.9 + frame.analysis.bass as f64 * 0.8)
                * frame.params.motion_rate as f64)
                .clamp(0.06, 2.4);
        let turn_bias = (0.18 + frame.analysis.mid as f64 * 0.35).clamp(0.15, 0.65);
        let line_width = 0.4 + frame.analysis.energy as f64 * 0.55;
        let hue_base =
            2.0 + (frame.phase * 10.0).sin() * 6.0 + frame.analysis.progress_ratio as f64 * 10.0;

        self.step_accumulator += speed_scalar;
        let step_count = self.step_accumulator.floor().clamp(0.0, 4.0) as usize;
        self.step_accumulator = (self.step_accumulator - step_count as f64).clamp(0.0, 1.0);

        for step_index in 0..step_count {
            for (index, walker) in self.walkers.iter_mut().enumerate() {
                let previous_x = walker.x;
                let previous_y = walker.y;
                let branch_phase = frame.phase + index as f64 * 0.27 + step_index as f64 * 0.41;
                let choice = ((branch_phase.sin().abs() * 997.0) as usize + index + step_index) % 6;
                if (branch_phase.cos() * 0.5 + 0.5) < turn_bias {
                    walker.angle = choice as f64 * std::f64::consts::PI / 3.0;
                }
                let next_x = walker.x + walker.angle.cos() * step_length;
                let next_y = walker.y + walker.angle.sin() * step_length;

                if next_x < -24.0
                    || next_x > width + 24.0
                    || next_y < -24.0
                    || next_y > height + 24.0
                {
                    walker.x = center_x;
                    walker.y = center_y;
                    walker.angle = choice as f64 * std::f64::consts::PI / 3.0;
                    continue;
                }

                walker.x = next_x;
                walker.y = next_y;

                let hue = hue_base
                    + choice as f64 * 8.0
                    + frame.analysis.treble as f64 * 48.0
                    + branch_phase.sin() * 18.0;
                let alpha = (0.05
                    + frame.analysis.energy as f64 * 0.08
                    + frame.analysis.peak as f64 * 0.05)
                    .clamp(0.04, 0.15);
                trail.begin_path();
                trail.set_stroke_style_str(&hsla(
                    hue,
                    92.0,
                    50.0 + frame.analysis.mid as f64 * 6.0,
                    alpha,
                ));
                trail.set_line_width(line_width);
                trail.move_to(previous_x, previous_y);
                trail.line_to(next_x, next_y);
                trail.stroke();
            }
        }

        trail.restore();
        Ok(())
    }

    fn clear(&mut self) {
        self.trail_context.save();
        self.trail_context.set_fill_style_str("rgb(0, 0, 0)");
        self.trail_context
            .fill_rect(0.0, 0.0, self.width as f64, self.height as f64);
        self.trail_context.restore();
    }
}

fn resolve_scene_state(
    config: &TrackVisualizerConfig,
    analysis: Option<AudioAnalysisSnapshot>,
    viewer_tick: u64,
) -> VisualizerScene {
    let params = clamp_params(&config.params);
    let idle =
        AudioAnalysisSnapshot::idle(analysis.map_or(0.0, |snapshot| snapshot.progress_ratio));
    let analysis = analysis.unwrap_or(idle);

    match config.mode {
        TrackVisualizerMode::DiplomaticSignalBloom | TrackVisualizerMode::ContainmentLattice => {
            let scene = DiplomaticSignalBloomState {
                energy: (analysis.energy * params.energy_gain).clamp(0.0, 1.0),
                bass: (analysis.bass * params.bass_gain).clamp(0.0, 1.0),
                mid: (analysis.mid * params.mid_gain).clamp(0.0, 1.0),
                treble: (analysis.treble * params.treble_gain).clamp(0.0, 1.0),
                peak: analysis.peak.clamp(0.0, 1.0),
                beat: analysis.beat.clamp(0.0, 1.0),
                progress_ratio: analysis.progress_ratio.clamp(0.0, 1.0),
                phase: viewer_tick as f64 / 1000.0 * params.motion_rate as f64,
                ring_count: params.ring_count,
                particle_count: params.particle_count,
                lattice_density: params.lattice_density,
            };
            VisualizerScene::DiplomaticSignalBloom(scene)
        }
        TrackVisualizerMode::HexWalkerRelay => {
            VisualizerScene::HexWalkerRelay(HexWalkerRelayFrame {
                analysis: AudioAnalysisSnapshot {
                    energy: (analysis.energy * params.energy_gain).clamp(0.0, 1.0),
                    bass: (analysis.bass * params.bass_gain).clamp(0.0, 1.0),
                    mid: (analysis.mid * params.mid_gain).clamp(0.0, 1.0),
                    treble: (analysis.treble * params.treble_gain).clamp(0.0, 1.0),
                    peak: analysis.peak.clamp(0.0, 1.0),
                    beat: analysis.beat.clamp(0.0, 1.0),
                    progress_ratio: analysis.progress_ratio.clamp(0.0, 1.0),
                    is_playing: analysis.is_playing,
                },
                phase: viewer_tick as f64 / 1000.0 * params.motion_rate as f64,
                params,
            })
        }
        TrackVisualizerMode::ChromaticBulgeGrid => {
            unreachable!("shader-backed visualizer modes should not resolve to canvas scenes")
        }
    }
}

fn clamp_params(params: &TrackVisualizerParams) -> ClampedVisualizerParams {
    ClampedVisualizerParams {
        motion_rate: params.motion_rate.clamp(0.2, 3.0),
        energy_gain: params.energy_gain.clamp(0.2, 2.5),
        bass_gain: params.bass_gain.clamp(0.2, 2.5),
        mid_gain: params.mid_gain.clamp(0.2, 2.5),
        treble_gain: params.treble_gain.clamp(0.2, 2.5),
        ring_count: params.ring_count.clamp(1, 8) as usize,
        particle_count: params.particle_count.clamp(8, 96) as usize,
        lattice_density: params.lattice_density.clamp(2, 12) as usize,
    }
}

fn seed_walkers(width: f64, height: f64, count: u16) -> Vec<Walker> {
    let count = count.clamp(4, 96) as usize;
    let center_x = width * 0.5;
    let center_y = height * 0.5;
    (0..count)
        .map(|index| Walker {
            x: center_x,
            y: center_y,
            angle: (index % 6) as f64 * std::f64::consts::PI / 3.0,
        })
        .collect()
}

fn create_detached_canvas() -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), Error> {
    let document = window()
        .ok_or_else(|| Error::UnableToRetrieveWindow)?
        .document()
        .ok_or_else(|| Error::UnableToRetrieveDocument)?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| Error::UnableToRetrieveElementById("canvas".to_string()))?;
    let context = canvas
        .get_context("2d")?
        .ok_or(Error::UnableToRetrieveCanvasContext)?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| Error::UnableToRetrieveCanvasContext)?;
    Ok((canvas, context))
}

fn draw_diplomatic_signal_bloom(
    canvas: &GraphicsCanvasContext<'_>,
    state: &DiplomaticSignalBloomState,
) -> Result<(), Error> {
    let context = canvas.context();
    let width = canvas.width();
    let height = canvas.height();
    let energy_boost = state.energy as f64;
    let bass_pulse = state.bass as f64;
    let treble_shimmer = state.treble as f64;
    let containment = state.progress_ratio as f64;
    let center_x = width * 0.5;
    let center_y = height * 0.5;
    let scale = (width.min(height) / 140.0).max(1.0);

    context.save();
    context.set_fill_style_str("rgb(0, 0, 0)");
    context.fill_rect(0.0, 0.0, width, height);

    for ring in 0..state.ring_count {
        let radius = (14.0 + ring as f64 * 10.0 + bass_pulse * 8.0) * scale;
        stroke_circle(
            context,
            center_x,
            center_y,
            radius,
            cyan_tone((0.45 + energy_boost * 0.4 + ring as f64 * 0.04) as f32),
            1.4 + energy_boost * 1.5,
            0.82,
        );
    }

    let ring_radius = (22.0 + containment * 34.0 + bass_pulse * 8.0) * scale;
    stroke_circle(
        context,
        center_x,
        center_y,
        ring_radius,
        red_tone((0.25 + containment * 0.65 + state.peak as f64 * 0.1) as f32),
        1.4 + containment * 3.2,
        0.35 + containment * 0.45,
    );

    fill_circle(
        context,
        center_x,
        center_y,
        (10.0 + energy_boost * 10.0 + treble_shimmer * 6.0) * scale,
        cyan_tone((0.25 + state.energy * 0.3) as f32),
        0.18 + energy_boost * 0.28,
    );

    context.restore();
    Ok(())
}

fn stroke_circle(
    context: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    color: Color,
    line_width: f64,
    alpha: f64,
) {
    context.begin_path();
    context.set_stroke_style_str(&css_rgba(color, alpha));
    context.set_line_width(line_width);
    let _ = context.arc(x, y, radius, 0.0, std::f64::consts::TAU);
    context.stroke();
}

fn fill_circle(
    context: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    color: Color,
    alpha: f64,
) {
    context.begin_path();
    context.set_fill_style_str(&css_rgba(color, alpha));
    let _ = context.arc(x, y, radius, 0.0, std::f64::consts::TAU);
    context.fill();
}

fn hsla(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> String {
    format!(
        "hsla({:.1}, {:.1}%, {:.1}%, {:.3})",
        hue.rem_euclid(360.0),
        saturation.clamp(0.0, 100.0),
        lightness.clamp(0.0, 100.0),
        alpha.clamp(0.0, 1.0)
    )
}

fn css_rgba(color: Color, alpha: f64) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("rgba({r}, {g}, {b}, {:.3})", alpha.clamp(0.0, 1.0)),
        _ => format!("rgba(255, 255, 255, {:.3})", alpha.clamp(0.0, 1.0)),
    }
}

fn cyan_tone(intensity: f32) -> Color {
    let intensity = intensity.clamp(0.0, 1.0);
    Color::Rgb(
        scale_channel(48, CYAN.r(), intensity),
        scale_channel(96, CYAN.g(), intensity),
        scale_channel(90, CYAN.b(), intensity),
    )
}

fn red_tone(intensity: f32) -> Color {
    let intensity = intensity.clamp(0.0, 1.0);
    Color::Rgb(
        scale_channel(90, RED.r(), intensity),
        scale_channel(16, RED.g(), intensity),
        scale_channel(16, RED.b(), intensity),
    )
}

fn scale_channel(base: u8, target: u8, intensity: f32) -> u8 {
    let base = base as f32;
    let target = target as f32;
    (base + (target - base) * intensity)
        .round()
        .clamp(0.0, 255.0) as u8
}

trait RgbChannelExt {
    fn r(self) -> u8;
    fn g(self) -> u8;
    fn b(self) -> u8;
}

impl RgbChannelExt for Color {
    fn r(self) -> u8 {
        match self {
            Color::Rgb(r, _, _) => r,
            _ => 0,
        }
    }

    fn g(self) -> u8 {
        match self {
            Color::Rgb(_, g, _) => g,
            _ => 0,
        }
    }

    fn b(self) -> u8 {
        match self {
            Color::Rgb(_, _, b) => b,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_params, decay_snapshot_toward_idle, is_shader_backed_mode, resolve_scene_state,
        AudioAnalysisSnapshot, ClampedVisualizerParams, VisualizerScene,
    };
    use crate::archive::{TrackVisualizerConfig, TrackVisualizerMode, TrackVisualizerParams};

    fn config(mode: TrackVisualizerMode, params: TrackVisualizerParams) -> TrackVisualizerConfig {
        TrackVisualizerConfig {
            mode,
            params,
            lfo_library: None,
            automation: None,
            timeline: None,
        }
    }

    #[test]
    fn parameter_clamping_keeps_values_in_expected_range() {
        let clamped = clamp_params(&TrackVisualizerParams {
            motion_rate: 10.0,
            energy_gain: 8.0,
            bass_gain: 0.0,
            mid_gain: -1.0,
            treble_gain: 5.0,
            ring_count: 99,
            particle_count: 0,
            lattice_density: 32,
            shader_states: None,
        });

        assert_eq!(
            clamped,
            ClampedVisualizerParams {
                motion_rate: 3.0,
                energy_gain: 2.5,
                bass_gain: 0.2,
                mid_gain: 0.2,
                treble_gain: 2.5,
                ring_count: 8,
                particle_count: 8,
                lattice_density: 12,
            }
        );
    }

    #[test]
    fn scene_selection_uses_diplomatic_signal_bloom() {
        let scene = resolve_scene_state(
            &config(
                TrackVisualizerMode::DiplomaticSignalBloom,
                TrackVisualizerParams::default(),
            ),
            None,
            1_200,
        );
        assert!(matches!(scene, VisualizerScene::DiplomaticSignalBloom(_)));
    }

    #[test]
    fn scene_selection_supports_hex_walker_relay() {
        let scene = resolve_scene_state(
            &config(
                TrackVisualizerMode::HexWalkerRelay,
                TrackVisualizerParams::default(),
            ),
            None,
            1_200,
        );
        assert!(matches!(scene, VisualizerScene::HexWalkerRelay(_)));
    }

    #[test]
    fn chromatic_bulge_grid_is_shader_backed() {
        assert!(is_shader_backed_mode(
            TrackVisualizerMode::ChromaticBulgeGrid
        ));
    }

    #[test]
    fn existing_modes_remain_canvas_backed() {
        assert!(!is_shader_backed_mode(
            TrackVisualizerMode::DiplomaticSignalBloom
        ));
        assert!(!is_shader_backed_mode(
            TrackVisualizerMode::ContainmentLattice
        ));
        assert!(!is_shader_backed_mode(TrackVisualizerMode::HexWalkerRelay));
    }

    #[test]
    fn paused_state_decays_toward_idle_values() {
        let snapshot = AudioAnalysisSnapshot {
            energy: 1.0,
            bass: 0.8,
            mid: 0.6,
            treble: 0.4,
            peak: 0.9,
            beat: 1.0,
            progress_ratio: 0.7,
            is_playing: true,
        };

        let decayed = decay_snapshot_toward_idle(snapshot, 0.7);

        assert!(decayed.energy < snapshot.energy);
        assert!(decayed.bass < snapshot.bass);
        assert!(decayed.mid < snapshot.mid);
        assert!(decayed.treble < snapshot.treble);
        assert!(decayed.beat < snapshot.beat);
        assert!(!decayed.is_playing);
        assert_eq!(decayed.progress_ratio, 0.7);
    }
}
