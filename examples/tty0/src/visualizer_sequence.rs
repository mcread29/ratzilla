use std::f32::consts::PI;

use crate::archive::{
    ChromaticBulgeGridAutomationLanes, ChromaticBulgeGridClipAuthoring, ChromaticBulgeGridLaneId,
    ChromaticBulgeGridShaderState, ClipTweenEase, ClipTweenStep, ClipTweenValue, ColorKeyframe,
    FloatKeyframe, InterpolationMode,
};

pub fn compile_authoring_lanes(
    base: ChromaticBulgeGridShaderState,
    clip_length_beats: f32,
    authoring: &ChromaticBulgeGridClipAuthoring,
) -> ChromaticBulgeGridAutomationLanes {
    let mut lanes = ChromaticBulgeGridAutomationLanes::default();

    for track in &authoring.tracks {
        if track.lane.is_color() {
            let mut cursor = 0.0f32;
            let mut current = base_color_for_lane(base, track.lane);
            let lane_keyframes = color_lane_mut(&mut lanes, track.lane);
            for step in &track.steps {
                let ClipTweenValue::Color(target) = step.to else {
                    continue;
                };
                cursor = compile_color_step(
                    lane_keyframes,
                    current,
                    target,
                    cursor,
                    step,
                    clip_length_beats,
                );
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
                cursor = compile_float_step(
                    lane_keyframes,
                    current,
                    target,
                    cursor,
                    step,
                    clip_length_beats,
                );
                current = target;
            }
        }
    }

    lanes.sort_all();
    lanes
}

fn compile_float_step(
    keyframes: &mut Vec<FloatKeyframe>,
    start_value: f32,
    end_value: f32,
    start_beat: f32,
    step: &ClipTweenStep,
    clip_length_beats: f32,
) -> f32 {
    let duration = step.duration_beats.max(0.0);
    let end_beat = (start_beat + duration).min(clip_length_beats.max(0.0));
    push_float_keyframe(
        keyframes,
        start_beat,
        start_value,
        interpolation_for_start(step.ease),
    );

    if duration <= 0.0001 || end_beat <= start_beat {
        push_float_keyframe(keyframes, end_beat, end_value, InterpolationMode::Hold);
        return end_beat;
    }

    if step.ease == ClipTweenEase::Hold {
        push_float_keyframe(keyframes, end_beat, end_value, InterpolationMode::Hold);
        return end_beat;
    }

    let segments = segments_for_ease(step.ease);
    for index in 1..=segments {
        let t = index as f32 / segments as f32;
        let beat = start_beat + (end_beat - start_beat) * t;
        let interpolation = if index == segments {
            InterpolationMode::Hold
        } else {
            InterpolationMode::Linear
        };
        push_float_keyframe(
            keyframes,
            beat,
            eased_float(start_value, end_value, step.ease, t),
            interpolation,
        );
    }

    end_beat
}

fn compile_color_step(
    keyframes: &mut Vec<ColorKeyframe>,
    start_value: [f32; 3],
    end_value: [f32; 3],
    start_beat: f32,
    step: &ClipTweenStep,
    clip_length_beats: f32,
) -> f32 {
    let duration = step.duration_beats.max(0.0);
    let end_beat = (start_beat + duration).min(clip_length_beats.max(0.0));
    push_color_keyframe(
        keyframes,
        start_beat,
        start_value,
        interpolation_for_start(step.ease),
    );

    if duration <= 0.0001 || end_beat <= start_beat {
        push_color_keyframe(keyframes, end_beat, end_value, InterpolationMode::Hold);
        return end_beat;
    }

    if step.ease == ClipTweenEase::Hold {
        push_color_keyframe(keyframes, end_beat, end_value, InterpolationMode::Hold);
        return end_beat;
    }

    let segments = segments_for_ease(step.ease);
    for index in 1..=segments {
        let t = index as f32 / segments as f32;
        let beat = start_beat + (end_beat - start_beat) * t;
        let interpolation = if index == segments {
            InterpolationMode::Hold
        } else {
            InterpolationMode::Linear
        };
        push_color_keyframe(
            keyframes,
            beat,
            eased_color(start_value, end_value, step.ease, t),
            interpolation,
        );
    }

    end_beat
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

fn eased_float(start: f32, end: f32, ease: ClipTweenEase, t: f32) -> f32 {
    lerp(start, end, ease_progress(ease, t))
}

fn eased_color(start: [f32; 3], end: [f32; 3], ease: ClipTweenEase, t: f32) -> [f32; 3] {
    let progress = ease_progress(ease, t);
    [
        lerp(start[0], end[0], progress),
        lerp(start[1], end[1], progress),
        lerp(start[2], end[2], progress),
    ]
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

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
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
        ChromaticBulgeGridLaneId::ColdColor | ChromaticBulgeGridLaneId::HotColor => {
            unreachable!("color lane requested as float")
        }
    }
}

fn base_color_for_lane(
    base: ChromaticBulgeGridShaderState,
    lane: ChromaticBulgeGridLaneId,
) -> [f32; 3] {
    match lane {
        ChromaticBulgeGridLaneId::ColdColor => base.cold_color,
        ChromaticBulgeGridLaneId::HotColor => base.hot_color,
        _ => unreachable!("float lane requested as color"),
    }
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

#[cfg(test)]
mod tests {
    use super::compile_authoring_lanes;
    use crate::archive::{
        ChromaticBulgeGridClipAuthoring, ChromaticBulgeGridLaneId, ChromaticBulgeGridShaderState,
        ClipParamTrack, ClipTweenEase, ClipTweenStep, ClipTweenValue,
    };

    #[test]
    fn sequential_float_steps_compile_to_keyframes() {
        let base = ChromaticBulgeGridShaderState::default();
        let authoring = ChromaticBulgeGridClipAuthoring {
            tracks: vec![ClipParamTrack {
                lane: ChromaticBulgeGridLaneId::CircleRadius,
                steps: vec![
                    ClipTweenStep {
                        to: ClipTweenValue::Float(0.8),
                        duration_beats: 0.125,
                        ease: ClipTweenEase::SineOut,
                    },
                    ClipTweenStep {
                        to: ClipTweenValue::Float(0.2),
                        duration_beats: 0.875,
                        ease: ClipTweenEase::SineIn,
                    },
                ],
            }],
        };

        let lanes = compile_authoring_lanes(base, 4.0, &authoring);
        assert!(!lanes.circle_radius.is_empty());
        assert!((lanes.circle_radius[0].beat - 0.0).abs() < 0.0001);
        assert!(
            lanes.circle_radius
                .iter()
                .any(|keyframe| (keyframe.beat - 1.0).abs() < 0.0001 && (keyframe.value - 0.2).abs() < 0.0001)
        );
    }

    #[test]
    fn nonlinear_ease_emits_intermediate_keyframes() {
        let base = ChromaticBulgeGridShaderState::default();
        let authoring = ChromaticBulgeGridClipAuthoring {
            tracks: vec![ClipParamTrack {
                lane: ChromaticBulgeGridLaneId::HotColor,
                steps: vec![ClipTweenStep {
                    to: ClipTweenValue::Color([0.9, 0.2, 0.1]),
                    duration_beats: 1.0,
                    ease: ClipTweenEase::SineInOut,
                }],
            }],
        };

        let lanes = compile_authoring_lanes(base, 4.0, &authoring);
        assert!(lanes.hot_color.len() > 2);
    }
}
