use crate::archive::{TrackVisualizerConfig, TrackVisualizerMode, TrackVisualizerParams};
use ratzilla::{
    error::Error,
    ratatui::{
        layout::Rect,
        style::{Color, Style},
        Frame,
    },
    widgets::{GraphicsCanvas, GraphicsCanvasContext, GraphicsCanvasLayer},
};

const CYAN: Color = Color::Rgb(110, 220, 212);
const AMBER: Color = Color::Rgb(234, 182, 92);
const RED: Color = Color::Rgb(240, 104, 96);
const GREEN: Color = Color::Rgb(130, 208, 132);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AudioAnalysisSnapshot {
    pub energy: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub peak: f32,
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
    progress_ratio: f32,
    phase: f64,
    ring_count: usize,
    particle_count: usize,
    lattice_density: usize,
}

pub fn render_visualizer(
    frame: &mut Frame,
    area: Rect,
    layer: GraphicsCanvasLayer,
    config: &TrackVisualizerConfig,
    analysis: Option<AudioAnalysisSnapshot>,
    viewer_tick: u64,
) {
    let scene = resolve_scene_state(config, analysis, viewer_tick);
    let widget = GraphicsCanvas::new(
        layer,
        move |canvas: &GraphicsCanvasContext<'_>| match scene {
            VisualizerScene::DiplomaticSignalBloom(state) => {
                draw_diplomatic_signal_bloom(canvas, &state)
            }
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
        progress_ratio: progress_ratio.clamp(0.0, 1.0),
        is_playing: false,
    }
}

enum VisualizerScene {
    DiplomaticSignalBloom(DiplomaticSignalBloomState),
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
        TrackVisualizerMode::DiplomaticSignalBloom => {
            let scene = DiplomaticSignalBloomState {
                energy: (analysis.energy * params.energy_gain).clamp(0.0, 1.0),
                bass: (analysis.bass * params.bass_gain).clamp(0.0, 1.0),
                mid: (analysis.mid * params.mid_gain).clamp(0.0, 1.0),
                treble: (analysis.treble * params.treble_gain).clamp(0.0, 1.0),
                peak: analysis.peak.clamp(0.0, 1.0),
                progress_ratio: analysis.progress_ratio.clamp(0.0, 1.0),
                phase: viewer_tick as f64 / 1000.0 * params.motion_rate as f64,
                ring_count: params.ring_count,
                particle_count: params.particle_count,
                lattice_density: params.lattice_density,
            };
            VisualizerScene::DiplomaticSignalBloom(scene)
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

fn draw_diplomatic_signal_bloom(
    canvas: &GraphicsCanvasContext<'_>,
    state: &DiplomaticSignalBloomState,
) -> Result<(), Error> {
    let context = canvas.context();
    let width = canvas.width();
    let height = canvas.height();
    let energy_boost = state.energy as f64;
    let bass_pulse = state.bass as f64;
    let mid_warp = state.mid as f64;
    let treble_shimmer = state.treble as f64;
    let containment = state.progress_ratio as f64;
    let center_x = width * 0.5;
    let center_y = height * 0.5;
    let scale = (width.min(height) / 140.0).max(1.0);

    context.save();
    context.set_fill_style_str("rgb(4, 8, 8)");
    context.fill_rect(0.0, 0.0, width, height);
    draw_vertical_gradient(context, width, height)?;

    for ring in 0..state.ring_count {
        let radius = (14.0 + ring as f64 * 10.0 + bass_pulse * 8.0) * scale;
        let sweep = (state.phase * (0.8 + ring as f64 * 0.2)).sin() * 0.22;
        stroke_arc(
            context,
            center_x + (-58.0 + ring as f64 * 5.0) * scale,
            center_y,
            radius,
            -1.05 + sweep,
            1.05 + sweep,
            cyan_tone((0.45 + energy_boost * 0.4 + ring as f64 * 0.04) as f32),
            1.4 + energy_boost * 1.5,
        );
    }

    for index in 0..state.lattice_density {
        let offset = (-46.0 + index as f64 * (92.0 / state.lattice_density as f64)) * scale;
        let bend = mid_warp * 12.0 * (state.phase * 1.3 + index as f64 * 0.7).sin();
        stroke_line(
            context,
            center_x + offset,
            center_y + (-44.0 - bend) * scale,
            center_x + offset * -0.65,
            center_y + (44.0 + bend) * scale,
            amber_tone((0.35 + state.mid * 0.55) as f32),
            0.9 + state.mid as f64 * 1.4,
        );
        stroke_line(
            context,
            center_x + offset,
            center_y + (44.0 + bend) * scale,
            center_x + offset * -0.65,
            center_y + (-44.0 - bend) * scale,
            amber_tone((0.22 + state.mid * 0.48) as f32),
            0.6 + state.mid as f64 * 1.0,
        );
    }

    let particles = particle_points(state);
    for (x, y) in particles {
        fill_circle(
            context,
            center_x + x * scale,
            center_y + y * scale,
            0.8 + treble_shimmer * 1.6,
            green_tone((0.4 + state.treble * 0.55) as f32),
            0.28 + state.treble as f64 * 0.45,
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
        amber_tone((0.25 + state.energy * 0.3) as f32),
        0.18 + energy_boost * 0.28,
    );

    context.restore();
    Ok(())
}

fn particle_points(state: &DiplomaticSignalBloomState) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(state.particle_count);
    let spread = 18.0 + state.treble as f64 * 24.0;
    let progress_pull = state.progress_ratio as f64 * 12.0;
    for index in 0..state.particle_count {
        let t = index as f64 / state.particle_count as f64;
        let orbit = state.phase * (1.6 + t * 1.8) + t * 12.0;
        let x = 26.0 + t * 62.0 - progress_pull + orbit.sin() * spread * 0.35;
        let y = (t * 2.0 - 1.0) * 42.0 + orbit.cos() * spread * 0.22;
        points.push((x, y));
    }
    points
}

fn draw_vertical_gradient(
    context: &web_sys::CanvasRenderingContext2d,
    width: f64,
    height: f64,
) -> Result<(), Error> {
    let gradient = context.create_linear_gradient(0.0, 0.0, width, height);
    gradient
        .add_color_stop(0.0, "rgba(8, 14, 18, 0.95)")
        .map_err(Error::from)?;
    gradient
        .add_color_stop(0.55, "rgba(16, 8, 12, 0.88)")
        .map_err(Error::from)?;
    gradient
        .add_color_stop(1.0, "rgba(6, 6, 10, 0.98)")
        .map_err(Error::from)?;
    context.set_fill_style_canvas_gradient(&gradient);
    context.fill_rect(0.0, 0.0, width, height);
    Ok(())
}

fn stroke_arc(
    context: &web_sys::CanvasRenderingContext2d,
    x: f64,
    y: f64,
    radius: f64,
    start: f64,
    end: f64,
    color: Color,
    line_width: f64,
) {
    context.begin_path();
    context.set_stroke_style_str(&css_rgba(color, 0.82));
    context.set_line_width(line_width);
    let _ = context.arc(x, y, radius, start, end);
    context.stroke();
}

fn stroke_circle(
    context: &web_sys::CanvasRenderingContext2d,
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
    context: &web_sys::CanvasRenderingContext2d,
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

fn stroke_line(
    context: &web_sys::CanvasRenderingContext2d,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: Color,
    line_width: f64,
) {
    context.begin_path();
    context.set_stroke_style_str(&css_rgba(color, 0.58));
    context.set_line_width(line_width);
    context.move_to(x1, y1);
    context.line_to(x2, y2);
    context.stroke();
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

fn amber_tone(intensity: f32) -> Color {
    let intensity = intensity.clamp(0.0, 1.0);
    Color::Rgb(
        scale_channel(64, AMBER.r(), intensity),
        scale_channel(54, AMBER.g(), intensity),
        scale_channel(18, AMBER.b(), intensity),
    )
}

fn green_tone(intensity: f32) -> Color {
    let intensity = intensity.clamp(0.0, 1.0);
    Color::Rgb(
        scale_channel(24, GREEN.r(), intensity),
        scale_channel(72, GREEN.g(), intensity),
        scale_channel(34, GREEN.b(), intensity),
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
        clamp_params, decay_snapshot_toward_idle, resolve_scene_state, AudioAnalysisSnapshot,
        ClampedVisualizerParams, VisualizerScene,
    };
    use crate::archive::{TrackVisualizerConfig, TrackVisualizerMode, TrackVisualizerParams};

    fn config(params: TrackVisualizerParams) -> TrackVisualizerConfig {
        TrackVisualizerConfig {
            mode: TrackVisualizerMode::DiplomaticSignalBloom,
            params,
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
        let scene = resolve_scene_state(&config(TrackVisualizerParams::default()), None, 1_200);
        assert!(matches!(scene, VisualizerScene::DiplomaticSignalBloom(_)));
    }

    #[test]
    fn paused_state_decays_toward_idle_values() {
        let snapshot = AudioAnalysisSnapshot {
            energy: 1.0,
            bass: 0.8,
            mid: 0.6,
            treble: 0.4,
            peak: 0.9,
            progress_ratio: 0.7,
            is_playing: true,
        };

        let decayed = decay_snapshot_toward_idle(snapshot, 0.7);

        assert!(decayed.energy < snapshot.energy);
        assert!(decayed.bass < snapshot.bass);
        assert!(decayed.mid < snapshot.mid);
        assert!(decayed.treble < snapshot.treble);
        assert!(!decayed.is_playing);
        assert_eq!(decayed.progress_ratio, 0.7);
    }
}
