use crate::archive::{
    ChromaticBulgeGridAutomationLanes, ChromaticBulgeGridClip, ChromaticBulgeGridClipAuthoring,
    ChromaticBulgeGridLaneId, ChromaticBulgeGridShaderState, ClipLaneAuthoringMode, ClipShape,
    ClipShapeAuthoring, ClipShapeEase, ColorKeyframe, FloatKeyframe, InterpolationMode,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequenceEase {
    Hold,
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    OutCubic,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatLaneTarget {
    MotionRate,
    LatticeDensity,
    CircleRadius,
    CircleFalloffStart,
    CircleFalloffEnd,
    BulgeAmount,
    RimGuard,
    RimExponent,
    RimWarp,
    SpacingMaxPx,
    SpacingMinPx,
    DotSize,
    OuterDotScale,
    EdgeSoftness,
    ChromaticAberration,
    ScrollBase,
    ScrollMotionScale,
    ScrollMotionFloor,
    ScrollMotionCeiling,
    ColorCycleRate,
    InnerAlpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorLaneTarget {
    ColdColor,
    HotColor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FloatTween {
    lane: FloatLaneTarget,
    from: Option<f32>,
    to: f32,
    start_beat: f32,
    duration_beats: f32,
    ease: SequenceEase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ColorTween {
    lane: ColorLaneTarget,
    from: Option<[f32; 3]>,
    to: [f32; 3],
    start_beat: f32,
    duration_beats: f32,
    ease: SequenceEase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SequenceStep {
    Float(FloatTween),
    Color(ColorTween),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sequence {
    cursor_beat: f32,
    anchor_beat: f32,
    steps: Vec<SequenceStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequenceTemplate {
    Pulse1Beat,
    Breath1Bar,
    Sweep1Bar,
    Stutter2Beat,
    Collapse1Bar,
}

impl Sequence {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_float(
        mut self,
        lane: FloatLaneTarget,
        to: f32,
        duration_beats: f32,
        ease: SequenceEase,
    ) -> Self {
        let start = self.cursor_beat;
        self.anchor_beat = start;
        self.cursor_beat = (self.cursor_beat + duration_beats.max(0.0)).max(self.cursor_beat);
        self.steps.push(SequenceStep::Float(FloatTween {
            lane,
            from: None,
            to,
            start_beat: start,
            duration_beats,
            ease,
        }));
        self
    }

    pub fn join_float(
        mut self,
        lane: FloatLaneTarget,
        to: f32,
        duration_beats: f32,
        ease: SequenceEase,
    ) -> Self {
        let start = if self.steps.is_empty() {
            self.cursor_beat
        } else {
            self.anchor_beat
        };
        self.cursor_beat = self.cursor_beat.max(start + duration_beats.max(0.0));
        self.steps.push(SequenceStep::Float(FloatTween {
            lane,
            from: None,
            to,
            start_beat: start,
            duration_beats,
            ease,
        }));
        self
    }

    pub fn insert_float(
        mut self,
        at_beat: f32,
        lane: FloatLaneTarget,
        to: f32,
        duration_beats: f32,
        ease: SequenceEase,
    ) -> Self {
        let start = at_beat.max(0.0);
        self.anchor_beat = start;
        self.cursor_beat = self.cursor_beat.max(start + duration_beats.max(0.0));
        self.steps.push(SequenceStep::Float(FloatTween {
            lane,
            from: None,
            to,
            start_beat: start,
            duration_beats,
            ease,
        }));
        self
    }

    #[allow(dead_code)]
    pub fn append_color(
        mut self,
        lane: ColorLaneTarget,
        to: [f32; 3],
        duration_beats: f32,
        ease: SequenceEase,
    ) -> Self {
        let start = self.cursor_beat;
        self.anchor_beat = start;
        self.cursor_beat = (self.cursor_beat + duration_beats.max(0.0)).max(self.cursor_beat);
        self.steps.push(SequenceStep::Color(ColorTween {
            lane,
            from: None,
            to,
            start_beat: start,
            duration_beats,
            ease,
        }));
        self
    }

    #[allow(dead_code)]
    pub fn join_color(
        mut self,
        lane: ColorLaneTarget,
        to: [f32; 3],
        duration_beats: f32,
        ease: SequenceEase,
    ) -> Self {
        let start = if self.steps.is_empty() {
            self.cursor_beat
        } else {
            self.anchor_beat
        };
        self.cursor_beat = self.cursor_beat.max(start + duration_beats.max(0.0));
        self.steps.push(SequenceStep::Color(ColorTween {
            lane,
            from: None,
            to,
            start_beat: start,
            duration_beats,
            ease,
        }));
        self
    }

    pub fn insert_color(
        mut self,
        at_beat: f32,
        lane: ColorLaneTarget,
        to: [f32; 3],
        duration_beats: f32,
        ease: SequenceEase,
    ) -> Self {
        let start = at_beat.max(0.0);
        self.anchor_beat = start;
        self.cursor_beat = self.cursor_beat.max(start + duration_beats.max(0.0));
        self.steps.push(SequenceStep::Color(ColorTween {
            lane,
            from: None,
            to,
            start_beat: start,
            duration_beats,
            ease,
        }));
        self
    }

    #[allow(dead_code)]
    pub fn interval(mut self, beats: f32) -> Self {
        self.cursor_beat = (self.cursor_beat + beats).max(0.0);
        self.anchor_beat = self.cursor_beat;
        self
    }

    pub fn length_beats(&self) -> f32 {
        self.steps
            .iter()
            .map(|step| match step {
                SequenceStep::Float(tween) => tween.start_beat + tween.duration_beats.max(0.0),
                SequenceStep::Color(tween) => tween.start_beat + tween.duration_beats.max(0.0),
            })
            .fold(0.0, f32::max)
    }
}

impl SequenceTemplate {
    pub const ALL: [Self; 5] = [
        Self::Pulse1Beat,
        Self::Breath1Bar,
        Self::Sweep1Bar,
        Self::Stutter2Beat,
        Self::Collapse1Bar,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pulse1Beat => "pulse_1b",
            Self::Breath1Bar => "breath_1bar",
            Self::Sweep1Bar => "sweep_1bar",
            Self::Stutter2Beat => "stutter_2b",
            Self::Collapse1Bar => "collapse_1bar",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Pulse1Beat => "Pulse",
            Self::Breath1Bar => "Breath",
            Self::Sweep1Bar => "Sweep",
            Self::Stutter2Beat => "Stutter",
            Self::Collapse1Bar => "Collapse",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|candidate| *candidate == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn build(self, beats_per_measure: u32, base: ChromaticBulgeGridShaderState) -> Sequence {
        let bar = beats_per_measure.max(1) as f32;
        match self {
            Self::Pulse1Beat => build_pulse_sequence(base),
            Self::Breath1Bar => build_breath_sequence(base, bar),
            Self::Sweep1Bar => build_sweep_sequence(base, bar),
            Self::Stutter2Beat => build_stutter_sequence(base),
            Self::Collapse1Bar => build_collapse_sequence(base, bar),
        }
    }
}

pub fn compile_sequence_to_clip(
    id: &str,
    name: &str,
    color: [f32; 3],
    base: ChromaticBulgeGridShaderState,
    sequence: &Sequence,
) -> ChromaticBulgeGridClip {
    let mut lanes = ChromaticBulgeGridAutomationLanes::default();

    for step in &sequence.steps {
        match *step {
            SequenceStep::Float(tween) => compile_float_tween(&mut lanes, base, tween),
            SequenceStep::Color(tween) => compile_color_tween(&mut lanes, base, tween),
        }
    }

    sort_compiled_lanes(&mut lanes);

    ChromaticBulgeGridClip {
        id: id.to_string(),
        name: name.to_string(),
        length_beats: sequence.length_beats().max(0.25),
        color,
        authoring: None,
        lanes,
    }
}

pub fn compile_authoring_lanes(
    base: ChromaticBulgeGridShaderState,
    clip_length_beats: f32,
    authoring: &ChromaticBulgeGridClipAuthoring,
    existing_lanes: &ChromaticBulgeGridAutomationLanes,
) -> ChromaticBulgeGridAutomationLanes {
    let mut lanes = ChromaticBulgeGridAutomationLanes::default();

    for row in &authoring.lanes {
        match &row.mode {
            ClipLaneAuthoringMode::Shape(shape) => {
                compile_shape_lane(&mut lanes, base, clip_length_beats, row.lane, shape);
            }
            ClipLaneAuthoringMode::Custom => copy_custom_lane(&mut lanes, existing_lanes, row.lane),
        }
    }

    sort_compiled_lanes(&mut lanes);
    lanes
}

fn compile_shape_lane(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    base: ChromaticBulgeGridShaderState,
    clip_length_beats: f32,
    lane: ChromaticBulgeGridLaneId,
    shape: &ClipShapeAuthoring,
) {
    let offset = shape.offset_beats.clamp(0.0, clip_length_beats.max(0.0));
    let width = shape
        .width_beats
        .clamp(0.0001, (clip_length_beats - offset).max(0.0001));

    if lane.is_color() {
        let Some(target_color) = shape.target_color else {
            return;
        };
        let base_color = base_color_for_lane(base, lane);
        let sequence = build_color_shape_sequence(base_color, target_color, shape.shape, offset, width, shape.ease);
        compile_color_steps_into_lane(lanes, base, lane, &sequence);
    } else {
        let Some(depth) = shape.depth else {
            return;
        };
        let base_value = base_float_for_lane(base, lane);
        let sequence = build_float_shape_sequence(base_value, depth, shape.shape, offset, width, shape.ease);
        compile_float_steps_into_lane(lanes, base, lane, &sequence);
    }
}

fn build_float_shape_sequence(
    base: f32,
    depth: f32,
    shape: ClipShape,
    offset: f32,
    width: f32,
    ease: ClipShapeEase,
) -> Sequence {
    let ease = map_shape_ease(ease);
    match shape {
        ClipShape::Pulse | ClipShape::Flash => Sequence::new()
            .insert_float(offset, FloatLaneTarget::CircleRadius, base + depth, width * 0.5, SequenceEase::OutQuad)
            .insert_float(offset + width * 0.5, FloatLaneTarget::CircleRadius, base, width * 0.5, ease),
        ClipShape::Rise => Sequence::new().insert_float(
            offset,
            FloatLaneTarget::CircleRadius,
            base + depth,
            width,
            ease,
        ),
        ClipShape::Fall => Sequence::new()
            .insert_float(offset, FloatLaneTarget::CircleRadius, base + depth, 0.0, SequenceEase::Hold)
            .insert_float(offset, FloatLaneTarget::CircleRadius, base, width, ease),
        ClipShape::Triangle => Sequence::new()
            .insert_float(offset, FloatLaneTarget::CircleRadius, base + depth, width * 0.5, ease)
            .insert_float(offset + width * 0.5, FloatLaneTarget::CircleRadius, base, width * 0.5, ease),
        ClipShape::Stutter => {
            let mut sequence = Sequence::new();
            let slice = (width / 4.0).max(0.0001);
            for index in 0..4 {
                let start = offset + slice * index as f32;
                let target = if index % 2 == 0 { base + depth } else { base };
                sequence = sequence.insert_float(start, FloatLaneTarget::CircleRadius, target, slice, ease);
            }
            sequence
        }
        ClipShape::Fade => Sequence::new().insert_float(
            offset,
            FloatLaneTarget::CircleRadius,
            base + depth,
            width,
            ease,
        ),
    }
}

fn build_color_shape_sequence(
    base: [f32; 3],
    target: [f32; 3],
    shape: ClipShape,
    offset: f32,
    width: f32,
    ease: ClipShapeEase,
) -> Sequence {
    let ease = map_shape_ease(ease);
    match shape {
        ClipShape::Flash | ClipShape::Pulse => Sequence::new()
            .insert_color(offset, ColorLaneTarget::HotColor, target, width * 0.5, SequenceEase::OutQuad)
            .insert_color(offset + width * 0.5, ColorLaneTarget::HotColor, base, width * 0.5, ease),
        ClipShape::Fade | ClipShape::Rise => Sequence::new().insert_color(
            offset,
            ColorLaneTarget::HotColor,
            target,
            width,
            ease,
        ),
        ClipShape::Fall => Sequence::new()
            .insert_color(offset, ColorLaneTarget::HotColor, target, 0.0, SequenceEase::Hold)
            .insert_color(offset, ColorLaneTarget::HotColor, base, width, ease),
        ClipShape::Triangle => Sequence::new()
            .insert_color(offset, ColorLaneTarget::HotColor, target, width * 0.5, ease)
            .insert_color(offset + width * 0.5, ColorLaneTarget::HotColor, base, width * 0.5, ease),
        ClipShape::Stutter => {
            let mut sequence = Sequence::new();
            let slice = (width / 4.0).max(0.0001);
            for index in 0..4 {
                let start = offset + slice * index as f32;
                let value = if index % 2 == 0 { target } else { base };
                sequence = sequence.insert_color(start, ColorLaneTarget::HotColor, value, slice, ease);
            }
            sequence
        }
    }
}

fn compile_float_steps_into_lane(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    base: ChromaticBulgeGridShaderState,
    lane: ChromaticBulgeGridLaneId,
    sequence: &Sequence,
) {
    let target = map_float_lane(lane);
    for step in &sequence.steps {
        if let SequenceStep::Float(mut tween) = *step {
            tween.lane = target;
            compile_float_tween(lanes, base, tween);
        }
    }
}

fn compile_color_steps_into_lane(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    base: ChromaticBulgeGridShaderState,
    lane: ChromaticBulgeGridLaneId,
    sequence: &Sequence,
) {
    let target = map_color_lane(lane);
    for step in &sequence.steps {
        if let SequenceStep::Color(mut tween) = *step {
            tween.lane = target;
            compile_color_tween(lanes, base, tween);
        }
    }
}

fn copy_custom_lane(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    existing: &ChromaticBulgeGridAutomationLanes,
    lane: ChromaticBulgeGridLaneId,
) {
    match lane {
        ChromaticBulgeGridLaneId::MotionRate => lanes.motion_rate = existing.motion_rate.clone(),
        ChromaticBulgeGridLaneId::LatticeDensity => lanes.lattice_density = existing.lattice_density.clone(),
        ChromaticBulgeGridLaneId::CircleRadius => lanes.circle_radius = existing.circle_radius.clone(),
        ChromaticBulgeGridLaneId::CircleFalloffStart => lanes.circle_falloff_start = existing.circle_falloff_start.clone(),
        ChromaticBulgeGridLaneId::CircleFalloffEnd => lanes.circle_falloff_end = existing.circle_falloff_end.clone(),
        ChromaticBulgeGridLaneId::BulgeAmount => lanes.bulge_amount = existing.bulge_amount.clone(),
        ChromaticBulgeGridLaneId::RimGuard => lanes.rim_guard = existing.rim_guard.clone(),
        ChromaticBulgeGridLaneId::RimExponent => lanes.rim_exponent = existing.rim_exponent.clone(),
        ChromaticBulgeGridLaneId::RimWarp => lanes.rim_warp = existing.rim_warp.clone(),
        ChromaticBulgeGridLaneId::SpacingMaxPx => lanes.spacing_max_px = existing.spacing_max_px.clone(),
        ChromaticBulgeGridLaneId::SpacingMinPx => lanes.spacing_min_px = existing.spacing_min_px.clone(),
        ChromaticBulgeGridLaneId::DotSize => lanes.dot_size = existing.dot_size.clone(),
        ChromaticBulgeGridLaneId::OuterDotScale => lanes.outer_dot_scale = existing.outer_dot_scale.clone(),
        ChromaticBulgeGridLaneId::EdgeSoftness => lanes.edge_softness = existing.edge_softness.clone(),
        ChromaticBulgeGridLaneId::ChromaticAberration => lanes.chromatic_aberration = existing.chromatic_aberration.clone(),
        ChromaticBulgeGridLaneId::ScrollBase => lanes.scroll_base = existing.scroll_base.clone(),
        ChromaticBulgeGridLaneId::ScrollMotionScale => lanes.scroll_motion_scale = existing.scroll_motion_scale.clone(),
        ChromaticBulgeGridLaneId::ScrollMotionFloor => lanes.scroll_motion_floor = existing.scroll_motion_floor.clone(),
        ChromaticBulgeGridLaneId::ScrollMotionCeiling => lanes.scroll_motion_ceiling = existing.scroll_motion_ceiling.clone(),
        ChromaticBulgeGridLaneId::ColdColor => lanes.cold_color = existing.cold_color.clone(),
        ChromaticBulgeGridLaneId::HotColor => lanes.hot_color = existing.hot_color.clone(),
        ChromaticBulgeGridLaneId::ColorCycleRate => lanes.color_cycle_rate = existing.color_cycle_rate.clone(),
        ChromaticBulgeGridLaneId::InnerAlpha => lanes.inner_alpha = existing.inner_alpha.clone(),
    }
}

fn map_shape_ease(ease: ClipShapeEase) -> SequenceEase {
    match ease {
        ClipShapeEase::Hold => SequenceEase::Hold,
        ClipShapeEase::Linear => SequenceEase::Linear,
        ClipShapeEase::EaseOut => SequenceEase::OutQuad,
        ClipShapeEase::EaseInOut => SequenceEase::InOutQuad,
    }
}

fn map_float_lane(lane: ChromaticBulgeGridLaneId) -> FloatLaneTarget {
    match lane {
        ChromaticBulgeGridLaneId::MotionRate => FloatLaneTarget::MotionRate,
        ChromaticBulgeGridLaneId::LatticeDensity => FloatLaneTarget::LatticeDensity,
        ChromaticBulgeGridLaneId::CircleRadius => FloatLaneTarget::CircleRadius,
        ChromaticBulgeGridLaneId::CircleFalloffStart => FloatLaneTarget::CircleFalloffStart,
        ChromaticBulgeGridLaneId::CircleFalloffEnd => FloatLaneTarget::CircleFalloffEnd,
        ChromaticBulgeGridLaneId::BulgeAmount => FloatLaneTarget::BulgeAmount,
        ChromaticBulgeGridLaneId::RimGuard => FloatLaneTarget::RimGuard,
        ChromaticBulgeGridLaneId::RimExponent => FloatLaneTarget::RimExponent,
        ChromaticBulgeGridLaneId::RimWarp => FloatLaneTarget::RimWarp,
        ChromaticBulgeGridLaneId::SpacingMaxPx => FloatLaneTarget::SpacingMaxPx,
        ChromaticBulgeGridLaneId::SpacingMinPx => FloatLaneTarget::SpacingMinPx,
        ChromaticBulgeGridLaneId::DotSize => FloatLaneTarget::DotSize,
        ChromaticBulgeGridLaneId::OuterDotScale => FloatLaneTarget::OuterDotScale,
        ChromaticBulgeGridLaneId::EdgeSoftness => FloatLaneTarget::EdgeSoftness,
        ChromaticBulgeGridLaneId::ChromaticAberration => FloatLaneTarget::ChromaticAberration,
        ChromaticBulgeGridLaneId::ScrollBase => FloatLaneTarget::ScrollBase,
        ChromaticBulgeGridLaneId::ScrollMotionScale => FloatLaneTarget::ScrollMotionScale,
        ChromaticBulgeGridLaneId::ScrollMotionFloor => FloatLaneTarget::ScrollMotionFloor,
        ChromaticBulgeGridLaneId::ScrollMotionCeiling => FloatLaneTarget::ScrollMotionCeiling,
        ChromaticBulgeGridLaneId::ColorCycleRate => FloatLaneTarget::ColorCycleRate,
        ChromaticBulgeGridLaneId::InnerAlpha => FloatLaneTarget::InnerAlpha,
        ChromaticBulgeGridLaneId::ColdColor | ChromaticBulgeGridLaneId::HotColor => {
            unreachable!("color lane requested as float")
        }
    }
}

fn map_color_lane(lane: ChromaticBulgeGridLaneId) -> ColorLaneTarget {
    match lane {
        ChromaticBulgeGridLaneId::ColdColor => ColorLaneTarget::ColdColor,
        ChromaticBulgeGridLaneId::HotColor => ColorLaneTarget::HotColor,
        _ => unreachable!("float lane requested as color"),
    }
}

fn base_float_for_lane(base: ChromaticBulgeGridShaderState, lane: ChromaticBulgeGridLaneId) -> f32 {
    match map_float_lane(lane) {
        FloatLaneTarget::MotionRate => base.motion_rate,
        FloatLaneTarget::LatticeDensity => base.lattice_density,
        FloatLaneTarget::CircleRadius => base.circle_radius,
        FloatLaneTarget::CircleFalloffStart => base.circle_falloff_start,
        FloatLaneTarget::CircleFalloffEnd => base.circle_falloff_end,
        FloatLaneTarget::BulgeAmount => base.bulge_amount,
        FloatLaneTarget::RimGuard => base.rim_guard,
        FloatLaneTarget::RimExponent => base.rim_exponent,
        FloatLaneTarget::RimWarp => base.rim_warp,
        FloatLaneTarget::SpacingMaxPx => base.spacing_max_px,
        FloatLaneTarget::SpacingMinPx => base.spacing_min_px,
        FloatLaneTarget::DotSize => base.dot_size,
        FloatLaneTarget::OuterDotScale => base.outer_dot_scale,
        FloatLaneTarget::EdgeSoftness => base.edge_softness,
        FloatLaneTarget::ChromaticAberration => base.chromatic_aberration,
        FloatLaneTarget::ScrollBase => base.scroll_base,
        FloatLaneTarget::ScrollMotionScale => base.scroll_motion_scale,
        FloatLaneTarget::ScrollMotionFloor => base.scroll_motion_floor,
        FloatLaneTarget::ScrollMotionCeiling => base.scroll_motion_ceiling,
        FloatLaneTarget::ColorCycleRate => base.color_cycle_rate,
        FloatLaneTarget::InnerAlpha => base.inner_alpha,
    }
}

fn base_color_for_lane(base: ChromaticBulgeGridShaderState, lane: ChromaticBulgeGridLaneId) -> [f32; 3] {
    match map_color_lane(lane) {
        ColorLaneTarget::ColdColor => base.cold_color,
        ColorLaneTarget::HotColor => base.hot_color,
    }
}

fn build_pulse_sequence(base: ChromaticBulgeGridShaderState) -> Sequence {
    Sequence::new()
        .append_float(
            FloatLaneTarget::CircleRadius,
            (base.circle_radius + 0.28).clamp(0.02, 0.95),
            0.18,
            SequenceEase::OutCubic,
        )
        .join_float(
            FloatLaneTarget::BulgeAmount,
            (base.bulge_amount + 0.62).clamp(0.0, 1.5),
            0.18,
            SequenceEase::OutQuad,
        )
        .join_float(
            FloatLaneTarget::ChromaticAberration,
            (base.chromatic_aberration + 6.0).clamp(0.0, 24.0),
            0.12,
            SequenceEase::OutQuad,
        )
        .append_float(
            FloatLaneTarget::CircleRadius,
            base.circle_radius,
            0.82,
            SequenceEase::InOutQuad,
        )
        .join_float(
            FloatLaneTarget::BulgeAmount,
            base.bulge_amount,
            0.82,
            SequenceEase::InOutQuad,
        )
        .join_float(
            FloatLaneTarget::ChromaticAberration,
            base.chromatic_aberration,
            0.50,
            SequenceEase::OutQuad,
        )
        .insert_color(
            0.0,
            ColorLaneTarget::HotColor,
            brighten(base.hot_color, 0.12),
            0.25,
            SequenceEase::OutQuad,
        )
        .insert_color(
            0.25,
            ColorLaneTarget::HotColor,
            base.hot_color,
            0.75,
            SequenceEase::InOutQuad,
        )
}

fn build_breath_sequence(base: ChromaticBulgeGridShaderState, bar: f32) -> Sequence {
    let open = bar * 0.5;
    let close = bar - open;
    Sequence::new()
        .append_float(
            FloatLaneTarget::CircleRadius,
            (base.circle_radius + 0.18).clamp(0.02, 0.95),
            open,
            SequenceEase::OutQuad,
        )
        .join_float(
            FloatLaneTarget::BulgeAmount,
            (base.bulge_amount + 0.25).clamp(0.0, 1.5),
            open,
            SequenceEase::OutQuad,
        )
        .join_float(
            FloatLaneTarget::InnerAlpha,
            (base.inner_alpha + 0.14).clamp(0.0, 1.0),
            open,
            SequenceEase::OutQuad,
        )
        .append_float(
            FloatLaneTarget::CircleRadius,
            base.circle_radius,
            close,
            SequenceEase::InOutQuad,
        )
        .join_float(
            FloatLaneTarget::BulgeAmount,
            base.bulge_amount,
            close,
            SequenceEase::InOutQuad,
        )
        .join_float(
            FloatLaneTarget::InnerAlpha,
            base.inner_alpha,
            close,
            SequenceEase::InOutQuad,
        )
        .insert_color(
            0.0,
            ColorLaneTarget::ColdColor,
            brighten(base.cold_color, 0.10),
            bar * 0.5,
            SequenceEase::Linear,
        )
        .insert_color(
            bar * 0.5,
            ColorLaneTarget::ColdColor,
            base.cold_color,
            bar * 0.5,
            SequenceEase::Linear,
        )
}

fn build_sweep_sequence(base: ChromaticBulgeGridShaderState, bar: f32) -> Sequence {
    Sequence::new()
        .append_float(
            FloatLaneTarget::ScrollBase,
            (base.scroll_base + 40.0).clamp(0.0, 200.0),
            bar,
            SequenceEase::Linear,
        )
        .join_float(
            FloatLaneTarget::MotionRate,
            (base.motion_rate + 0.35).clamp(0.2, 3.0),
            bar * 0.35,
            SequenceEase::OutQuad,
        )
        .insert_float(
            bar * 0.35,
            FloatLaneTarget::MotionRate,
            base.motion_rate,
            bar * 0.65,
            SequenceEase::InOutQuad,
        )
        .join_float(
            FloatLaneTarget::ChromaticAberration,
            (base.chromatic_aberration + 5.0).clamp(0.0, 24.0),
            bar * 0.45,
            SequenceEase::OutQuad,
        )
        .insert_float(
            bar * 0.45,
            FloatLaneTarget::ChromaticAberration,
            base.chromatic_aberration,
            bar * 0.55,
            SequenceEase::InOutQuad,
        )
}

fn build_stutter_sequence(base: ChromaticBulgeGridShaderState) -> Sequence {
    let mut sequence = Sequence::new();
    for _ in 0..4 {
        sequence = sequence
            .append_float(
                FloatLaneTarget::CircleRadius,
                (base.circle_radius + 0.20).clamp(0.02, 0.95),
                0.08,
                SequenceEase::OutQuad,
            )
            .join_float(
                FloatLaneTarget::BulgeAmount,
                (base.bulge_amount + 0.30).clamp(0.0, 1.5),
                0.08,
                SequenceEase::OutQuad,
            )
            .append_float(
                FloatLaneTarget::CircleRadius,
                base.circle_radius,
                0.17,
                SequenceEase::InQuad,
            )
            .join_float(
                FloatLaneTarget::BulgeAmount,
                base.bulge_amount,
                0.17,
                SequenceEase::InQuad,
            );
    }
    sequence
        .insert_float(
            0.0,
            FloatLaneTarget::ChromaticAberration,
            (base.chromatic_aberration + 3.5).clamp(0.0, 24.0),
            0.16,
            SequenceEase::OutQuad,
        )
        .insert_float(
            0.16,
            FloatLaneTarget::ChromaticAberration,
            base.chromatic_aberration,
            0.34,
            SequenceEase::Linear,
        )
}

fn build_collapse_sequence(base: ChromaticBulgeGridShaderState, bar: f32) -> Sequence {
    Sequence::new()
        .append_float(
            FloatLaneTarget::RimWarp,
            (base.rim_warp + 18.0).clamp(0.0, 64.0),
            bar * 0.7,
            SequenceEase::InOutQuad,
        )
        .join_float(
            FloatLaneTarget::ChromaticAberration,
            (base.chromatic_aberration + 10.0).clamp(0.0, 24.0),
            bar * 0.7,
            SequenceEase::OutCubic,
        )
        .join_float(
            FloatLaneTarget::EdgeSoftness,
            (base.edge_softness + 1.2).clamp(0.1, 8.0),
            bar * 0.7,
            SequenceEase::OutQuad,
        )
        .append_float(
            FloatLaneTarget::RimWarp,
            base.rim_warp,
            bar * 0.3,
            SequenceEase::InQuad,
        )
        .join_float(
            FloatLaneTarget::ChromaticAberration,
            base.chromatic_aberration,
            bar * 0.3,
            SequenceEase::InQuad,
        )
        .join_float(
            FloatLaneTarget::EdgeSoftness,
            base.edge_softness,
            bar * 0.3,
            SequenceEase::InQuad,
        )
        .insert_color(
            0.0,
            ColorLaneTarget::HotColor,
            brighten(base.hot_color, 0.18),
            bar * 0.4,
            SequenceEase::OutQuad,
        )
        .insert_color(
            bar * 0.4,
            ColorLaneTarget::HotColor,
            base.hot_color,
            bar * 0.6,
            SequenceEase::InOutQuad,
        )
}

fn compile_float_tween(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    base: ChromaticBulgeGridShaderState,
    tween: FloatTween,
) {
    let target_lane = float_lane_mut(lanes, tween.lane);
    let start = tween.start_beat.max(0.0);
    let duration = tween.duration_beats.max(0.0);
    let end = start + duration;
    let start_value = tween
        .from
        .unwrap_or_else(|| sample_float_lane(target_lane, start, base_float_value(base, tween.lane)));
    let end_value = tween.to;
    push_float_keyframe(target_lane, start, start_value, interpolation_for_start(tween.ease));
    if duration <= 0.0001 {
        push_float_keyframe(target_lane, end, end_value, InterpolationMode::Hold);
        return;
    }

    if tween.ease == SequenceEase::Hold {
        push_float_keyframe(target_lane, end, end_value, InterpolationMode::Hold);
        return;
    }

    let segments = segments_for_ease(tween.ease);
    for index in 1..=segments {
        let t = index as f32 / segments as f32;
        let beat = start + duration * t;
        let value = ease_float(start_value, end_value, tween.ease, t);
        let interpolation = if index == segments {
            InterpolationMode::Hold
        } else {
            InterpolationMode::Linear
        };
        push_float_keyframe(target_lane, beat, value, interpolation);
    }
}

fn compile_color_tween(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    base: ChromaticBulgeGridShaderState,
    tween: ColorTween,
) {
    let target_lane = color_lane_mut(lanes, tween.lane);
    let start = tween.start_beat.max(0.0);
    let duration = tween.duration_beats.max(0.0);
    let end = start + duration;
    let start_value = tween
        .from
        .unwrap_or_else(|| sample_color_lane(target_lane, start, base_color_value(base, tween.lane)));
    let end_value = tween.to;
    push_color_keyframe(target_lane, start, start_value, interpolation_for_start(tween.ease));
    if duration <= 0.0001 {
        push_color_keyframe(target_lane, end, end_value, InterpolationMode::Hold);
        return;
    }

    if tween.ease == SequenceEase::Hold {
        push_color_keyframe(target_lane, end, end_value, InterpolationMode::Hold);
        return;
    }

    let segments = segments_for_ease(tween.ease);
    for index in 1..=segments {
        let t = index as f32 / segments as f32;
        let beat = start + duration * t;
        let value = ease_color(start_value, end_value, tween.ease, t);
        let interpolation = if index == segments {
            InterpolationMode::Hold
        } else {
            InterpolationMode::Linear
        };
        push_color_keyframe(target_lane, beat, value, interpolation);
    }
}

fn interpolation_for_start(ease: SequenceEase) -> InterpolationMode {
    match ease {
        SequenceEase::Hold => InterpolationMode::Hold,
        SequenceEase::Linear
        | SequenceEase::InQuad
        | SequenceEase::OutQuad
        | SequenceEase::InOutQuad
        | SequenceEase::OutCubic => InterpolationMode::Linear,
    }
}

fn segments_for_ease(ease: SequenceEase) -> usize {
    match ease {
        SequenceEase::Hold | SequenceEase::Linear => 1,
        SequenceEase::InQuad | SequenceEase::OutQuad => 4,
        SequenceEase::InOutQuad | SequenceEase::OutCubic => 6,
    }
}

fn ease_float(start: f32, end: f32, ease: SequenceEase, t: f32) -> f32 {
    lerp(start, end, ease_progress(ease, t))
}

fn ease_color(start: [f32; 3], end: [f32; 3], ease: SequenceEase, t: f32) -> [f32; 3] {
    let progress = ease_progress(ease, t);
    [
        lerp(start[0], end[0], progress),
        lerp(start[1], end[1], progress),
        lerp(start[2], end[2], progress),
    ]
}

fn ease_progress(ease: SequenceEase, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        SequenceEase::Hold => {
            if t >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
        SequenceEase::Linear => t,
        SequenceEase::InQuad => t * t,
        SequenceEase::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        SequenceEase::InOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        SequenceEase::OutCubic => 1.0 - (1.0 - t).powi(3),
    }
}

fn brighten(color: [f32; 3], amount: f32) -> [f32; 3] {
    [
        (color[0] + amount).clamp(0.0, 1.0),
        (color[1] + amount).clamp(0.0, 1.0),
        (color[2] + amount).clamp(0.0, 1.0),
    ]
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn float_lane_mut(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    lane: FloatLaneTarget,
) -> &mut Vec<FloatKeyframe> {
    match lane {
        FloatLaneTarget::MotionRate => &mut lanes.motion_rate,
        FloatLaneTarget::LatticeDensity => &mut lanes.lattice_density,
        FloatLaneTarget::CircleRadius => &mut lanes.circle_radius,
        FloatLaneTarget::CircleFalloffStart => &mut lanes.circle_falloff_start,
        FloatLaneTarget::CircleFalloffEnd => &mut lanes.circle_falloff_end,
        FloatLaneTarget::BulgeAmount => &mut lanes.bulge_amount,
        FloatLaneTarget::RimGuard => &mut lanes.rim_guard,
        FloatLaneTarget::RimExponent => &mut lanes.rim_exponent,
        FloatLaneTarget::RimWarp => &mut lanes.rim_warp,
        FloatLaneTarget::SpacingMaxPx => &mut lanes.spacing_max_px,
        FloatLaneTarget::SpacingMinPx => &mut lanes.spacing_min_px,
        FloatLaneTarget::DotSize => &mut lanes.dot_size,
        FloatLaneTarget::OuterDotScale => &mut lanes.outer_dot_scale,
        FloatLaneTarget::EdgeSoftness => &mut lanes.edge_softness,
        FloatLaneTarget::ChromaticAberration => &mut lanes.chromatic_aberration,
        FloatLaneTarget::ScrollBase => &mut lanes.scroll_base,
        FloatLaneTarget::ScrollMotionScale => &mut lanes.scroll_motion_scale,
        FloatLaneTarget::ScrollMotionFloor => &mut lanes.scroll_motion_floor,
        FloatLaneTarget::ScrollMotionCeiling => &mut lanes.scroll_motion_ceiling,
        FloatLaneTarget::ColorCycleRate => &mut lanes.color_cycle_rate,
        FloatLaneTarget::InnerAlpha => &mut lanes.inner_alpha,
    }
}

fn color_lane_mut(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    lane: ColorLaneTarget,
) -> &mut Vec<ColorKeyframe> {
    match lane {
        ColorLaneTarget::ColdColor => &mut lanes.cold_color,
        ColorLaneTarget::HotColor => &mut lanes.hot_color,
    }
}

fn base_float_value(base: ChromaticBulgeGridShaderState, lane: FloatLaneTarget) -> f32 {
    match lane {
        FloatLaneTarget::MotionRate => base.motion_rate,
        FloatLaneTarget::LatticeDensity => base.lattice_density,
        FloatLaneTarget::CircleRadius => base.circle_radius,
        FloatLaneTarget::CircleFalloffStart => base.circle_falloff_start,
        FloatLaneTarget::CircleFalloffEnd => base.circle_falloff_end,
        FloatLaneTarget::BulgeAmount => base.bulge_amount,
        FloatLaneTarget::RimGuard => base.rim_guard,
        FloatLaneTarget::RimExponent => base.rim_exponent,
        FloatLaneTarget::RimWarp => base.rim_warp,
        FloatLaneTarget::SpacingMaxPx => base.spacing_max_px,
        FloatLaneTarget::SpacingMinPx => base.spacing_min_px,
        FloatLaneTarget::DotSize => base.dot_size,
        FloatLaneTarget::OuterDotScale => base.outer_dot_scale,
        FloatLaneTarget::EdgeSoftness => base.edge_softness,
        FloatLaneTarget::ChromaticAberration => base.chromatic_aberration,
        FloatLaneTarget::ScrollBase => base.scroll_base,
        FloatLaneTarget::ScrollMotionScale => base.scroll_motion_scale,
        FloatLaneTarget::ScrollMotionFloor => base.scroll_motion_floor,
        FloatLaneTarget::ScrollMotionCeiling => base.scroll_motion_ceiling,
        FloatLaneTarget::ColorCycleRate => base.color_cycle_rate,
        FloatLaneTarget::InnerAlpha => base.inner_alpha,
    }
}

fn base_color_value(base: ChromaticBulgeGridShaderState, lane: ColorLaneTarget) -> [f32; 3] {
    match lane {
        ColorLaneTarget::ColdColor => base.cold_color,
        ColorLaneTarget::HotColor => base.hot_color,
    }
}

fn sample_float_lane(keyframes: &[FloatKeyframe], beat: f32, base: f32) -> f32 {
    if keyframes.is_empty() {
        return base;
    }
    if beat < keyframes[0].beat {
        return base;
    }
    for window in keyframes.windows(2) {
        let left = window[0];
        let right = window[1];
        if beat < right.beat {
            return match left.interpolation {
                InterpolationMode::Hold => left.value,
                InterpolationMode::Linear => {
                    let span = (right.beat - left.beat).max(0.0001);
                    let t = ((beat - left.beat) / span).clamp(0.0, 1.0);
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
    if beat < keyframes[0].beat {
        return base;
    }
    for window in keyframes.windows(2) {
        let left = window[0];
        let right = window[1];
        if beat < right.beat {
            return match left.interpolation {
                InterpolationMode::Hold => left.value,
                InterpolationMode::Linear => {
                    let span = (right.beat - left.beat).max(0.0001);
                    let t = ((beat - left.beat) / span).clamp(0.0, 1.0);
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

fn push_float_keyframe(
    keyframes: &mut Vec<FloatKeyframe>,
    beat: f32,
    value: f32,
    interpolation: InterpolationMode,
) {
    if let Some(existing) = keyframes
        .iter_mut()
        .find(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
    {
        existing.value = value;
        existing.interpolation = interpolation;
        return;
    }
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
    if let Some(existing) = keyframes
        .iter_mut()
        .find(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
    {
        existing.value = value;
        existing.interpolation = interpolation;
        return;
    }
    keyframes.push(ColorKeyframe {
        beat,
        value,
        interpolation,
    });
}

fn sort_compiled_lanes(lanes: &mut ChromaticBulgeGridAutomationLanes) {
    lanes.motion_rate.sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.lattice_density
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.circle_radius
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.circle_falloff_start
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.circle_falloff_end
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.bulge_amount
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.rim_guard
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.rim_exponent
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.rim_warp
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.spacing_max_px
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.spacing_min_px
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.dot_size
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.outer_dot_scale
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.edge_softness
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.chromatic_aberration
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.scroll_base
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.scroll_motion_scale
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.scroll_motion_floor
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.scroll_motion_ceiling
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.cold_color
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.hot_color
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.color_cycle_rate
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
    lanes.inner_alpha
        .sort_by(|left, right| left.beat.total_cmp(&right.beat));
}

#[cfg(test)]
mod tests {
    use super::{compile_sequence_to_clip, SequenceTemplate};
    use crate::archive::ChromaticBulgeGridShaderState;

    #[test]
    fn pulse_template_compiles_authored_clip() {
        let base = ChromaticBulgeGridShaderState::default();
        let sequence = SequenceTemplate::Pulse1Beat.build(4, base);
        let clip = compile_sequence_to_clip("pulse", "Pulse", [0.8, 0.2, 0.2], base, &sequence);
        assert!(clip.length_beats >= 1.0);
        assert!(!clip.lanes.circle_radius.is_empty());
        assert!(!clip.lanes.bulge_amount.is_empty());
    }

    #[test]
    fn nonlinear_ease_emits_intermediate_keyframes() {
        let base = ChromaticBulgeGridShaderState::default();
        let sequence = SequenceTemplate::Collapse1Bar.build(4, base);
        let clip = compile_sequence_to_clip("collapse", "Collapse", [0.8, 0.2, 0.2], base, &sequence);
        assert!(clip.lanes.rim_warp.len() > 2);
    }
}
