use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CHROMATIC_BULGE_GRID_VERTEX_SHADER: &str = r#"#version 300 es
precision highp float;

out vec2 v_uv;

const vec2 pos[3] = vec2[](
  vec2(-1.0, -1.0),
  vec2( 3.0, -1.0),
  vec2(-1.0,  3.0)
);

void main() {
  vec2 p = pos[gl_VertexID];
  v_uv = 0.5 * (p + 1.0);
  gl_Position = vec4(p, 0.0, 1.0);
}
"#;

pub const CHROMATIC_BULGE_GRID_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;

uniform vec2 u_resolution;
uniform float u_time;
uniform vec2 u_motion_rate;
uniform float u_lattice_density;
uniform float u_circle_radius;
uniform float u_circle_falloff_start;
uniform float u_circle_falloff_end;
uniform float u_bulge_amount;
uniform float u_rim_guard;
uniform float u_rim_exponent;
uniform float u_rim_warp;
uniform float u_dot_size;
uniform float u_outer_dot_scale;
uniform float u_edge_softness;
uniform float u_chromatic_aberration;
uniform vec3 u_cold_color;
uniform vec3 u_hot_color;
uniform float u_inner_alpha;

out vec4 out_color;

const float GRID_SPACING_MAX_PX = 22.0;
const float GRID_SPACING_MIN_PX = 12.0;

float dot_mask(vec2 sample_px, float spacing_px, float radius_px, float edge_px) {
  vec2 local = mod(sample_px + 0.5 * spacing_px, spacing_px) - 0.5 * spacing_px;
  float distance_to_center = length(local);
  return 1.0 - smoothstep(radius_px, radius_px + edge_px, distance_to_center);
}

void main() {
  vec2 frag_px = v_uv * u_resolution;
  vec2 center = 0.5 * u_resolution;
  vec2 lens_delta = frag_px - center;
  vec2 radial_axis = length(lens_delta) > 0.0001 ? normalize(lens_delta) : vec2(1.0, 0.0);

  float lens_radius = u_circle_radius * min(u_resolution.x, u_resolution.y);
  float lens_distance = length(lens_delta);
  float normalized_radius = lens_distance / max(lens_radius, 1.0);
  float falloff = 1.0 - smoothstep(u_circle_falloff_start, u_circle_falloff_end, normalized_radius);
  float hemisphere = sqrt(max(0.0, 1.0 - normalized_radius * normalized_radius));
  vec2 sphere_xy = lens_radius > 0.0 ? lens_delta / lens_radius : vec2(0.0);
  vec3 sphere_normal = normalize(vec3(sphere_xy, max(hemisphere, 0.001)));

  float density = clamp((u_lattice_density - 2.0) / 10.0, 0.0, 1.0);
  float spacing_px = mix(GRID_SPACING_MAX_PX, GRID_SPACING_MIN_PX, density);
  float base_radius_px = spacing_px * u_dot_size;
  vec2 motion_px = u_time * u_motion_rate * 60.0;
  vec2 base_sample_px = frag_px;
  base_sample_px += motion_px;

  vec2 sphere_offset = frag_px - center;
  float center_profile = falloff * hemisphere;
  float magnify = 1.0 - center_profile * u_bulge_amount;
  vec2 warped_screen_px = center + sphere_offset * magnify;
  vec2 rim_direction = sphere_normal.xy / max(sphere_normal.z, u_rim_guard);
  float rim_profile = falloff * pow(clamp(1.0 - sphere_normal.z, 0.0, 1.0), u_rim_exponent);
  float rim_warp = rim_profile * u_rim_warp;
  vec2 sphere_sample_px = warped_screen_px + rim_direction * rim_warp;
  sphere_sample_px += motion_px;
  float sphere_mix = clamp(falloff * hemisphere, 0.0, 1.0);
  float dot_radius_px = mix(base_radius_px * u_outer_dot_scale, base_radius_px, sphere_mix);
  float edge_px = u_edge_softness;
  vec2 final_sample_px = mix(base_sample_px, sphere_sample_px, sphere_mix);

  float chroma_drive = sphere_mix * u_chromatic_aberration;
  vec2 chroma_offset = radial_axis * chroma_drive;

  float mask_g = dot_mask(final_sample_px, spacing_px, dot_radius_px, edge_px);
  float mask_r = dot_mask(
    final_sample_px + chroma_offset,
    spacing_px,
    dot_radius_px,
    edge_px
  );
  float mask_b = dot_mask(
    final_sample_px - chroma_offset,
    spacing_px,
    dot_radius_px,
    edge_px
  );

  vec3 dot_color = mix(u_cold_color, u_hot_color, sphere_mix);

  float outer_alpha = mask_g;
  float inner_alpha = mask_g * u_inner_alpha;
  float alpha = mix(outer_alpha, inner_alpha, sphere_mix);
  vec3 color = vec3(mask_r, mask_g, mask_b) * dot_color;

  out_color = vec4(clamp(color, 0.0, 1.0), clamp(alpha, 0.0, 1.0));
}
"#;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TrackVisualizerConfig {
    pub mode: TrackVisualizerMode,
    #[serde(default)]
    pub params: TrackVisualizerParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<ChromaticBulgeGridAutomation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<ChromaticBulgeGridClipTimeline>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackVisualizerMode {
    DiplomaticSignalBloom,
    HexWalkerRelay,
    ContainmentLattice,
    ChromaticBulgeGrid,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TrackVisualizerParams {
    #[serde(default = "default_motion_rate")]
    pub motion_rate: f32,
    #[serde(default = "default_energy_gain")]
    pub energy_gain: f32,
    #[serde(default = "default_bass_gain")]
    pub bass_gain: f32,
    #[serde(default = "default_mid_gain")]
    pub mid_gain: f32,
    #[serde(default = "default_treble_gain")]
    pub treble_gain: f32,
    #[serde(default = "default_ring_count")]
    pub ring_count: u16,
    #[serde(default = "default_particle_count")]
    pub particle_count: u16,
    #[serde(default = "default_lattice_density")]
    pub lattice_density: u16,
    #[serde(default)]
    pub shader_states: Option<ChromaticBulgeGridShaderStates>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridShaderStates {
    #[serde(default)]
    pub playing: ChromaticBulgeGridShaderState,
    #[serde(default, alias = "not_playing")]
    pub idle: ChromaticBulgeGridShaderState,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridShaderState {
    #[serde(default = "default_motion_rate")]
    pub motion_rate: f32,
    #[serde(default = "default_motion_rate_y")]
    pub motion_rate_y: f32,
    #[serde(default = "default_shader_lattice_density")]
    pub lattice_density: f32,
    #[serde(default = "default_circle_radius")]
    pub circle_radius: f32,
    #[serde(default = "default_circle_falloff_start")]
    pub circle_falloff_start: f32,
    #[serde(default = "default_circle_falloff_end")]
    pub circle_falloff_end: f32,
    #[serde(default = "default_bulge_amount")]
    pub bulge_amount: f32,
    #[serde(default = "default_rim_guard")]
    pub rim_guard: f32,
    #[serde(default = "default_rim_exponent")]
    pub rim_exponent: f32,
    #[serde(default = "default_rim_warp")]
    pub rim_warp: f32,
    #[serde(default = "default_dot_size")]
    pub dot_size: f32,
    #[serde(default = "default_outer_dot_scale")]
    pub outer_dot_scale: f32,
    #[serde(default = "default_edge_softness")]
    pub edge_softness: f32,
    #[serde(default = "default_chromatic_aberration")]
    pub chromatic_aberration: f32,
    #[serde(default = "default_cold_color")]
    pub cold_color: [f32; 3],
    #[serde(default = "default_hot_color")]
    pub hot_color: [f32; 3],
    #[serde(default = "default_color_cycle_rate")]
    pub color_cycle_rate: f32,
    #[serde(default = "default_inner_alpha")]
    pub inner_alpha: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridAutomation {
    pub bpm: f32,
    pub measures: u32,
    #[serde(default = "default_beats_per_measure")]
    pub beats_per_measure: u32,
    #[serde(default)]
    pub lanes: ChromaticBulgeGridAutomationLanes,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridClipTimeline {
    pub bpm: f32,
    pub measures: u32,
    #[serde(default = "default_beats_per_measure")]
    pub beats_per_measure: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<ChromaticBulgeGridClip>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrangement: Vec<ClipPlacement>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridClip {
    pub id: String,
    pub name: String,
    pub length_beats: f32,
    #[serde(default = "default_clip_color")]
    pub color: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ChromaticBulgeGridClipSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authoring: Option<ChromaticBulgeGridClipAuthoring>,
    #[serde(default)]
    pub lanes: ChromaticBulgeGridAutomationLanes,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClipPlacement {
    pub clip_id: String,
    pub start_beat: f32,
    #[serde(default)]
    pub track: u8,
    #[serde(default = "default_repeat_count")]
    pub repeats: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridClipAuthoring {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<ClipParamTrack>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridLfoShape {
    #[serde(default)]
    pub interpolation: LfoInterpolation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<LfoPoint>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct LfoPoint {
    pub phase: f32,
    pub value: f32,
    #[serde(default, skip_serializing_if = "lfo_curve_is_zero")]
    pub curve_to_next: f32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LfoInterpolation {
    #[default]
    Linear,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChromaticBulgeGridClipSource {
    Lfo(ChromaticBulgeGridLfoClip),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridLfoClip {
    pub lane: ChromaticBulgeGridLaneId,
    pub shape: ChromaticBulgeGridLfoShape,
    pub min: f32,
    pub max: f32,
    pub period_beats: f32,
    #[serde(default)]
    pub phase_offset_beats: f32,
    #[serde(default)]
    pub start_mode: LfoStartMode,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LfoStartMode {
    #[default]
    Retrigger,
    Continue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClipParamTrack {
    pub lane: ChromaticBulgeGridLaneId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<ClipTweenStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClipTweenStep {
    pub to: ClipTweenValue,
    pub duration_beats: f32,
    #[serde(default)]
    pub ease: ClipTweenEase,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ClipTweenValue {
    Float(f32),
    Color([f32; 3]),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipTweenEase {
    Hold,
    #[default]
    Linear,
    SineIn,
    SineOut,
    SineInOut,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ChromaticBulgeGridLaneId {
    MotionRate,
    MotionRateY,
    LatticeDensity,
    CircleRadius,
    CircleFalloffStart,
    CircleFalloffEnd,
    BulgeAmount,
    RimGuard,
    RimExponent,
    RimWarp,
    DotSize,
    OuterDotScale,
    EdgeSoftness,
    ChromaticAberration,
    ColdColor,
    HotColor,
    ColorCycleRate,
    InnerAlpha,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridAutomationLanes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub motion_rate: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub motion_rate_y: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lattice_density: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circle_radius: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circle_falloff_start: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circle_falloff_end: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bulge_amount: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rim_guard: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rim_exponent: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rim_warp: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dot_size: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_dot_scale: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_softness: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chromatic_aberration: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cold_color: Vec<ColorKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hot_color: Vec<ColorKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub color_cycle_rate: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_alpha: Vec<FloatKeyframe>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct FloatKeyframe {
    pub beat: f32,
    pub value: f32,
    #[serde(default)]
    pub interpolation: InterpolationMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ColorKeyframe {
    pub beat: f32,
    pub value: [f32; 3],
    #[serde(default)]
    pub interpolation: InterpolationMode,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMode {
    #[default]
    Hold,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaybackClock {
    pub current_time_secs: f32,
    pub visual_time_secs: f32,
    pub duration_secs: Option<f32>,
    pub is_playing: bool,
    pub timeline_preview: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChromaticBulgeGridResolvedState {
    pub uniforms: ChromaticBulgeGridShaderState,
    pub current_beat: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChromaticBulgeGridUniforms {
    pub resolution: [f32; 2],
    pub time: f32,
    pub motion_rate: f32,
    pub motion_rate_y: f32,
    pub lattice_density: f32,
    pub circle_radius: f32,
    pub circle_falloff_start: f32,
    pub circle_falloff_end: f32,
    pub bulge_amount: f32,
    pub rim_guard: f32,
    pub rim_exponent: f32,
    pub rim_warp: f32,
    pub dot_size: f32,
    pub outer_dot_scale: f32,
    pub edge_softness: f32,
    pub chromatic_aberration: f32,
    pub cold_color: [f32; 3],
    pub hot_color: [f32; 3],
    pub color_cycle_rate: f32,
    pub inner_alpha: f32,
}

#[derive(Debug, Error)]
pub enum VfxConfigError {
    #[error("{0}")]
    Validation(String),
    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),
}

impl Default for TrackVisualizerParams {
    fn default() -> Self {
        Self {
            motion_rate: default_motion_rate(),
            energy_gain: default_energy_gain(),
            bass_gain: default_bass_gain(),
            mid_gain: default_mid_gain(),
            treble_gain: default_treble_gain(),
            ring_count: default_ring_count(),
            particle_count: default_particle_count(),
            lattice_density: default_lattice_density(),
            shader_states: None,
        }
    }
}

impl Default for ChromaticBulgeGridShaderStates {
    fn default() -> Self {
        Self {
            playing: ChromaticBulgeGridShaderState::default(),
            idle: ChromaticBulgeGridShaderState::default(),
        }
    }
}

impl Default for ChromaticBulgeGridShaderState {
    fn default() -> Self {
        Self {
            motion_rate: default_motion_rate(),
            motion_rate_y: default_motion_rate_y(),
            lattice_density: default_shader_lattice_density(),
            circle_radius: default_circle_radius(),
            circle_falloff_start: default_circle_falloff_start(),
            circle_falloff_end: default_circle_falloff_end(),
            bulge_amount: default_bulge_amount(),
            rim_guard: default_rim_guard(),
            rim_exponent: default_rim_exponent(),
            rim_warp: default_rim_warp(),
            dot_size: default_dot_size(),
            outer_dot_scale: default_outer_dot_scale(),
            edge_softness: default_edge_softness(),
            chromatic_aberration: default_chromatic_aberration(),
            cold_color: default_cold_color(),
            hot_color: default_hot_color(),
            color_cycle_rate: default_color_cycle_rate(),
            inner_alpha: default_inner_alpha(),
        }
    }
}

impl Default for PlaybackClock {
    fn default() -> Self {
        Self {
            current_time_secs: 0.0,
            visual_time_secs: 0.0,
            duration_secs: None,
            is_playing: false,
            timeline_preview: false,
        }
    }
}

impl TrackVisualizerMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::DiplomaticSignalBloom => "diplomatic signal bloom",
            Self::HexWalkerRelay => "hex walker relay",
            Self::ContainmentLattice => "containment lattice",
            Self::ChromaticBulgeGrid => "chromatic bulge grid",
        }
    }
}

impl ChromaticBulgeGridLaneId {
    pub const ALL: [Self; 18] = [
        Self::MotionRate,
        Self::MotionRateY,
        Self::LatticeDensity,
        Self::CircleRadius,
        Self::CircleFalloffStart,
        Self::CircleFalloffEnd,
        Self::BulgeAmount,
        Self::RimGuard,
        Self::RimExponent,
        Self::RimWarp,
        Self::DotSize,
        Self::OuterDotScale,
        Self::EdgeSoftness,
        Self::ChromaticAberration,
        Self::ColdColor,
        Self::HotColor,
        Self::ColorCycleRate,
        Self::InnerAlpha,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MotionRate => "motion_rate",
            Self::MotionRateY => "motion_rate_y",
            Self::LatticeDensity => "lattice_density",
            Self::CircleRadius => "circle_radius",
            Self::CircleFalloffStart => "circle_falloff_start",
            Self::CircleFalloffEnd => "circle_falloff_end",
            Self::BulgeAmount => "bulge_amount",
            Self::RimGuard => "rim_guard",
            Self::RimExponent => "rim_exponent",
            Self::RimWarp => "rim_warp",
            Self::DotSize => "dot_size",
            Self::OuterDotScale => "outer_dot_scale",
            Self::EdgeSoftness => "edge_softness",
            Self::ChromaticAberration => "chromatic_aberration",
            Self::ColdColor => "cold_color",
            Self::HotColor => "hot_color",
            Self::ColorCycleRate => "color_cycle_rate",
            Self::InnerAlpha => "inner_alpha",
        }
    }

    pub fn is_color(self) -> bool {
        matches!(self, Self::ColdColor | Self::HotColor)
    }

    pub fn supports_lfo(self) -> bool {
        !self.is_color()
    }

    pub fn from_track_index(track: u8) -> Option<Self> {
        Self::ALL.get(track as usize).copied()
    }

    pub fn track_index(self) -> u8 {
        Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0) as u8
    }
}

impl ChromaticBulgeGridClip {
    pub fn primary_lane(&self) -> Option<ChromaticBulgeGridLaneId> {
        if let Some(ChromaticBulgeGridClipSource::Lfo(lfo)) = &self.source {
            return Some(lfo.lane);
        }
        if let Some(authoring) = &self.authoring {
            if let Some(track) = authoring.tracks.first() {
                return Some(track.lane);
            }
        }
        let lanes = self.lanes.non_empty_lane_ids();
        if lanes.len() == 1 {
            lanes.into_iter().next()
        } else {
            None
        }
    }

    pub fn apply_to_state(
        &self,
        base: ChromaticBulgeGridShaderState,
        local_beat: f32,
        global_beat: f32,
    ) -> ChromaticBulgeGridShaderState {
        if let Some(ChromaticBulgeGridClipSource::Lfo(lfo)) = &self.source {
            return apply_lfo_clip_to_state(
                base,
                lfo,
                local_beat.clamp(0.0, self.length_beats.max(0.0)),
                global_beat,
            );
        }
        apply_lanes_to_state(
            base,
            &self.lanes,
            local_beat.clamp(0.0, self.length_beats.max(0.0)),
        )
    }

    pub fn authored_lanes(&self) -> HashSet<ChromaticBulgeGridLaneId> {
        if let Some(ChromaticBulgeGridClipSource::Lfo(lfo)) = &self.source {
            return HashSet::from([lfo.lane]);
        }
        if let Some(authoring) = &self.authoring {
            return authoring
                .tracks
                .iter()
                .filter(|track| !track.steps.is_empty())
                .map(|track| track.lane)
                .collect();
        }
        self.lanes.non_empty_lane_ids().into_iter().collect()
    }
}

impl ClipPlacement {
    pub fn end_beat(&self, clip: &ChromaticBulgeGridClip) -> f32 {
        self.start_beat + clip.length_beats * self.repeats.max(1) as f32
    }
}

impl ChromaticBulgeGridShaderState {
    pub fn from_legacy_params(params: &TrackVisualizerParams) -> Self {
        Self {
            motion_rate: params.motion_rate,
            motion_rate_y: default_motion_rate_y(),
            lattice_density: params.lattice_density as f32,
            ..Self::default()
        }
    }

    pub fn clamp(self) -> Self {
        Self {
            motion_rate: self.motion_rate.clamp(-4.0, 4.0),
            motion_rate_y: self.motion_rate_y.clamp(-4.0, 4.0),
            lattice_density: self.lattice_density.clamp(2.0, 12.0),
            circle_radius: self.circle_radius.clamp(0.02, 0.95),
            circle_falloff_start: self.circle_falloff_start.clamp(0.0, 1.0),
            circle_falloff_end: self.circle_falloff_end.clamp(0.0, 1.2),
            bulge_amount: self.bulge_amount.clamp(0.0, 1.5),
            rim_guard: self.rim_guard.clamp(0.01, 2.0),
            rim_exponent: self.rim_exponent.clamp(0.1, 6.0),
            rim_warp: self.rim_warp.clamp(0.0, 64.0),
            dot_size: self.dot_size.clamp(0.02, 1.0),
            outer_dot_scale: self.outer_dot_scale.clamp(0.02, 2.0),
            edge_softness: self.edge_softness.clamp(0.1, 8.0),
            chromatic_aberration: self.chromatic_aberration.clamp(0.0, 24.0),
            cold_color: clamp_color(self.cold_color),
            hot_color: clamp_color(self.hot_color),
            color_cycle_rate: self.color_cycle_rate.clamp(0.0, 6.0),
            inner_alpha: self.inner_alpha.clamp(0.0, 1.0),
        }
    }

    pub fn to_uniforms(self, resolution: [f32; 2], time: f32) -> ChromaticBulgeGridUniforms {
        ChromaticBulgeGridUniforms {
            resolution,
            time,
            motion_rate: self.motion_rate,
            motion_rate_y: self.motion_rate_y,
            lattice_density: self.lattice_density,
            circle_radius: self.circle_radius,
            circle_falloff_start: self.circle_falloff_start,
            circle_falloff_end: self.circle_falloff_end,
            bulge_amount: self.bulge_amount,
            rim_guard: self.rim_guard,
            rim_exponent: self.rim_exponent,
            rim_warp: self.rim_warp,
            dot_size: self.dot_size,
            outer_dot_scale: self.outer_dot_scale,
            edge_softness: self.edge_softness,
            chromatic_aberration: self.chromatic_aberration,
            cold_color: self.cold_color,
            hot_color: self.hot_color,
            color_cycle_rate: self.color_cycle_rate,
            inner_alpha: self.inner_alpha,
        }
    }
}

impl ChromaticBulgeGridLfoShape {
    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        for point in &mut normalized.points {
            point.phase = point.phase.clamp(0.0, 1.0);
            point.value = point.value.clamp(0.0, 1.0);
            point.curve_to_next = point.curve_to_next.clamp(-1.0, 1.0);
        }
        normalized
            .points
            .sort_by(|left, right| left.phase.total_cmp(&right.phase));
        normalized
            .points
            .dedup_by(|left, right| (left.phase - right.phase).abs() < 0.0001);
        normalized.points.truncate(64);
        if let Some(last) = normalized.points.last_mut() {
            last.curve_to_next = 0.0;
        }
        normalized
    }
}

impl TrackVisualizerConfig {
    pub fn normalized_for_export(&self) -> Self {
        let mut normalized = self.clone();
        normalized.params = normalized.params.normalized();
        if let Some(timeline) = normalized.timeline.take() {
            normalized.timeline = Some(timeline.normalized());
            normalized.automation = None;
        } else {
            normalized.automation = normalized
                .automation
                .map(|automation| automation.normalized());
        }
        normalized
    }

    pub fn with_synced_base_states(mut self) -> Self {
        if let Some(states) = self.params.shader_states.as_mut() {
            states.idle = states.playing;
        }
        self
    }

    pub fn resolve_chromatic_bulge_grid(
        &self,
        playback: PlaybackClock,
    ) -> ChromaticBulgeGridResolvedState {
        let states = self.params.shader_states.unwrap_or_else(|| {
            let state = ChromaticBulgeGridShaderState::from_legacy_params(&self.params);
            ChromaticBulgeGridShaderStates {
                playing: state,
                idle: state,
            }
        });
        let bpm = self
            .timeline
            .as_ref()
            .map(|timeline| timeline.bpm)
            .or_else(|| self.automation.as_ref().map(|automation| automation.bpm))
            .unwrap_or_else(default_bpm)
            .max(1.0);
        let current_beat = playback.current_time_secs.max(0.0) * bpm / 60.0;
        let base = if playback.is_playing {
            states.playing
        } else {
            states.idle
        };

        let uniforms = if let Some(timeline) = &self.timeline {
            if playback.is_playing || playback.timeline_preview {
                timeline
                    .resolve_state_at_beat(base, current_beat)
                    .unwrap_or(base)
            } else {
                base
            }
        } else if let Some(automation) = &self.automation {
            if playback.is_playing {
                automation.apply_to_state(base, current_beat)
            } else {
                base
            }
        } else {
            base
        };

        ChromaticBulgeGridResolvedState {
            uniforms: uniforms.clamp(),
            current_beat,
        }
    }
}

impl TrackVisualizerParams {
    fn normalized(&self) -> Self {
        Self {
            motion_rate: self.motion_rate,
            energy_gain: self.energy_gain,
            bass_gain: self.bass_gain,
            mid_gain: self.mid_gain,
            treble_gain: self.treble_gain,
            ring_count: self.ring_count,
            particle_count: self.particle_count,
            lattice_density: self.lattice_density,
            shader_states: self
                .shader_states
                .map(|states| ChromaticBulgeGridShaderStates {
                    playing: states.playing.clamp(),
                    idle: states.idle.clamp(),
                }),
        }
    }
}

impl ChromaticBulgeGridAutomation {
    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.bpm = normalized.bpm.max(1.0);
        normalized.measures = normalized.measures.max(1);
        normalized.beats_per_measure = normalized.beats_per_measure.max(1);
        normalized.lanes.sort_all();
        normalized
    }

    pub fn total_beats(&self) -> f32 {
        self.measures.max(1) as f32 * self.beats_per_measure.max(1) as f32
    }

    fn apply_to_state(
        &self,
        base: ChromaticBulgeGridShaderState,
        beat: f32,
    ) -> ChromaticBulgeGridShaderState {
        apply_lanes_to_state(base, &self.normalized().lanes, beat)
    }
}

impl ChromaticBulgeGridClipTimeline {
    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.bpm = normalized.bpm.max(1.0);
        normalized.measures = normalized.measures.max(1);
        normalized.beats_per_measure = normalized.beats_per_measure.max(1);
        for clip in &mut normalized.clips {
            clip.length_beats = clip.length_beats.max(0.0001);
            clip.color = clamp_color(clip.color);
            clip.lanes.sort_all();
            if let Some(ChromaticBulgeGridClipSource::Lfo(lfo)) = &mut clip.source {
                lfo.period_beats = lfo.period_beats.max(0.0001);
                lfo.shape = lfo.shape.normalized();
            }
        }
        for placement in &mut normalized.arrangement {
            placement.start_beat = placement.start_beat.max(0.0);
            placement.track = placement
                .track
                .min((ChromaticBulgeGridLaneId::ALL.len().saturating_sub(1)) as u8);
            placement.repeats = placement.repeats.max(1);
        }
        normalized
    }

    pub fn total_beats(&self) -> f32 {
        self.measures.max(1) as f32 * self.beats_per_measure.max(1) as f32
    }

    pub fn clip_by_id(&self, clip_id: &str) -> Option<&ChromaticBulgeGridClip> {
        self.clips.iter().find(|clip| clip.id == clip_id)
    }

    pub fn resolve_state_at_beat(
        &self,
        base: ChromaticBulgeGridShaderState,
        beat: f32,
    ) -> Option<ChromaticBulgeGridShaderState> {
        let held = self.held_clips_at_beat(beat);
        if held.is_empty() {
            return None;
        }
        let mut state = base;
        for (clip, local_beat) in held {
            state = clip.apply_to_state(state, local_beat, beat);
        }
        Some(state)
    }

    pub fn held_clips_at_beat(&self, beat: f32) -> Vec<(&ChromaticBulgeGridClip, f32)> {
        let beat = beat.max(0.0);
        let mut held: Vec<Option<(f32, &ChromaticBulgeGridClip, f32)>> =
            vec![None; ChromaticBulgeGridLaneId::ALL.len()];
        for placement in &self.arrangement {
            let Some(clip) = self.clip_by_id(&placement.clip_id) else {
                continue;
            };
            if beat < placement.start_beat {
                continue;
            }
            let end = placement.end_beat(clip);
            let local = if clip.length_beats <= 0.0 {
                0.0
            } else if beat < end {
                ((beat - placement.start_beat) % clip.length_beats).clamp(0.0, clip.length_beats)
            } else {
                clip.length_beats
            };
            let slot = &mut held[placement.track as usize];
            if slot
                .map(|(start_beat, _, _)| placement.start_beat >= start_beat)
                .unwrap_or(true)
            {
                *slot = Some((placement.start_beat, clip, local));
            }
        }
        held.into_iter()
            .flatten()
            .map(|(_, clip, local)| (clip, local))
            .collect()
    }

    pub fn active_clips_at_beat(&self, beat: f32) -> Vec<(&ChromaticBulgeGridClip, f32)> {
        let beat = beat.max(0.0);
        let mut active = Vec::new();
        for placement in &self.arrangement {
            let Some(clip) = self.clip_by_id(&placement.clip_id) else {
                continue;
            };
            let end = placement.end_beat(clip);
            if beat >= placement.start_beat && beat < end {
                let local = if clip.length_beats <= 0.0 {
                    0.0
                } else {
                    ((beat - placement.start_beat) % clip.length_beats)
                        .clamp(0.0, clip.length_beats)
                };
                active.push((placement.track, placement.start_beat, clip, local));
            }
        }
        active.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
        });
        active
            .into_iter()
            .map(|(_, _, clip, local)| (clip, local))
            .collect()
    }

    pub fn has_authored_content(&self) -> bool {
        !self.clips.is_empty() || !self.arrangement.is_empty()
    }
}

impl ChromaticBulgeGridAutomationLanes {
    pub fn sort_all(&mut self) {
        sort_float_keyframes(&mut self.motion_rate);
        sort_float_keyframes(&mut self.motion_rate_y);
        sort_float_keyframes(&mut self.lattice_density);
        sort_float_keyframes(&mut self.circle_radius);
        sort_float_keyframes(&mut self.circle_falloff_start);
        sort_float_keyframes(&mut self.circle_falloff_end);
        sort_float_keyframes(&mut self.bulge_amount);
        sort_float_keyframes(&mut self.rim_guard);
        sort_float_keyframes(&mut self.rim_exponent);
        sort_float_keyframes(&mut self.rim_warp);
        sort_float_keyframes(&mut self.dot_size);
        sort_float_keyframes(&mut self.outer_dot_scale);
        sort_float_keyframes(&mut self.edge_softness);
        sort_float_keyframes(&mut self.chromatic_aberration);
        sort_color_keyframes(&mut self.cold_color);
        sort_color_keyframes(&mut self.hot_color);
        sort_float_keyframes(&mut self.color_cycle_rate);
        sort_float_keyframes(&mut self.inner_alpha);
    }

    pub fn is_empty(&self) -> bool {
        self.non_empty_lane_ids().is_empty()
    }

    pub fn non_empty_lane_ids(&self) -> Vec<ChromaticBulgeGridLaneId> {
        ChromaticBulgeGridLaneId::ALL
            .into_iter()
            .filter(|lane| self.lane_has_values(*lane))
            .collect()
    }

    pub fn lane_has_values(&self, lane: ChromaticBulgeGridLaneId) -> bool {
        match lane {
            ChromaticBulgeGridLaneId::MotionRate => !self.motion_rate.is_empty(),
            ChromaticBulgeGridLaneId::MotionRateY => !self.motion_rate_y.is_empty(),
            ChromaticBulgeGridLaneId::LatticeDensity => !self.lattice_density.is_empty(),
            ChromaticBulgeGridLaneId::CircleRadius => !self.circle_radius.is_empty(),
            ChromaticBulgeGridLaneId::CircleFalloffStart => !self.circle_falloff_start.is_empty(),
            ChromaticBulgeGridLaneId::CircleFalloffEnd => !self.circle_falloff_end.is_empty(),
            ChromaticBulgeGridLaneId::BulgeAmount => !self.bulge_amount.is_empty(),
            ChromaticBulgeGridLaneId::RimGuard => !self.rim_guard.is_empty(),
            ChromaticBulgeGridLaneId::RimExponent => !self.rim_exponent.is_empty(),
            ChromaticBulgeGridLaneId::RimWarp => !self.rim_warp.is_empty(),
            ChromaticBulgeGridLaneId::DotSize => !self.dot_size.is_empty(),
            ChromaticBulgeGridLaneId::OuterDotScale => !self.outer_dot_scale.is_empty(),
            ChromaticBulgeGridLaneId::EdgeSoftness => !self.edge_softness.is_empty(),
            ChromaticBulgeGridLaneId::ChromaticAberration => !self.chromatic_aberration.is_empty(),
            ChromaticBulgeGridLaneId::ColdColor => !self.cold_color.is_empty(),
            ChromaticBulgeGridLaneId::HotColor => !self.hot_color.is_empty(),
            ChromaticBulgeGridLaneId::ColorCycleRate => !self.color_cycle_rate.is_empty(),
            ChromaticBulgeGridLaneId::InnerAlpha => !self.inner_alpha.is_empty(),
        }
    }

    pub fn only_lane(&self, lane: ChromaticBulgeGridLaneId) -> Self {
        let mut subset = Self::default();
        match lane {
            ChromaticBulgeGridLaneId::MotionRate => subset.motion_rate = self.motion_rate.clone(),
            ChromaticBulgeGridLaneId::MotionRateY => {
                subset.motion_rate_y = self.motion_rate_y.clone()
            }
            ChromaticBulgeGridLaneId::LatticeDensity => {
                subset.lattice_density = self.lattice_density.clone()
            }
            ChromaticBulgeGridLaneId::CircleRadius => {
                subset.circle_radius = self.circle_radius.clone()
            }
            ChromaticBulgeGridLaneId::CircleFalloffStart => {
                subset.circle_falloff_start = self.circle_falloff_start.clone()
            }
            ChromaticBulgeGridLaneId::CircleFalloffEnd => {
                subset.circle_falloff_end = self.circle_falloff_end.clone()
            }
            ChromaticBulgeGridLaneId::BulgeAmount => {
                subset.bulge_amount = self.bulge_amount.clone()
            }
            ChromaticBulgeGridLaneId::RimGuard => subset.rim_guard = self.rim_guard.clone(),
            ChromaticBulgeGridLaneId::RimExponent => {
                subset.rim_exponent = self.rim_exponent.clone()
            }
            ChromaticBulgeGridLaneId::RimWarp => subset.rim_warp = self.rim_warp.clone(),
            ChromaticBulgeGridLaneId::DotSize => subset.dot_size = self.dot_size.clone(),
            ChromaticBulgeGridLaneId::OuterDotScale => {
                subset.outer_dot_scale = self.outer_dot_scale.clone()
            }
            ChromaticBulgeGridLaneId::EdgeSoftness => {
                subset.edge_softness = self.edge_softness.clone()
            }
            ChromaticBulgeGridLaneId::ChromaticAberration => {
                subset.chromatic_aberration = self.chromatic_aberration.clone()
            }
            ChromaticBulgeGridLaneId::ColdColor => subset.cold_color = self.cold_color.clone(),
            ChromaticBulgeGridLaneId::HotColor => subset.hot_color = self.hot_color.clone(),
            ChromaticBulgeGridLaneId::ColorCycleRate => {
                subset.color_cycle_rate = self.color_cycle_rate.clone()
            }
            ChromaticBulgeGridLaneId::InnerAlpha => subset.inner_alpha = self.inner_alpha.clone(),
        }
        subset
    }
}

pub fn compile_authoring_lanes(
    base: ChromaticBulgeGridShaderState,
    clip_length_beats: f32,
    authoring: &ChromaticBulgeGridClipAuthoring,
) -> ChromaticBulgeGridAutomationLanes {
    use std::f32::consts::PI;

    fn lerp(start: f32, end: f32, t: f32) -> f32 {
        start + (end - start) * t
    }

    fn ease_progress(ease: ClipTweenEase, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match ease {
            ClipTweenEase::Hold => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            ClipTweenEase::Linear => t,
            ClipTweenEase::SineIn => 1.0 - ((t * PI) / 2.0).cos(),
            ClipTweenEase::SineOut => ((t * PI) / 2.0).sin(),
            ClipTweenEase::SineInOut => -(PI * t).cos() * 0.5 + 0.5,
        }
    }

    fn interpolation_for_start(ease: ClipTweenEase) -> InterpolationMode {
        match ease {
            ClipTweenEase::Hold => InterpolationMode::Hold,
            ClipTweenEase::Linear
            | ClipTweenEase::SineIn
            | ClipTweenEase::SineOut
            | ClipTweenEase::SineInOut => InterpolationMode::Linear,
        }
    }

    fn segments_for_ease(ease: ClipTweenEase) -> usize {
        match ease {
            ClipTweenEase::Hold | ClipTweenEase::Linear => 1,
            ClipTweenEase::SineIn | ClipTweenEase::SineOut => 4,
            ClipTweenEase::SineInOut => 6,
        }
    }

    fn push_float_keyframe(
        keyframes: &mut Vec<FloatKeyframe>,
        beat: f32,
        value: f32,
        interpolation: InterpolationMode,
    ) {
        keyframes.push(FloatKeyframe {
            beat,
            value,
            interpolation,
        });
    }

    fn push_color_keyframe(
        keyframes: &mut Vec<ColorKeyframe>,
        beat: f32,
        value: [f32; 3],
        interpolation: InterpolationMode,
    ) {
        keyframes.push(ColorKeyframe {
            beat,
            value,
            interpolation,
        });
    }

    fn float_lane_mut(
        lanes: &mut ChromaticBulgeGridAutomationLanes,
        lane: ChromaticBulgeGridLaneId,
    ) -> &mut Vec<FloatKeyframe> {
        match lane {
            ChromaticBulgeGridLaneId::MotionRate => &mut lanes.motion_rate,
            ChromaticBulgeGridLaneId::MotionRateY => &mut lanes.motion_rate_y,
            ChromaticBulgeGridLaneId::LatticeDensity => &mut lanes.lattice_density,
            ChromaticBulgeGridLaneId::CircleRadius => &mut lanes.circle_radius,
            ChromaticBulgeGridLaneId::CircleFalloffStart => &mut lanes.circle_falloff_start,
            ChromaticBulgeGridLaneId::CircleFalloffEnd => &mut lanes.circle_falloff_end,
            ChromaticBulgeGridLaneId::BulgeAmount => &mut lanes.bulge_amount,
            ChromaticBulgeGridLaneId::RimGuard => &mut lanes.rim_guard,
            ChromaticBulgeGridLaneId::RimExponent => &mut lanes.rim_exponent,
            ChromaticBulgeGridLaneId::RimWarp => &mut lanes.rim_warp,
            ChromaticBulgeGridLaneId::DotSize => &mut lanes.dot_size,
            ChromaticBulgeGridLaneId::OuterDotScale => &mut lanes.outer_dot_scale,
            ChromaticBulgeGridLaneId::EdgeSoftness => &mut lanes.edge_softness,
            ChromaticBulgeGridLaneId::ChromaticAberration => &mut lanes.chromatic_aberration,
            ChromaticBulgeGridLaneId::ColorCycleRate => &mut lanes.color_cycle_rate,
            ChromaticBulgeGridLaneId::InnerAlpha => &mut lanes.inner_alpha,
            ChromaticBulgeGridLaneId::ColdColor | ChromaticBulgeGridLaneId::HotColor => {
                unreachable!("color lane requested as float")
            }
        }
    }

    fn color_lane_mut(
        lanes: &mut ChromaticBulgeGridAutomationLanes,
        lane: ChromaticBulgeGridLaneId,
    ) -> &mut Vec<ColorKeyframe> {
        match lane {
            ChromaticBulgeGridLaneId::ColdColor => &mut lanes.cold_color,
            ChromaticBulgeGridLaneId::HotColor => &mut lanes.hot_color,
            _ => unreachable!("float lane requested as color"),
        }
    }

    let mut lanes = ChromaticBulgeGridAutomationLanes::default();

    for track in &authoring.tracks {
        if track.lane.is_color() {
            let mut cursor = 0.0f32;
            let mut current = match track.lane {
                ChromaticBulgeGridLaneId::ColdColor => base.cold_color,
                ChromaticBulgeGridLaneId::HotColor => base.hot_color,
                _ => unreachable!(),
            };
            let lane_keyframes = color_lane_mut(&mut lanes, track.lane);
            for step in &track.steps {
                let ClipTweenValue::Color(target) = step.to else {
                    continue;
                };
                let duration = step.duration_beats.max(0.0);
                let end_beat = (cursor + duration).min(clip_length_beats.max(0.0));
                push_color_keyframe(
                    lane_keyframes,
                    cursor,
                    current,
                    interpolation_for_start(step.ease),
                );
                if duration <= 0.0001 || end_beat <= cursor || step.ease == ClipTweenEase::Hold {
                    push_color_keyframe(lane_keyframes, end_beat, target, InterpolationMode::Hold);
                    cursor = end_beat;
                    current = target;
                    continue;
                }
                let segments = segments_for_ease(step.ease);
                for index in 1..=segments {
                    let t = index as f32 / segments as f32;
                    let beat = cursor + (end_beat - cursor) * t;
                    let progress = ease_progress(step.ease, t);
                    let interpolation = if index == segments {
                        InterpolationMode::Hold
                    } else {
                        InterpolationMode::Linear
                    };
                    push_color_keyframe(
                        lane_keyframes,
                        beat,
                        [
                            lerp(current[0], target[0], progress),
                            lerp(current[1], target[1], progress),
                            lerp(current[2], target[2], progress),
                        ],
                        interpolation,
                    );
                }
                cursor = end_beat;
                current = target;
            }
        } else {
            let mut cursor = 0.0f32;
            let mut current = base_float_for_lane(base, track.lane);
            let lane_keyframes = float_lane_mut(&mut lanes, track.lane);
            for step in &track.steps {
                let ClipTweenValue::Float(target) = step.to else {
                    continue;
                };
                let duration = step.duration_beats.max(0.0);
                let end_beat = (cursor + duration).min(clip_length_beats.max(0.0));
                push_float_keyframe(
                    lane_keyframes,
                    cursor,
                    current,
                    interpolation_for_start(step.ease),
                );
                if duration <= 0.0001 || end_beat <= cursor || step.ease == ClipTweenEase::Hold {
                    push_float_keyframe(lane_keyframes, end_beat, target, InterpolationMode::Hold);
                    cursor = end_beat;
                    current = target;
                    continue;
                }
                let segments = segments_for_ease(step.ease);
                for index in 1..=segments {
                    let t = index as f32 / segments as f32;
                    let beat = cursor + (end_beat - cursor) * t;
                    let interpolation = if index == segments {
                        InterpolationMode::Hold
                    } else {
                        InterpolationMode::Linear
                    };
                    push_float_keyframe(
                        lane_keyframes,
                        beat,
                        lerp(current, target, ease_progress(step.ease, t)),
                        interpolation,
                    );
                }
                cursor = end_beat;
                current = target;
            }
        }
    }

    lanes.sort_all();
    lanes
}

pub fn legacy_automation_to_timeline(
    automation: &ChromaticBulgeGridAutomation,
) -> ChromaticBulgeGridClipTimeline {
    let total_beats = automation.total_beats();
    let lane_clips = automation
        .lanes
        .non_empty_lane_ids()
        .into_iter()
        .enumerate()
        .map(|(index, lane)| ChromaticBulgeGridClip {
            id: format!("imported_{}", lane.label()),
            name: format!("Imported {}", lane.label()),
            length_beats: total_beats,
            color: default_clip_color_for_index(index),
            source: None,
            authoring: None,
            lanes: automation.lanes.only_lane(lane),
        })
        .collect::<Vec<_>>();
    let clips = if lane_clips.is_empty() {
        vec![ChromaticBulgeGridClip {
            id: "imported_timeline".to_string(),
            name: "Imported Timeline".to_string(),
            length_beats: total_beats,
            color: default_clip_color(),
            source: None,
            authoring: None,
            lanes: automation.lanes.clone(),
        }]
    } else {
        lane_clips
    };
    let arrangement = clips
        .iter()
        .filter_map(|clip| {
            let lane = clip.primary_lane()?;
            Some(ClipPlacement {
                clip_id: clip.id.clone(),
                start_beat: 0.0,
                track: lane.track_index(),
                repeats: 1,
            })
        })
        .collect::<Vec<_>>();
    ChromaticBulgeGridClipTimeline {
        bpm: automation.bpm,
        measures: automation.measures,
        beats_per_measure: automation.beats_per_measure,
        clips,
        arrangement,
    }
}

pub fn validate_visualizer_config(
    record_id: &str,
    visualizer: &TrackVisualizerConfig,
) -> Result<(), VfxConfigError> {
    let has_automation = visualizer
        .automation
        .as_ref()
        .is_some_and(|automation| !automation.lanes.is_empty());
    let has_timeline = visualizer
        .timeline
        .as_ref()
        .is_some_and(ChromaticBulgeGridClipTimeline::has_authored_content);
    if has_automation && has_timeline {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} contains both automation and timeline"
        )));
    }
    if visualizer.automation.is_some() || visualizer.timeline.is_some() {
        if visualizer.mode != TrackVisualizerMode::ChromaticBulgeGrid {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} uses automation/timeline on unsupported visualizer mode {}",
                visualizer.mode.label()
            )));
        }
    }
    if let Some(automation) = &visualizer.automation {
        validate_automation(record_id, automation)?;
    }
    if let Some(timeline) = &visualizer.timeline {
        validate_clip_timeline(record_id, timeline)?;
    }
    Ok(())
}

pub fn parse_visualizer_json(json: &str) -> Result<TrackVisualizerConfig, VfxConfigError> {
    let raw = serde_json::from_str::<serde_json::Value>(json)?;
    reject_legacy_lfo_shape_schema(&raw)?;
    let config = serde_json::from_value::<TrackVisualizerConfig>(raw)?;
    validate_visualizer_config("visualizer", &config)?;
    Ok(config)
}

#[cfg(feature = "wasm")]
mod wasm_api {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn normalize_visualizer_json(json: &str) -> Result<String, JsValue> {
        let config =
            parse_visualizer_json(json).map_err(|error| JsValue::from_str(&error.to_string()))?;
        serde_json::to_string(&config.normalized_for_export())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen]
    pub fn resolve_visualizer_json(
        json: &str,
        current_time_secs: f32,
        visual_time_secs: f32,
        is_playing: bool,
        timeline_preview: bool,
    ) -> Result<JsValue, JsValue> {
        let config =
            parse_visualizer_json(json).map_err(|error| JsValue::from_str(&error.to_string()))?;
        let resolved = config.resolve_chromatic_bulge_grid(PlaybackClock {
            current_time_secs,
            visual_time_secs,
            duration_secs: None,
            is_playing,
            timeline_preview,
        });
        JsValue::from_serde(&resolved).map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen]
    pub fn compile_authoring_json(
        base_json: &str,
        clip_length_beats: f32,
        authoring_json: &str,
    ) -> Result<String, JsValue> {
        let base: ChromaticBulgeGridShaderState = serde_json::from_str(base_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let authoring: ChromaticBulgeGridClipAuthoring = serde_json::from_str(authoring_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        serde_json::to_string(&compile_authoring_lanes(
            base,
            clip_length_beats,
            &authoring,
        ))
        .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}

fn reject_legacy_lfo_shape_schema(value: &serde_json::Value) -> Result<(), VfxConfigError> {
    let Some(visualizer) = value.as_object() else {
        return Ok(());
    };
    if visualizer.contains_key("lfo_library") {
        return Err(VfxConfigError::Validation(
            "visualizer uses removed lfo_library schema; each LFO clip must embed its own shape"
                .to_string(),
        ));
    }
    let Some(timeline) = visualizer.get("timeline").and_then(serde_json::Value::as_object) else {
        return Ok(());
    };
    let Some(clips) = timeline.get("clips").and_then(serde_json::Value::as_array) else {
        return Ok(());
    };
    for clip in clips {
        let Some(source) = clip
            .as_object()
            .and_then(|object| object.get("source"))
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        if source.get("kind").and_then(serde_json::Value::as_str) != Some("lfo") {
            continue;
        }
        if source.contains_key("shape_id") || !source.contains_key("shape") {
            return Err(VfxConfigError::Validation(
                "visualizer uses removed shared LFO shape references; each LFO clip must embed its own shape"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_automation(
    record_id: &str,
    automation: &ChromaticBulgeGridAutomation,
) -> Result<(), VfxConfigError> {
    if !automation.bpm.is_finite() || automation.bpm <= 0.0 {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has invalid automation bpm"
        )));
    }
    if automation.measures == 0 || automation.beats_per_measure == 0 {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has invalid automation timing"
        )));
    }
    validate_automation_lanes(record_id, "automation", &automation.lanes, None)
}

fn validate_clip_timeline(
    record_id: &str,
    timeline: &ChromaticBulgeGridClipTimeline,
) -> Result<(), VfxConfigError> {
    if !timeline.bpm.is_finite() || timeline.bpm <= 0.0 {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has invalid timeline bpm"
        )));
    }
    if timeline.measures == 0 || timeline.beats_per_measure == 0 {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has invalid timeline timing"
        )));
    }
    let total_beats = timeline.total_beats();
    let mut clip_ids = HashSet::new();
    let mut spans = Vec::<(f32, f32, u8, &str, HashSet<ChromaticBulgeGridLaneId>)>::new();
    for clip in &timeline.clips {
        if clip.id.trim().is_empty() || clip.name.trim().is_empty() {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has clip with empty id or name"
            )));
        }
        if !clip_ids.insert(clip.id.as_str()) {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has duplicate clip id {}",
                clip.id
            )));
        }
        if !clip.length_beats.is_finite() || clip.length_beats <= 0.0 {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has invalid length_beats for clip {}",
                clip.id
            )));
        }
        if let Some(ChromaticBulgeGridClipSource::Lfo(lfo)) = &clip.source {
            validate_lfo_clip(record_id, clip, lfo)?;
        }
        validate_automation_lanes(
            record_id,
            &format!("clip {}", clip.id),
            &clip.lanes,
            Some(clip.length_beats),
        )?;
    }
    for placement in &timeline.arrangement {
        let Some(clip) = timeline.clip_by_id(&placement.clip_id) else {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} placement references missing clip {}",
                placement.clip_id
            )));
        };
        if !placement.start_beat.is_finite() || placement.start_beat < 0.0 || placement.repeats == 0
        {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has invalid placement for {}",
                placement.clip_id
            )));
        }
        let end = placement.end_beat(clip);
        if end > total_beats + 0.0001 {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} placement for {} extends beyond total timeline beats",
                placement.clip_id
            )));
        }
        spans.push((
            placement.start_beat,
            end,
            placement.track,
            placement.clip_id.as_str(),
            clip.authored_lanes(),
        ));
    }
    spans.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.2.cmp(&right.2)));
    for (index, left) in spans.iter().enumerate() {
        for right in spans.iter().skip(index + 1) {
            if right.0 >= left.1 - 0.0001 {
                break;
            }
            if left.2 == right.2 || !left.4.is_disjoint(&right.4) {
                return Err(VfxConfigError::Validation(format!(
                    "record {record_id} has overlapping placements between {} and {}",
                    left.3, right.3
                )));
            }
        }
    }
    Ok(())
}

fn validate_lfo_clip(
    record_id: &str,
    clip: &ChromaticBulgeGridClip,
    lfo: &ChromaticBulgeGridLfoClip,
) -> Result<(), VfxConfigError> {
    if !lfo.lane.supports_lfo() {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has unsupported lfo lane {} in clip {}",
            lfo.lane.label(),
            clip.id
        )));
    }
    if lfo.shape.points.is_empty() || lfo.shape.points.len() > 64 {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has invalid point count in clip-local lfo shape for clip {}",
            clip.id
        )));
    }
    if !lfo.min.is_finite()
        || !lfo.max.is_finite()
        || !lfo.period_beats.is_finite()
        || lfo.period_beats <= 0.0
        || !lfo.phase_offset_beats.is_finite()
    {
        return Err(VfxConfigError::Validation(format!(
            "record {record_id} has invalid lfo values in clip {}",
            clip.id
        )));
    }
    let mut points = lfo.shape.points.clone();
    for point in &points {
        if !point.phase.is_finite()
            || !(0.0..=1.0).contains(&point.phase)
            || !point.value.is_finite()
            || !(0.0..=1.0).contains(&point.value)
        {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has invalid clip-local lfo point in clip {}",
                clip.id
            )));
        }
    }
    points.sort_by(|left, right| left.phase.total_cmp(&right.phase));
    for window in points.windows(2) {
        if (window[0].phase - window[1].phase).abs() < 0.0001 {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has duplicate phase {} in clip-local lfo shape for clip {}",
                window[1].phase, clip.id
            )));
        }
    }
    Ok(())
}

fn validate_automation_lanes(
    record_id: &str,
    lane_prefix: &str,
    lanes: &ChromaticBulgeGridAutomationLanes,
    max_beat: Option<f32>,
) -> Result<(), VfxConfigError> {
    for (name, beats) in [
        (
            "motion_rate",
            lanes.motion_rate.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "motion_rate_y",
            lanes.motion_rate_y.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "lattice_density",
            lanes
                .lattice_density
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "circle_radius",
            lanes
                .circle_radius
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "circle_falloff_start",
            lanes
                .circle_falloff_start
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "circle_falloff_end",
            lanes
                .circle_falloff_end
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "bulge_amount",
            lanes
                .bulge_amount
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "rim_guard",
            lanes.rim_guard.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "rim_exponent",
            lanes
                .rim_exponent
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "rim_warp",
            lanes.rim_warp.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "dot_size",
            lanes.dot_size.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "outer_dot_scale",
            lanes
                .outer_dot_scale
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "edge_softness",
            lanes
                .edge_softness
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "chromatic_aberration",
            lanes
                .chromatic_aberration
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "cold_color",
            lanes.cold_color.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "hot_color",
            lanes.hot_color.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
        (
            "color_cycle_rate",
            lanes
                .color_cycle_rate
                .iter()
                .map(|k| k.beat)
                .collect::<Vec<_>>(),
        ),
        (
            "inner_alpha",
            lanes.inner_alpha.iter().map(|k| k.beat).collect::<Vec<_>>(),
        ),
    ] {
        validate_duplicate_beats(record_id, lane_prefix, name, beats.into_iter(), max_beat)?;
    }
    Ok(())
}

fn validate_duplicate_beats(
    record_id: &str,
    lane_prefix: &str,
    lane_name: &str,
    beats: impl Iterator<Item = f32>,
    max_beat: Option<f32>,
) -> Result<(), VfxConfigError> {
    let mut unique = Vec::<f32>::new();
    for beat in beats {
        if !beat.is_finite() || beat < 0.0 {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has invalid beat in {lane_prefix} lane {lane_name}"
            )));
        }
        if let Some(limit) = max_beat {
            if beat > limit + 0.0001 {
                return Err(VfxConfigError::Validation(format!(
                    "record {record_id} has beat {beat} beyond clip length in {lane_prefix} lane {lane_name}"
                )));
            }
        }
        if unique.iter().any(|seen| (seen - beat).abs() < 0.0001) {
            return Err(VfxConfigError::Validation(format!(
                "record {record_id} has duplicate beat {beat} in {lane_prefix} lane {lane_name}"
            )));
        }
        unique.push(beat);
    }
    Ok(())
}

fn apply_lanes_to_state(
    base: ChromaticBulgeGridShaderState,
    lanes: &ChromaticBulgeGridAutomationLanes,
    beat: f32,
) -> ChromaticBulgeGridShaderState {
    ChromaticBulgeGridShaderState {
        motion_rate: sample_float_lane(&lanes.motion_rate, beat, base.motion_rate),
        motion_rate_y: sample_float_lane(&lanes.motion_rate_y, beat, base.motion_rate_y),
        lattice_density: sample_float_lane(&lanes.lattice_density, beat, base.lattice_density),
        circle_radius: sample_float_lane(&lanes.circle_radius, beat, base.circle_radius),
        circle_falloff_start: sample_float_lane(
            &lanes.circle_falloff_start,
            beat,
            base.circle_falloff_start,
        ),
        circle_falloff_end: sample_float_lane(
            &lanes.circle_falloff_end,
            beat,
            base.circle_falloff_end,
        ),
        bulge_amount: sample_float_lane(&lanes.bulge_amount, beat, base.bulge_amount),
        rim_guard: sample_float_lane(&lanes.rim_guard, beat, base.rim_guard),
        rim_exponent: sample_float_lane(&lanes.rim_exponent, beat, base.rim_exponent),
        rim_warp: sample_float_lane(&lanes.rim_warp, beat, base.rim_warp),
        dot_size: sample_float_lane(&lanes.dot_size, beat, base.dot_size),
        outer_dot_scale: sample_float_lane(&lanes.outer_dot_scale, beat, base.outer_dot_scale),
        edge_softness: sample_float_lane(&lanes.edge_softness, beat, base.edge_softness),
        chromatic_aberration: sample_float_lane(
            &lanes.chromatic_aberration,
            beat,
            base.chromatic_aberration,
        ),
        cold_color: sample_color_lane(&lanes.cold_color, beat, base.cold_color),
        hot_color: sample_color_lane(&lanes.hot_color, beat, base.hot_color),
        color_cycle_rate: sample_float_lane(&lanes.color_cycle_rate, beat, base.color_cycle_rate),
        inner_alpha: sample_float_lane(&lanes.inner_alpha, beat, base.inner_alpha),
    }
}

fn apply_lfo_clip_to_state(
    base: ChromaticBulgeGridShaderState,
    clip: &ChromaticBulgeGridLfoClip,
    local_beat: f32,
    global_beat: f32,
) -> ChromaticBulgeGridShaderState {
    let phase_beats = match clip.start_mode {
        LfoStartMode::Retrigger => local_beat + clip.phase_offset_beats,
        LfoStartMode::Continue => global_beat + clip.phase_offset_beats,
    };
    let phase = (phase_beats / clip.period_beats.max(0.0001)).rem_euclid(1.0);
    let value = clip.min + (clip.max - clip.min) * sample_lfo_shape(&clip.shape, phase);
    apply_lfo_value_to_state(base, clip.lane, value)
}

fn sample_lfo_shape(shape: &ChromaticBulgeGridLfoShape, phase: f32) -> f32 {
    if shape.points.is_empty() {
        return 0.0;
    }
    if shape.points.len() == 1 {
        return shape.points[0].value.clamp(0.0, 1.0);
    }
    let normalized = shape.normalized();
    let phase = phase.rem_euclid(1.0);
    let points = &normalized.points;
    if phase <= points[0].phase {
        let first = points[0];
        let last = points[points.len() - 1];
        let span = (first.phase + 1.0 - last.phase).max(0.0001);
        let wrapped_phase = if phase < first.phase { phase + 1.0 } else { phase };
        let t = ((wrapped_phase - last.phase) / span).clamp(0.0, 1.0);
        return lerp(last.value, first.value, t);
    }
    for window in points.windows(2) {
        let left = window[0];
        let right = window[1];
        if phase <= right.phase {
            let span = (right.phase - left.phase).max(0.0001);
            let t = ((phase - left.phase) / span).clamp(0.0, 1.0);
            return quadratic_bezier(left.value, lfo_control_value(left, right), right.value, t);
        }
    }
    let first = points[0];
    let last = points[points.len() - 1];
    let span = (first.phase + 1.0 - last.phase).max(0.0001);
    let t = ((phase + 1.0 - last.phase) / span).clamp(0.0, 1.0);
    lerp(last.value, first.value, t)
}

fn apply_lfo_value_to_state(
    mut state: ChromaticBulgeGridShaderState,
    lane: ChromaticBulgeGridLaneId,
    value: f32,
) -> ChromaticBulgeGridShaderState {
    match lane {
        ChromaticBulgeGridLaneId::MotionRate => state.motion_rate = value,
        ChromaticBulgeGridLaneId::MotionRateY => state.motion_rate_y = value,
        ChromaticBulgeGridLaneId::LatticeDensity => state.lattice_density = value,
        ChromaticBulgeGridLaneId::CircleRadius => state.circle_radius = value,
        ChromaticBulgeGridLaneId::CircleFalloffStart => state.circle_falloff_start = value,
        ChromaticBulgeGridLaneId::CircleFalloffEnd => state.circle_falloff_end = value,
        ChromaticBulgeGridLaneId::BulgeAmount => state.bulge_amount = value,
        ChromaticBulgeGridLaneId::RimGuard => state.rim_guard = value,
        ChromaticBulgeGridLaneId::RimExponent => state.rim_exponent = value,
        ChromaticBulgeGridLaneId::RimWarp => state.rim_warp = value,
        ChromaticBulgeGridLaneId::DotSize => state.dot_size = value,
        ChromaticBulgeGridLaneId::OuterDotScale => state.outer_dot_scale = value,
        ChromaticBulgeGridLaneId::EdgeSoftness => state.edge_softness = value,
        ChromaticBulgeGridLaneId::ChromaticAberration => state.chromatic_aberration = value,
        ChromaticBulgeGridLaneId::ColorCycleRate => state.color_cycle_rate = value,
        ChromaticBulgeGridLaneId::InnerAlpha => state.inner_alpha = value,
        ChromaticBulgeGridLaneId::ColdColor | ChromaticBulgeGridLaneId::HotColor => {}
    }
    state
}

fn sample_float_lane(keyframes: &[FloatKeyframe], beat: f32, base: f32) -> f32 {
    if keyframes.is_empty() {
        return base;
    }
    let clamped_beat = beat.max(0.0);
    if clamped_beat < keyframes[0].beat {
        return base;
    }
    for window in keyframes.windows(2) {
        let left = window[0];
        let right = window[1];
        if clamped_beat < right.beat {
            return match left.interpolation {
                InterpolationMode::Hold => left.value,
                InterpolationMode::Linear => {
                    let span = (right.beat - left.beat).max(0.0001);
                    let t = ((clamped_beat - left.beat) / span).clamp(0.0, 1.0);
                    lerp(left.value, right.value, t)
                }
            };
        }
    }
    keyframes.last().map_or(base, |keyframe| keyframe.value)
}

fn sample_color_lane(keyframes: &[ColorKeyframe], beat: f32, base: [f32; 3]) -> [f32; 3] {
    if keyframes.is_empty() {
        return base;
    }
    let clamped_beat = beat.max(0.0);
    if clamped_beat < keyframes[0].beat {
        return base;
    }
    for window in keyframes.windows(2) {
        let left = window[0];
        let right = window[1];
        if clamped_beat < right.beat {
            return match left.interpolation {
                InterpolationMode::Hold => left.value,
                InterpolationMode::Linear => {
                    let span = (right.beat - left.beat).max(0.0001);
                    let t = ((clamped_beat - left.beat) / span).clamp(0.0, 1.0);
                    [
                        lerp(left.value[0], right.value[0], t),
                        lerp(left.value[1], right.value[1], t),
                        lerp(left.value[2], right.value[2], t),
                    ]
                }
            };
        }
    }
    keyframes.last().map_or(base, |keyframe| keyframe.value)
}

fn base_float_for_lane(base: ChromaticBulgeGridShaderState, lane: ChromaticBulgeGridLaneId) -> f32 {
    match lane {
        ChromaticBulgeGridLaneId::MotionRate => base.motion_rate,
        ChromaticBulgeGridLaneId::MotionRateY => base.motion_rate_y,
        ChromaticBulgeGridLaneId::LatticeDensity => base.lattice_density,
        ChromaticBulgeGridLaneId::CircleRadius => base.circle_radius,
        ChromaticBulgeGridLaneId::CircleFalloffStart => base.circle_falloff_start,
        ChromaticBulgeGridLaneId::CircleFalloffEnd => base.circle_falloff_end,
        ChromaticBulgeGridLaneId::BulgeAmount => base.bulge_amount,
        ChromaticBulgeGridLaneId::RimGuard => base.rim_guard,
        ChromaticBulgeGridLaneId::RimExponent => base.rim_exponent,
        ChromaticBulgeGridLaneId::RimWarp => base.rim_warp,
        ChromaticBulgeGridLaneId::DotSize => base.dot_size,
        ChromaticBulgeGridLaneId::OuterDotScale => base.outer_dot_scale,
        ChromaticBulgeGridLaneId::EdgeSoftness => base.edge_softness,
        ChromaticBulgeGridLaneId::ChromaticAberration => base.chromatic_aberration,
        ChromaticBulgeGridLaneId::ColorCycleRate => base.color_cycle_rate,
        ChromaticBulgeGridLaneId::InnerAlpha => base.inner_alpha,
        ChromaticBulgeGridLaneId::ColdColor | ChromaticBulgeGridLaneId::HotColor => unreachable!(),
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn quadratic_bezier(start: f32, control: f32, end: f32, t: f32) -> f32 {
    let inverse = 1.0 - t;
    inverse * inverse * start + 2.0 * inverse * t * control + t * t * end
}

fn lfo_control_value(left: LfoPoint, right: LfoPoint) -> f32 {
    let midpoint = (left.value + right.value) * 0.5;
    if left.curve_to_next >= 0.0 {
        lerp(midpoint, 1.0, left.curve_to_next)
    } else {
        lerp(midpoint, 0.0, left.curve_to_next.abs())
    }
}

fn lfo_curve_is_zero(value: &f32) -> bool {
    value.abs() < 0.0001
}

fn clamp_color(value: [f32; 3]) -> [f32; 3] {
    [
        value[0].clamp(0.0, 1.0),
        value[1].clamp(0.0, 1.0),
        value[2].clamp(0.0, 1.0),
    ]
}

fn sort_float_keyframes(keyframes: &mut [FloatKeyframe]) {
    keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
}

fn sort_color_keyframes(keyframes: &mut [ColorKeyframe]) {
    keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
}

fn default_motion_rate() -> f32 {
    1.0
}
fn default_motion_rate_y() -> f32 {
    0.0
}
fn default_bpm() -> f32 {
    120.0
}
fn default_beats_per_measure() -> u32 {
    4
}
fn default_repeat_count() -> u32 {
    1
}
fn default_clip_color() -> [f32; 3] {
    default_clip_color_for_index(0)
}
fn default_clip_color_for_index(index: usize) -> [f32; 3] {
    const PALETTE: [[f32; 3]; 6] = [
        [110.0 / 255.0, 220.0 / 255.0, 212.0 / 255.0],
        [229.0 / 255.0, 145.0 / 255.0, 87.0 / 255.0],
        [140.0 / 255.0, 186.0 / 255.0, 245.0 / 255.0],
        [219.0 / 255.0, 176.0 / 255.0, 89.0 / 255.0],
        [176.0 / 255.0, 217.0 / 255.0, 125.0 / 255.0],
        [232.0 / 255.0, 128.0 / 255.0, 150.0 / 255.0],
    ];
    PALETTE[index % PALETTE.len()]
}
fn default_energy_gain() -> f32 {
    1.1
}
fn default_bass_gain() -> f32 {
    1.25
}
fn default_mid_gain() -> f32 {
    1.0
}
fn default_treble_gain() -> f32 {
    1.1
}
fn default_ring_count() -> u16 {
    4
}
fn default_particle_count() -> u16 {
    48
}
fn default_lattice_density() -> u16 {
    6
}
fn default_shader_lattice_density() -> f32 {
    6.0
}
fn default_circle_radius() -> f32 {
    0.24
}
fn default_circle_falloff_start() -> f32 {
    0.78
}
fn default_circle_falloff_end() -> f32 {
    1.0
}
fn default_bulge_amount() -> f32 {
    0.42
}
fn default_rim_guard() -> f32 {
    0.55
}
fn default_rim_exponent() -> f32 {
    1.8
}
fn default_rim_warp() -> f32 {
    0.18
}
fn default_dot_size() -> f32 {
    0.16
}
fn default_outer_dot_scale() -> f32 {
    0.33
}
fn default_edge_softness() -> f32 {
    1.0
}
fn default_chromatic_aberration() -> f32 {
    0.28
}
fn default_cold_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn default_hot_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn default_color_cycle_rate() -> f32 {
    0.16
}
fn default_inner_alpha() -> f32 {
    0.9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_migration_matches_resolved_output() {
        let legacy = ChromaticBulgeGridAutomation {
            bpm: 120.0,
            measures: 8,
            beats_per_measure: 4,
            lanes: ChromaticBulgeGridAutomationLanes {
                circle_radius: vec![
                    FloatKeyframe {
                        beat: 0.0,
                        value: 0.3,
                        interpolation: InterpolationMode::Linear,
                    },
                    FloatKeyframe {
                        beat: 8.0,
                        value: 0.5,
                        interpolation: InterpolationMode::Linear,
                    },
                ],
                ..Default::default()
            },
        };
        let states = ChromaticBulgeGridShaderStates {
            playing: ChromaticBulgeGridShaderState {
                circle_radius: 0.24,
                ..Default::default()
            },
            idle: ChromaticBulgeGridShaderState {
                circle_radius: 0.18,
                ..Default::default()
            },
        };
        let legacy_config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(states),
                ..Default::default()
            },
            automation: Some(legacy.clone()),
            timeline: None,
        };
        let timeline_config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(states),
                ..Default::default()
            },
            automation: None,
            timeline: Some(legacy_automation_to_timeline(&legacy)),
        };
        let playback = PlaybackClock {
            current_time_secs: 2.0,
            visual_time_secs: 2.0,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        };
        assert_eq!(
            legacy_config
                .resolve_chromatic_bulge_grid(playback)
                .uniforms
                .circle_radius,
            timeline_config
                .resolve_chromatic_bulge_grid(playback)
                .uniforms
                .circle_radius
        );
    }

    #[test]
    fn rejects_duplicate_lane_beats() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: Default::default(),
            automation: Some(ChromaticBulgeGridAutomation {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                lanes: ChromaticBulgeGridAutomationLanes {
                    circle_radius: vec![
                        FloatKeyframe {
                            beat: 1.0,
                            value: 0.3,
                            interpolation: InterpolationMode::Hold,
                        },
                        FloatKeyframe {
                            beat: 1.0,
                            value: 0.4,
                            interpolation: InterpolationMode::Hold,
                        },
                    ],
                    ..Default::default()
                },
            }),
            timeline: None,
        };
        assert!(validate_visualizer_config("record", &config).is_err());
    }

    #[test]
    fn timeline_preview_applies_while_paused() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(ChromaticBulgeGridShaderStates {
                    playing: ChromaticBulgeGridShaderState {
                        circle_radius: 0.4,
                        ..Default::default()
                    },
                    idle: ChromaticBulgeGridShaderState {
                        circle_radius: 0.18,
                        ..Default::default()
                    },
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![ChromaticBulgeGridClip {
                    id: "pulse".to_string(),
                    name: "Pulse".to_string(),
                    length_beats: 1.0,
                    color: [1.0, 1.0, 1.0],
                    source: None,
                    authoring: None,
                    lanes: ChromaticBulgeGridAutomationLanes {
                        circle_radius: vec![FloatKeyframe {
                            beat: 0.0,
                            value: 0.5,
                            interpolation: InterpolationMode::Hold,
                        }],
                        ..Default::default()
                    },
                }],
                arrangement: vec![ClipPlacement {
                    clip_id: "pulse".to_string(),
                    start_beat: 0.0,
                    track: 0,
                    repeats: 1,
                }],
            }),
        };
        let resolved = config.resolve_chromatic_bulge_grid(PlaybackClock {
            current_time_secs: 0.0,
            visual_time_secs: 0.0,
            duration_secs: None,
            is_playing: false,
            timeline_preview: true,
        });
        assert_eq!(resolved.uniforms.circle_radius, 0.5);
    }

    #[test]
    fn lfo_timeline_samples_shape_and_range() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(ChromaticBulgeGridShaderStates {
                    playing: ChromaticBulgeGridShaderState {
                        bulge_amount: 0.1,
                        ..Default::default()
                    },
                    idle: ChromaticBulgeGridShaderState::default(),
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![ChromaticBulgeGridClip {
                    id: "lfo".to_string(),
                    name: "LFO".to_string(),
                    length_beats: 4.0,
                    color: [1.0, 1.0, 1.0],
                    source: Some(ChromaticBulgeGridClipSource::Lfo(ChromaticBulgeGridLfoClip {
                        lane: ChromaticBulgeGridLaneId::BulgeAmount,
                        shape: ChromaticBulgeGridLfoShape {
                            interpolation: LfoInterpolation::Linear,
                            points: vec![
                                LfoPoint {
                                    phase: 0.0,
                                    value: 0.0,
                                    curve_to_next: 0.0,
                                },
                                LfoPoint {
                                    phase: 0.5,
                                    value: 1.0,
                                    curve_to_next: 0.0,
                                },
                                LfoPoint {
                                    phase: 1.0,
                                    value: 0.0,
                                    curve_to_next: 0.0,
                                },
                            ],
                        },
                        min: 0.2,
                        max: 0.6,
                        period_beats: 4.0,
                        phase_offset_beats: 0.0,
                        start_mode: LfoStartMode::Retrigger,
                    })),
                    authoring: None,
                    lanes: Default::default(),
                }],
                arrangement: vec![ClipPlacement {
                    clip_id: "lfo".to_string(),
                    start_beat: 0.0,
                    track: ChromaticBulgeGridLaneId::BulgeAmount.track_index(),
                    repeats: 1,
                }],
            }),
        };
        let resolved = config.resolve_chromatic_bulge_grid(PlaybackClock {
            current_time_secs: 0.5,
            visual_time_secs: 0.5,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });
        assert!((resolved.uniforms.bulge_amount - 0.4).abs() < 0.0001);
    }

    #[test]
    fn curved_lfo_segments_preserve_linear_legacy_behavior() {
        let shape = ChromaticBulgeGridLfoShape {
            interpolation: LfoInterpolation::Linear,
            points: vec![
                LfoPoint {
                    phase: 0.0,
                    value: 0.2,
                    curve_to_next: 0.0,
                },
                LfoPoint {
                    phase: 0.75,
                    value: 0.8,
                    curve_to_next: 0.0,
                },
            ],
        };
        let sampled = sample_lfo_shape(&shape, 0.375);
        assert!((sampled - 0.5).abs() < 0.0001);
    }

    #[test]
    fn curved_lfo_segments_bow_up_and_down() {
        let bowed_up = ChromaticBulgeGridLfoShape {
            interpolation: LfoInterpolation::Linear,
            points: vec![
                LfoPoint {
                    phase: 0.0,
                    value: 0.0,
                    curve_to_next: 1.0,
                },
                LfoPoint {
                    phase: 1.0,
                    value: 1.0,
                    curve_to_next: 0.0,
                },
            ],
        };
        let bowed_down = ChromaticBulgeGridLfoShape {
            interpolation: LfoInterpolation::Linear,
            points: vec![
                LfoPoint {
                    phase: 0.0,
                    value: 0.0,
                    curve_to_next: -1.0,
                },
                LfoPoint {
                    phase: 1.0,
                    value: 1.0,
                    curve_to_next: 0.0,
                },
            ],
        };

        assert!((sample_lfo_shape(&bowed_up, 0.25) - 0.4375).abs() < 0.0001);
        assert!((sample_lfo_shape(&bowed_down, 0.25) - 0.0625).abs() < 0.0001);
    }

    #[test]
    fn lfo_shape_normalization_clamps_curve_and_zeroes_last_segment() {
        let normalized = ChromaticBulgeGridLfoShape {
            interpolation: LfoInterpolation::Linear,
            points: vec![
                LfoPoint {
                    phase: 0.5,
                    value: 0.4,
                    curve_to_next: 2.0,
                },
                LfoPoint {
                    phase: 1.2,
                    value: -1.0,
                    curve_to_next: -2.0,
                },
            ],
        }
        .normalized();

        assert_eq!(normalized.points[0].curve_to_next, 1.0);
        assert_eq!(normalized.points[1].curve_to_next, 0.0);
        assert_eq!(normalized.points[1].phase, 1.0);
        assert_eq!(normalized.points[1].value, 0.0);
    }

    #[test]
    fn lfo_shape_json_round_trip_omits_zero_curves() {
        let shape = ChromaticBulgeGridLfoShape {
            interpolation: LfoInterpolation::Linear,
            points: vec![
                LfoPoint {
                    phase: 0.0,
                    value: 0.0,
                    curve_to_next: 0.4,
                },
                LfoPoint {
                    phase: 1.0,
                    value: 1.0,
                    curve_to_next: 0.0,
                },
            ],
        };

        let value = serde_json::to_value(&shape).unwrap();
        let points = value
            .get("points")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert!(
            (points[0]
                .get("curve_to_next")
                .and_then(serde_json::Value::as_f64)
                .unwrap()
                - 0.4)
                .abs()
                < 0.0001
        );
        assert!(points[1].get("curve_to_next").is_none());

        let round_trip: ChromaticBulgeGridLfoShape = serde_json::from_value(value).unwrap();
        assert!((round_trip.points[0].curve_to_next - 0.4).abs() < 0.0001);
        assert_eq!(round_trip.points[1].curve_to_next, 0.0);
    }

    #[test]
    fn rejects_missing_lfo_shape_reference() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: Default::default(),
            automation: None,
            timeline: Some(ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 4,
                beats_per_measure: 4,
                clips: vec![ChromaticBulgeGridClip {
                    id: "clip".to_string(),
                    name: "Clip".to_string(),
                    length_beats: 4.0,
                    color: [1.0, 1.0, 1.0],
                    source: Some(ChromaticBulgeGridClipSource::Lfo(ChromaticBulgeGridLfoClip {
                        lane: ChromaticBulgeGridLaneId::BulgeAmount,
                        shape: ChromaticBulgeGridLfoShape {
                            interpolation: LfoInterpolation::Linear,
                            points: vec![],
                        },
                        min: 0.0,
                        max: 1.0,
                        period_beats: 4.0,
                        phase_offset_beats: 0.0,
                        start_mode: LfoStartMode::Retrigger,
                    })),
                    authoring: None,
                    lanes: Default::default(),
                }],
                arrangement: vec![],
            }),
        };
        assert!(validate_visualizer_config("record", &config).is_err());
    }

    #[test]
    fn rejects_duplicate_clip_local_shape_phases() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: Default::default(),
            automation: None,
            timeline: Some(ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 4,
                beats_per_measure: 4,
                clips: vec![ChromaticBulgeGridClip {
                    id: "clip".to_string(),
                    name: "Clip".to_string(),
                    length_beats: 4.0,
                    color: [1.0, 1.0, 1.0],
                    source: Some(ChromaticBulgeGridClipSource::Lfo(ChromaticBulgeGridLfoClip {
                        lane: ChromaticBulgeGridLaneId::BulgeAmount,
                        shape: ChromaticBulgeGridLfoShape {
                            interpolation: LfoInterpolation::Linear,
                            points: vec![
                                LfoPoint {
                                    phase: 0.25,
                                    value: 0.0,
                                    curve_to_next: 0.0,
                                },
                                LfoPoint {
                                    phase: 0.25,
                                    value: 1.0,
                                    curve_to_next: 0.0,
                                },
                            ],
                        },
                        min: 0.0,
                        max: 1.0,
                        period_beats: 4.0,
                        phase_offset_beats: 0.0,
                        start_mode: LfoStartMode::Retrigger,
                    })),
                    authoring: None,
                    lanes: Default::default(),
                }],
                arrangement: vec![],
            }),
        };
        assert!(validate_visualizer_config("record", &config).is_err());
    }

    #[test]
    fn parse_visualizer_json_rejects_removed_shared_shape_schema() {
        let json = r#"{
            "mode": "chromatic_bulge_grid",
            "params": {},
            "lfo_library": {
                "shapes": []
            },
            "timeline": {
                "bpm": 120.0,
                "measures": 4,
                "beats_per_measure": 4,
                "clips": []
            }
        }"#;
        assert!(parse_visualizer_json(json).is_err());
    }

    #[test]
    fn normalized_for_export_emits_no_lfo_library() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: Default::default(),
            automation: None,
            timeline: Some(ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 4,
                beats_per_measure: 4,
                clips: vec![ChromaticBulgeGridClip {
                    id: "clip".to_string(),
                    name: "Clip".to_string(),
                    length_beats: 4.0,
                    color: [1.0, 1.0, 1.0],
                    source: Some(ChromaticBulgeGridClipSource::Lfo(ChromaticBulgeGridLfoClip {
                        lane: ChromaticBulgeGridLaneId::BulgeAmount,
                        shape: ChromaticBulgeGridLfoShape {
                            interpolation: LfoInterpolation::Linear,
                            points: vec![LfoPoint {
                                phase: 0.0,
                                value: 0.0,
                                curve_to_next: 0.0,
                            }],
                        },
                        min: 0.0,
                        max: 1.0,
                        period_beats: 4.0,
                        phase_offset_beats: 0.0,
                        start_mode: LfoStartMode::Retrigger,
                    })),
                    authoring: None,
                    lanes: Default::default(),
                }],
                arrangement: vec![],
            }),
        };
        let exported = serde_json::to_value(config.normalized_for_export()).unwrap();
        assert!(exported.get("lfo_library").is_none());
    }
}
