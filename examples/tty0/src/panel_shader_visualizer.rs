use std::{cell::RefCell, rc::Rc};

use crate::archive::{ChromaticBulgeGridShaderState, PlaybackClock, TrackVisualizerMode};
use glow::{self, HasContext, PixelUnpackData};
use ratzilla::{
    backend::hooks::{BackendKind, RenderHook, RenderHookContext, RenderHookHandle},
    error::Error,
    ratatui::layout::Rect,
};

const FULLSCREEN_VERTEX_SHADER: &str = r#"#version 300 es
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

const CHROMATIC_BULGE_GRID_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_motion_rate;
uniform float u_lattice_density;
uniform float u_circle_radius;
uniform float u_circle_falloff_start;
uniform float u_circle_falloff_end;
uniform float u_bulge_amount;
uniform float u_rim_guard;
uniform float u_rim_exponent;
uniform float u_rim_warp;
uniform float u_spacing_max_px;
uniform float u_spacing_min_px;
uniform float u_dot_size;
uniform float u_outer_dot_scale;
uniform float u_edge_softness;
uniform float u_chromatic_aberration;
uniform float u_scroll_base;
uniform float u_scroll_motion_scale;
uniform float u_scroll_motion_floor;
uniform float u_scroll_motion_ceiling;
uniform vec3 u_cold_color;
uniform vec3 u_hot_color;
uniform float u_color_cycle_rate;
uniform float u_inner_alpha;

out vec4 out_color;

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
  float spacing_px = mix(u_spacing_max_px, u_spacing_min_px, density);
  float base_radius_px = spacing_px * u_dot_size;
  float scroll_px = u_time * (
    u_scroll_base
    + u_scroll_motion_scale * clamp(
      u_motion_rate - u_scroll_motion_floor,
      0.0,
      u_scroll_motion_ceiling
    )
  );
  vec2 base_sample_px = frag_px;
  base_sample_px.x += scroll_px;

  vec2 sphere_offset = frag_px - center;
  float center_profile = falloff * hemisphere;
  float magnify = 1.0 - center_profile * u_bulge_amount;
  vec2 warped_screen_px = center + sphere_offset * magnify;
  vec2 rim_direction = sphere_normal.xy / max(sphere_normal.z, u_rim_guard);
  float rim_profile = falloff * pow(clamp(1.0 - sphere_normal.z, 0.0, 1.0), u_rim_exponent);
  float rim_warp = rim_profile * u_rim_warp;
  vec2 sphere_sample_px = warped_screen_px + rim_direction * rim_warp;
  sphere_sample_px.x += scroll_px;
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

  float color_phase = 0.5 + 0.5 * sin(u_time * u_color_cycle_rate);
  vec3 base_color = mix(u_cold_color, u_hot_color, color_phase);
  vec3 dot_color = mix(vec3(1.0), base_color, sphere_mix);

  float outer_alpha = mask_g;
  float inner_alpha = mask_g * u_inner_alpha;
  float alpha = mix(outer_alpha, inner_alpha, sphere_mix);
  vec3 color = vec3(mask_r, mask_g, mask_b) * dot_color;

  out_color = vec4(clamp(color, 0.0, 1.0), clamp(alpha, 0.0, 1.0));
}
"#;

const BLIT_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;

uniform sampler2D u_scene;

out vec4 out_color;

void main() {
  out_color = texture(u_scene, v_uv);
}
"#;

#[derive(Clone, Debug)]
pub struct PanelShaderRequest {
    pub area: Rect,
    pub mode: TrackVisualizerMode,
    pub record_id: String,
    pub playback: PlaybackClock,
    pub shader_state: ChromaticBulgeGridShaderState,
}

#[derive(Clone, Default)]
pub struct PanelShaderVisualizerLayer {
    shared: Rc<RefCell<SharedPanelShaderState>>,
}

#[derive(Default)]
struct SharedPanelShaderState {
    pending_request: Option<PanelShaderRequest>,
}

struct PanelShaderRenderHook {
    shared: Rc<RefCell<SharedPanelShaderState>>,
    runtime: PanelShaderRuntime,
}

#[derive(Default)]
struct PanelShaderRuntime {
    resources: Option<RuntimeResources>,
    panel_size: Option<(i32, i32)>,
    active_record_id: Option<String>,
    disabled: bool,
}

#[derive(Clone, Copy, Debug)]
struct PanelPixelRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug)]
struct UniformSet {
    resolution: (f32, f32),
    time: f32,
    motion_rate: f32,
    lattice_density: f32,
    circle_radius: f32,
    circle_falloff_start: f32,
    circle_falloff_end: f32,
    bulge_amount: f32,
    rim_guard: f32,
    rim_exponent: f32,
    rim_warp: f32,
    spacing_max_px: f32,
    spacing_min_px: f32,
    dot_size: f32,
    outer_dot_scale: f32,
    edge_softness: f32,
    chromatic_aberration: f32,
    scroll_base: f32,
    scroll_motion_scale: f32,
    scroll_motion_floor: f32,
    scroll_motion_ceiling: f32,
    cold_color: [f32; 3],
    hot_color: [f32; 3],
    color_cycle_rate: f32,
    inner_alpha: f32,
}

struct RuntimeResources {
    scene_program: glow::Program,
    blit_program: glow::Program,
    vao: glow::VertexArray,
    prev_texture: glow::Texture,
    next_texture: glow::Texture,
    prev_framebuffer: glow::Framebuffer,
    next_framebuffer: glow::Framebuffer,
    scene_uniforms: SceneUniformLocations,
    blit_uniforms: BlitUniformLocations,
}

struct SceneUniformLocations {
    resolution: glow::UniformLocation,
    time: glow::UniformLocation,
    motion_rate: glow::UniformLocation,
    lattice_density: glow::UniformLocation,
    circle_radius: glow::UniformLocation,
    circle_falloff_start: glow::UniformLocation,
    circle_falloff_end: glow::UniformLocation,
    bulge_amount: glow::UniformLocation,
    rim_guard: glow::UniformLocation,
    rim_exponent: glow::UniformLocation,
    rim_warp: glow::UniformLocation,
    spacing_max_px: glow::UniformLocation,
    spacing_min_px: glow::UniformLocation,
    dot_size: glow::UniformLocation,
    outer_dot_scale: glow::UniformLocation,
    edge_softness: glow::UniformLocation,
    chromatic_aberration: glow::UniformLocation,
    scroll_base: glow::UniformLocation,
    scroll_motion_scale: glow::UniformLocation,
    scroll_motion_floor: glow::UniformLocation,
    scroll_motion_ceiling: glow::UniformLocation,
    cold_color: glow::UniformLocation,
    hot_color: glow::UniformLocation,
    color_cycle_rate: glow::UniformLocation,
    inner_alpha: glow::UniformLocation,
}

struct BlitUniformLocations {
    scene: glow::UniformLocation,
}

#[derive(Clone, Copy, Debug)]
enum ShaderType {
    Vertex,
    Fragment,
}

impl ShaderType {
    fn as_gl_shader_type(self) -> u32 {
        match self {
            Self::Vertex => glow::VERTEX_SHADER,
            Self::Fragment => glow::FRAGMENT_SHADER,
        }
    }
}

impl PanelShaderVisualizerLayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render_hook(&self) -> RenderHookHandle {
        RenderHookHandle::new(PanelShaderRenderHook {
            shared: Rc::clone(&self.shared),
            runtime: PanelShaderRuntime::default(),
        })
    }

    pub fn queue(&self, request: PanelShaderRequest) {
        self.shared.borrow_mut().pending_request = Some(request);
    }

    pub fn begin_frame(&self) {
        self.shared.borrow_mut().pending_request = None;
    }
}

impl RenderHook for PanelShaderRenderHook {
    fn pre_render(&mut self, _context: &RenderHookContext<'_>) -> Result<(), Error> {
        Ok(())
    }

    fn post_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        if context.backend() != BackendKind::WebGl2 || self.runtime.disabled {
            return Ok(());
        }

        let Some(request) = self.shared.borrow_mut().pending_request.take() else {
            return Ok(());
        };
        let Some(gl) = context.webgl_context() else {
            return Ok(());
        };

        if self.runtime.render(gl, context, &request).is_err() {
            self.runtime.disabled = true;
        }

        Ok(())
    }
}

impl PanelShaderRuntime {
    fn render(
        &mut self,
        gl: &glow::Context,
        context: &RenderHookContext<'_>,
        request: &PanelShaderRequest,
    ) -> Result<(), Error> {
        if !matches!(request.mode, TrackVisualizerMode::ChromaticBulgeGrid) {
            return Ok(());
        }

        let Some(cell_size) = context.cell_size() else {
            return Ok(());
        };
        let (_, canvas_height) = context.canvas_size();
        let Some(panel_rect) = PanelPixelRect::from_rect(request.area, cell_size, canvas_height)
        else {
            return Ok(());
        };

        self.ensure_initialized(gl)?;

        let size_changed = self.panel_size != Some((panel_rect.width, panel_rect.height));
        if size_changed {
            self.resize_feedback(gl, panel_rect.width, panel_rect.height)?;
            self.panel_size = Some((panel_rect.width, panel_rect.height));
        }

        let record_changed = self.active_record_id.as_deref() != Some(request.record_id.as_str());
        if record_changed {
            self.active_record_id = Some(request.record_id.clone());
        }

        if size_changed || record_changed {
            self.clear_feedback(gl)?;
        }

        let uniforms =
            UniformSet::from_request(request, panel_rect.width, panel_rect.height, cell_size);
        let target_framebuffer = unsafe { gl.get_parameter_framebuffer(glow::FRAMEBUFFER_BINDING) };
        let scissor_enabled = unsafe { gl.is_enabled(glow::SCISSOR_TEST) };
        let mut scissor_box = [0; 4];
        let mut viewport = [0; 4];
        unsafe {
            gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor_box);
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
        }

        self.render_scene_pass(gl, &uniforms)?;
        self.blit_panel(gl, target_framebuffer.clone(), panel_rect)?;
        self.swap_feedback();

        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, target_framebuffer);
            gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
            if scissor_enabled {
                gl.enable(glow::SCISSOR_TEST);
                gl.scissor(
                    scissor_box[0],
                    scissor_box[1],
                    scissor_box[2],
                    scissor_box[3],
                );
            } else {
                gl.disable(glow::SCISSOR_TEST);
            }
        }

        Ok(())
    }

    fn ensure_initialized(&mut self, gl: &glow::Context) -> Result<(), Error> {
        if self.resources.is_some() {
            return Ok(());
        }

        self.resources = Some(RuntimeResources::new(gl)?);
        Ok(())
    }

    fn resize_feedback(
        &mut self,
        gl: &glow::Context,
        width: i32,
        height: i32,
    ) -> Result<(), Error> {
        if width <= 0 || height <= 0 {
            return Ok(());
        }

        let resources = self.resources_mut()?;
        unsafe {
            gl.delete_texture(resources.prev_texture);
            gl.delete_texture(resources.next_texture);
            gl.delete_framebuffer(resources.prev_framebuffer);
            gl.delete_framebuffer(resources.next_framebuffer);
        }

        let (prev_texture, prev_framebuffer) = create_render_target(gl, width, height)?;
        let (next_texture, next_framebuffer) = create_render_target(gl, width, height)?;
        resources.prev_texture = prev_texture;
        resources.prev_framebuffer = prev_framebuffer;
        resources.next_texture = next_texture;
        resources.next_framebuffer = next_framebuffer;
        Ok(())
    }

    fn clear_feedback(&mut self, gl: &glow::Context) -> Result<(), Error> {
        let (width, height) = self.panel_size.unwrap_or((1, 1));
        let resources = self.resources_mut()?;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.prev_framebuffer));
            gl.viewport(0, 0, width, height);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.next_framebuffer));
            gl.viewport(0, 0, width, height);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        Ok(())
    }

    fn render_scene_pass(
        &mut self,
        gl: &glow::Context,
        uniforms: &UniformSet,
    ) -> Result<(), Error> {
        let resources = self.resources_mut()?;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.next_framebuffer));
            gl.viewport(
                0,
                0,
                uniforms.resolution.0 as i32,
                uniforms.resolution.1 as i32,
            );
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.use_program(Some(resources.scene_program));
            gl.bind_vertex_array(Some(resources.vao));
            gl.uniform_2_f32(
                Some(&resources.scene_uniforms.resolution),
                uniforms.resolution.0,
                uniforms.resolution.1,
            );
            gl.uniform_1_f32(Some(&resources.scene_uniforms.time), uniforms.time);
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.motion_rate),
                uniforms.motion_rate,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.lattice_density),
                uniforms.lattice_density,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.circle_radius),
                uniforms.circle_radius,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.circle_falloff_start),
                uniforms.circle_falloff_start,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.circle_falloff_end),
                uniforms.circle_falloff_end,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.bulge_amount),
                uniforms.bulge_amount,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.rim_guard),
                uniforms.rim_guard,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.rim_exponent),
                uniforms.rim_exponent,
            );
            gl.uniform_1_f32(Some(&resources.scene_uniforms.rim_warp), uniforms.rim_warp);
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.spacing_max_px),
                uniforms.spacing_max_px,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.spacing_min_px),
                uniforms.spacing_min_px,
            );
            gl.uniform_1_f32(Some(&resources.scene_uniforms.dot_size), uniforms.dot_size);
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.outer_dot_scale),
                uniforms.outer_dot_scale,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.edge_softness),
                uniforms.edge_softness,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.chromatic_aberration),
                uniforms.chromatic_aberration,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.scroll_base),
                uniforms.scroll_base,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.scroll_motion_scale),
                uniforms.scroll_motion_scale,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.scroll_motion_floor),
                uniforms.scroll_motion_floor,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.scroll_motion_ceiling),
                uniforms.scroll_motion_ceiling,
            );
            gl.uniform_3_f32(
                Some(&resources.scene_uniforms.cold_color),
                uniforms.cold_color[0],
                uniforms.cold_color[1],
                uniforms.cold_color[2],
            );
            gl.uniform_3_f32(
                Some(&resources.scene_uniforms.hot_color),
                uniforms.hot_color[0],
                uniforms.hot_color[1],
                uniforms.hot_color[2],
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.color_cycle_rate),
                uniforms.color_cycle_rate,
            );
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.inner_alpha),
                uniforms.inner_alpha,
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
        Ok(())
    }

    fn blit_panel(
        &mut self,
        gl: &glow::Context,
        target_framebuffer: Option<glow::Framebuffer>,
        panel_rect: PanelPixelRect,
    ) -> Result<(), Error> {
        let resources = self.resources_mut()?;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, target_framebuffer);
            gl.viewport(
                panel_rect.x,
                panel_rect.y,
                panel_rect.width,
                panel_rect.height,
            );
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(
                panel_rect.x,
                panel_rect.y,
                panel_rect.width,
                panel_rect.height,
            );
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            gl.use_program(Some(resources.blit_program));
            gl.bind_vertex_array(Some(resources.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(resources.next_texture));
            gl.uniform_1_i32(Some(&resources.blit_uniforms.scene), 0);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.disable(glow::BLEND);
        }
        Ok(())
    }

    fn swap_feedback(&mut self) {
        let Some(resources) = &mut self.resources else {
            return;
        };

        std::mem::swap(&mut resources.prev_texture, &mut resources.next_texture);
        std::mem::swap(
            &mut resources.prev_framebuffer,
            &mut resources.next_framebuffer,
        );
    }

    fn resources_mut(&mut self) -> Result<&mut RuntimeResources, Error> {
        self.resources
            .as_mut()
            .ok_or_else(|| gl_error("panel shader runtime resources are unavailable"))
    }
}

impl PanelPixelRect {
    fn from_rect(area: Rect, cell_size: (i32, i32), canvas_height: i32) -> Option<Self> {
        if area.width == 0 || area.height == 0 || cell_size.0 <= 0 || cell_size.1 <= 0 {
            return None;
        }

        let width = area.width as i32 * cell_size.0;
        let height = area.height as i32 * cell_size.1;
        let x = area.x as i32 * cell_size.0;
        let y_top = area.y as i32 * cell_size.1;
        let y = canvas_height - (y_top + height);
        if width <= 0 || height <= 0 {
            return None;
        }

        Some(Self {
            x,
            y,
            width,
            height,
        })
    }
}

impl UniformSet {
    fn from_request(
        request: &PanelShaderRequest,
        width: i32,
        height: i32,
        cell_size: (i32, i32),
    ) -> Self {
        let shader_state = request.shader_state.clamp();
        let spacing_max_px = shader_state.spacing_max_px.clamp(2.0, 64.0);
        let spacing_min_px = shader_state.spacing_min_px.clamp(2.0, spacing_max_px);
        let lattice_density = resolve_lattice_density(
            shader_state.lattice_density,
            cell_size.1,
            spacing_max_px,
            spacing_min_px,
        );

        Self {
            resolution: (width as f32, height as f32),
            time: request.playback.visual_time_secs.max(0.0),
            motion_rate: shader_state.motion_rate.clamp(0.2, 3.0),
            lattice_density,
            circle_radius: shader_state.circle_radius.clamp(0.05, 0.48),
            circle_falloff_start: shader_state.circle_falloff_start.clamp(0.0, 0.98),
            circle_falloff_end: shader_state
                .circle_falloff_end
                .max(shader_state.circle_falloff_start + 0.01)
                .clamp(0.02, 1.2),
            bulge_amount: shader_state.bulge_amount.clamp(0.0, 1.5),
            rim_guard: shader_state.rim_guard.clamp(0.05, 1.0),
            rim_exponent: shader_state.rim_exponent.clamp(0.2, 4.0),
            rim_warp: shader_state.rim_warp.clamp(0.0, 1.0),
            spacing_max_px,
            spacing_min_px,
            dot_size: shader_state.dot_size.clamp(0.02, 0.5),
            outer_dot_scale: shader_state.outer_dot_scale.clamp(0.02, 1.0),
            edge_softness: shader_state.edge_softness.clamp(0.1, 8.0),
            chromatic_aberration: shader_state.chromatic_aberration.clamp(0.0, 4.0),
            scroll_base: shader_state.scroll_base.clamp(0.0, 256.0),
            scroll_motion_scale: shader_state.scroll_motion_scale.clamp(0.0, 256.0),
            scroll_motion_floor: shader_state.scroll_motion_floor.clamp(0.0, 3.0),
            scroll_motion_ceiling: shader_state.scroll_motion_ceiling.clamp(0.0, 4.0),
            cold_color: shader_state.cold_color.map(|value| value.clamp(0.0, 1.0)),
            hot_color: shader_state.hot_color.map(|value| value.clamp(0.0, 1.0)),
            color_cycle_rate: shader_state.color_cycle_rate.clamp(0.0, 4.0),
            inner_alpha: shader_state.inner_alpha.clamp(0.0, 1.0),
        }
    }
}

fn resolve_lattice_density(
    lattice_density: f32,
    cell_height_px: i32,
    spacing_max_px: f32,
    spacing_min_px: f32,
) -> f32 {
    let density = lattice_density.clamp(2.0, 12.0);
    if (density - 6.0).abs() > f32::EPSILON {
        return density;
    }

    let cell_height = cell_height_px.max(1) as f32;
    let spacing_span = (spacing_max_px - spacing_min_px).abs();
    if spacing_span <= f32::EPSILON {
        return density;
    }

    let normalized =
        ((spacing_max_px - cell_height) / (spacing_max_px - spacing_min_px)).clamp(0.0, 1.0);
    (2.0 + normalized * 10.0).clamp(2.0, 12.0)
}

impl RuntimeResources {
    fn new(gl: &glow::Context) -> Result<Self, Error> {
        let scene_program = create_program(
            gl,
            FULLSCREEN_VERTEX_SHADER,
            CHROMATIC_BULGE_GRID_FRAGMENT_SHADER,
        )?;
        let blit_program = create_program(gl, FULLSCREEN_VERTEX_SHADER, BLIT_FRAGMENT_SHADER)?;
        let vao = unsafe {
            gl.create_vertex_array()
                .map_err(Error::UnableToRetrieveElementById)?
        };
        let (prev_texture, prev_framebuffer) = create_render_target(gl, 1, 1)?;
        let (next_texture, next_framebuffer) = create_render_target(gl, 1, 1)?;

        Ok(Self {
            scene_uniforms: SceneUniformLocations {
                resolution: uniform_location(gl, scene_program, "u_resolution")?,
                time: uniform_location(gl, scene_program, "u_time")?,
                motion_rate: uniform_location(gl, scene_program, "u_motion_rate")?,
                lattice_density: uniform_location(gl, scene_program, "u_lattice_density")?,
                circle_radius: uniform_location(gl, scene_program, "u_circle_radius")?,
                circle_falloff_start: uniform_location(
                    gl,
                    scene_program,
                    "u_circle_falloff_start",
                )?,
                circle_falloff_end: uniform_location(gl, scene_program, "u_circle_falloff_end")?,
                bulge_amount: uniform_location(gl, scene_program, "u_bulge_amount")?,
                rim_guard: uniform_location(gl, scene_program, "u_rim_guard")?,
                rim_exponent: uniform_location(gl, scene_program, "u_rim_exponent")?,
                rim_warp: uniform_location(gl, scene_program, "u_rim_warp")?,
                spacing_max_px: uniform_location(gl, scene_program, "u_spacing_max_px")?,
                spacing_min_px: uniform_location(gl, scene_program, "u_spacing_min_px")?,
                dot_size: uniform_location(gl, scene_program, "u_dot_size")?,
                outer_dot_scale: uniform_location(gl, scene_program, "u_outer_dot_scale")?,
                edge_softness: uniform_location(gl, scene_program, "u_edge_softness")?,
                chromatic_aberration: uniform_location(
                    gl,
                    scene_program,
                    "u_chromatic_aberration",
                )?,
                scroll_base: uniform_location(gl, scene_program, "u_scroll_base")?,
                scroll_motion_scale: uniform_location(gl, scene_program, "u_scroll_motion_scale")?,
                scroll_motion_floor: uniform_location(gl, scene_program, "u_scroll_motion_floor")?,
                scroll_motion_ceiling: uniform_location(
                    gl,
                    scene_program,
                    "u_scroll_motion_ceiling",
                )?,
                cold_color: uniform_location(gl, scene_program, "u_cold_color")?,
                hot_color: uniform_location(gl, scene_program, "u_hot_color")?,
                color_cycle_rate: uniform_location(gl, scene_program, "u_color_cycle_rate")?,
                inner_alpha: uniform_location(gl, scene_program, "u_inner_alpha")?,
            },
            blit_uniforms: BlitUniformLocations {
                scene: uniform_location(gl, blit_program, "u_scene")?,
            },
            scene_program,
            blit_program,
            vao,
            prev_texture,
            next_texture,
            prev_framebuffer,
            next_framebuffer,
        })
    }
}

fn create_render_target(
    gl: &glow::Context,
    width: i32,
    height: i32,
) -> Result<(glow::Texture, glow::Framebuffer), Error> {
    let texture = unsafe {
        gl.create_texture()
            .map_err(Error::UnableToRetrieveElementById)?
    };
    let framebuffer = unsafe {
        gl.create_framebuffer()
            .map_err(Error::UnableToRetrieveElementById)?
    };

    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            width,
            height,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(None),
        );
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(framebuffer);
            gl.delete_texture(texture);
            return Err(gl_error("panel shader framebuffer is incomplete"));
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }

    Ok((texture, framebuffer))
}

fn create_shader(
    gl: &glow::Context,
    shader_type: ShaderType,
    shader_source: &str,
) -> Result<glow::Shader, Error> {
    let shader = unsafe {
        gl.create_shader(shader_type.as_gl_shader_type())
            .map_err(Error::UnableToRetrieveElementById)?
    };

    unsafe {
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);
    }

    if unsafe { !gl.get_shader_compile_status(shader) } {
        let info = unsafe { gl.get_shader_info_log(shader) };
        unsafe {
            gl.delete_shader(shader);
        }
        return Err(gl_error(&info));
    }

    Ok(shader)
}

fn create_program(
    gl: &glow::Context,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<glow::Program, Error> {
    let program = unsafe {
        gl.create_program()
            .map_err(Error::UnableToRetrieveElementById)?
    };
    let vertex_shader = create_shader(gl, ShaderType::Vertex, vertex_source)?;
    let fragment_shader = create_shader(gl, ShaderType::Fragment, fragment_source)?;

    unsafe {
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
    }

    if unsafe { !gl.get_program_link_status(program) } {
        let info = unsafe { gl.get_program_info_log(program) };
        unsafe {
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);
            gl.delete_program(program);
        }
        return Err(gl_error(&info));
    }

    unsafe {
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
    }

    Ok(program)
}

fn uniform_location(
    gl: &glow::Context,
    program: glow::Program,
    name: &str,
) -> Result<glow::UniformLocation, Error> {
    unsafe { gl.get_uniform_location(program, name) }
        .ok_or_else(|| gl_error(&format!("missing panel shader uniform {name}")))
}

fn gl_error(message: &str) -> Error {
    Error::UnableToRetrieveElementById(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::{PanelShaderRequest, PanelShaderVisualizerLayer, UniformSet};
    use crate::archive::{ChromaticBulgeGridShaderState, PlaybackClock, TrackVisualizerMode};
    use ratzilla::ratatui::layout::Rect;

    fn request() -> PanelShaderRequest {
        PanelShaderRequest {
            area: Rect::new(2, 3, 20, 10),
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            record_id: "0x07E2BIG".to_string(),
            playback: PlaybackClock {
                current_time_secs: 1.25,
                visual_time_secs: 1.25,
                duration_secs: Some(99.0),
                is_playing: true,
                timeline_preview: false,
            },
            shader_state: ChromaticBulgeGridShaderState {
                motion_rate: 8.0,
                lattice_density: 99.0,
                circle_radius: 0.31,
                chromatic_aberration: 0.63,
                hot_color: [1.2, 0.4, 0.1],
                ..ChromaticBulgeGridShaderState::default()
            },
        }
    }

    #[test]
    fn begin_frame_clears_queued_request() {
        let layer = PanelShaderVisualizerLayer::new();
        layer.queue(request());

        assert!(layer.shared.borrow().pending_request.is_some());

        layer.begin_frame();

        assert!(layer.shared.borrow().pending_request.is_none());
    }

    #[test]
    fn queue_preserves_area_and_mode() {
        let layer = PanelShaderVisualizerLayer::new();
        let request = request();
        layer.queue(request);

        let pending = layer.shared.borrow();
        let pending = pending
            .pending_request
            .as_ref()
            .expect("queued request should exist");

        assert_eq!(pending.area, Rect::new(2, 3, 20, 10));
        assert_eq!(pending.mode, TrackVisualizerMode::ChromaticBulgeGrid);
    }

    #[test]
    fn uniform_set_uses_resolved_shader_state() {
        let uniforms = UniformSet::from_request(&request(), 320, 180, (9, 18));

        assert_eq!(uniforms.resolution, (320.0, 180.0));
        assert_eq!(uniforms.time, 1.25);
        assert_eq!(uniforms.motion_rate, 3.0);
        assert_eq!(uniforms.lattice_density, 12.0);
        assert_eq!(uniforms.circle_radius, 0.31);
        assert_eq!(uniforms.chromatic_aberration, 0.63);
        assert_eq!(uniforms.hot_color, [1.0, 0.4, 0.1]);
    }

    #[test]
    fn uniform_set_uses_request_playback_clock_for_time() {
        let mut request = request();
        request.playback.visual_time_secs = 4.5;
        request.shader_state.motion_rate = 0.05;
        request.shader_state.lattice_density = 1.0;
        request.shader_state.circle_radius = 0.12;
        request.shader_state.chromatic_aberration = 0.05;
        request.shader_state.hot_color = [0.2, 0.3, 0.4];

        let uniforms = UniformSet::from_request(&request, 320, 180, (9, 18));

        assert_eq!(uniforms.time, 4.5);
        assert_eq!(uniforms.motion_rate, 0.2);
        assert_eq!(uniforms.lattice_density, 2.0);
        assert_eq!(uniforms.circle_radius, 0.12);
        assert_eq!(uniforms.chromatic_aberration, 0.05);
        assert_eq!(uniforms.hot_color, [0.2, 0.3, 0.4]);
    }

    #[test]
    fn default_lattice_density_aligns_to_terminal_rows() {
        let mut request = request();
        request.shader_state.motion_rate = 1.0;
        request.shader_state.lattice_density = 6.0;

        let uniforms = UniformSet::from_request(&request, 320, 180, (9, 18));

        assert_eq!(uniforms.lattice_density, 6.0);
    }
}
