use std::{cell::RefCell, rc::Rc};

use crate::{
    archive::TrackVisualizerParams,
    track_visualizer::AudioAnalysisSnapshot,
};
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

const CONTAINMENT_SCENE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;

in vec2 v_uv;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_energy;
uniform float u_bass;
uniform float u_mid;
uniform float u_treble;
uniform float u_peak;
uniform float u_progress;
uniform float u_motion_rate;
uniform float u_ring_count;
uniform float u_lattice_density;
uniform sampler2D u_prev_frame;

out vec4 out_color;

mat2 rot2(float a) {
  float s = sin(a);
  float c = cos(a);
  return mat2(c, -s, s, c);
}

float hash12(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

vec2 aspect_uv(vec2 uv) {
  vec2 p = uv * 2.0 - 1.0;
  p.x *= u_resolution.x / max(u_resolution.y, 1.0);
  return p;
}

vec2 to_tex_uv(vec2 p) {
  p.x /= u_resolution.x / max(u_resolution.y, 1.0);
  return p * 0.5 + 0.5;
}

void main() {
  vec2 base = aspect_uv(v_uv);
  float radius = length(base);

  float ring_count = max(u_ring_count, 1.0);
  float pulse = u_bass * 0.08 * sin(u_time * (2.8 + u_motion_rate * 1.7) + radius * 16.0);
  float ring_coord = radius + pulse;
  float ring_lines = abs(fract(ring_coord * (ring_count * 2.2 + 1.0)) - 0.5);
  float ring_width = 0.02 + (8.0 - ring_count) * 0.0015 + u_peak * 0.01;
  float rings = 1.0 - smoothstep(ring_width, ring_width + 0.018, ring_lines);
  rings *= smoothstep(1.0, 0.12, radius);

  float shock = 1.0 - smoothstep(0.01, 0.035, abs(radius - (0.26 + u_bass * 0.12 + u_peak * 0.06)));
  shock *= 0.35 + 0.65 * u_peak;

  vec2 prev_uv = to_tex_uv(base * (1.0 - (0.008 + 0.016 * u_bass)));
  prev_uv = clamp(prev_uv, vec2(0.001), vec2(0.999));
  vec3 prev = texture(u_prev_frame, prev_uv).rgb;

  float warning = smoothstep(0.55, 1.0, u_progress);
  vec3 bg = vec3(0.0, 0.0, 0.0);
  vec3 ring_color = mix(vec3(0.86, 0.58, 0.16), vec3(0.95, 0.34, 0.16), warning * 0.45 + u_peak * 0.2);

  vec3 current = bg;
  current += ring_color * rings * (0.18 + u_bass * 0.82);
  current += ring_color * shock * 0.45;

  float feedback = clamp(0.16 + u_energy * 0.24, 0.16, 0.42);
  vec3 color = mix(current, max(current, prev * 0.93), feedback);

  float edge = clamp(rings, 0.0, 1.0);
  float split = edge * u_treble * (0.012 + 0.01 * u_progress);
  vec2 split_px = vec2(split / max(u_resolution.x, 1.0), 0.0);
  vec3 split_sample;
  split_sample.r = texture(u_prev_frame, clamp(prev_uv + split_px, vec2(0.001), vec2(0.999))).r;
  split_sample.g = texture(u_prev_frame, prev_uv).g;
  split_sample.b = texture(u_prev_frame, clamp(prev_uv - split_px, vec2(0.001), vec2(0.999))).b;
  color = max(color, split_sample * edge * (0.15 + u_treble * 0.22));

  float flash = smoothstep(0.55, 1.0, u_peak);
  vec3 flash_color = mix(vec3(0.2, 0.9, 0.9), vec3(1.0, 0.24, 0.16), 0.65 + 0.35 * warning);
  color = mix(color, max(color, flash_color), flash * 0.18);
  color += flash_color * shock * flash * 0.1;

  out_color = vec4(clamp(color, 0.0, 1.0), 1.0);
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
    pub record_id: String,
    pub analysis: AudioAnalysisSnapshot,
    pub viewer_tick: u64,
    pub params: TrackVisualizerParams,
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
    runtime: ContainmentLatticeRuntime,
}

#[derive(Default)]
struct ContainmentLatticeRuntime {
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
    energy: f32,
    bass: f32,
    mid: f32,
    treble: f32,
    peak: f32,
    progress: f32,
    motion_rate: f32,
    ring_count: f32,
    lattice_density: f32,
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
    energy: glow::UniformLocation,
    bass: glow::UniformLocation,
    mid: glow::UniformLocation,
    treble: glow::UniformLocation,
    peak: glow::UniformLocation,
    progress: glow::UniformLocation,
    motion_rate: glow::UniformLocation,
    ring_count: glow::UniformLocation,
    lattice_density: glow::UniformLocation,
    prev_frame: glow::UniformLocation,
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
            runtime: ContainmentLatticeRuntime::default(),
        })
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

impl ContainmentLatticeRuntime {
    fn render(
        &mut self,
        gl: &glow::Context,
        context: &RenderHookContext<'_>,
        request: &PanelShaderRequest,
    ) -> Result<(), Error> {
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

        let uniforms = UniformSet::from_request(request, panel_rect.width, panel_rect.height);
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
                gl.scissor(scissor_box[0], scissor_box[1], scissor_box[2], scissor_box[3]);
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
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.next_framebuffer));
            gl.viewport(0, 0, width, height);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        Ok(())
    }

    fn render_scene_pass(&mut self, gl: &glow::Context, uniforms: &UniformSet) -> Result<(), Error> {
        let resources = self.resources_mut()?;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.next_framebuffer));
            gl.viewport(0, 0, uniforms.resolution.0 as i32, uniforms.resolution.1 as i32);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.use_program(Some(resources.scene_program));
            gl.bind_vertex_array(Some(resources.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(resources.prev_texture));
            gl.uniform_1_i32(Some(&resources.scene_uniforms.prev_frame), 0);
            gl.uniform_2_f32(
                Some(&resources.scene_uniforms.resolution),
                uniforms.resolution.0,
                uniforms.resolution.1,
            );
            gl.uniform_1_f32(Some(&resources.scene_uniforms.time), uniforms.time);
            gl.uniform_1_f32(Some(&resources.scene_uniforms.energy), uniforms.energy);
            gl.uniform_1_f32(Some(&resources.scene_uniforms.bass), uniforms.bass);
            gl.uniform_1_f32(Some(&resources.scene_uniforms.mid), uniforms.mid);
            gl.uniform_1_f32(Some(&resources.scene_uniforms.treble), uniforms.treble);
            gl.uniform_1_f32(Some(&resources.scene_uniforms.peak), uniforms.peak);
            gl.uniform_1_f32(Some(&resources.scene_uniforms.progress), uniforms.progress);
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.motion_rate),
                uniforms.motion_rate,
            );
            gl.uniform_1_f32(Some(&resources.scene_uniforms.ring_count), uniforms.ring_count);
            gl.uniform_1_f32(
                Some(&resources.scene_uniforms.lattice_density),
                uniforms.lattice_density,
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.bind_texture(glow::TEXTURE_2D, None);
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
            gl.viewport(panel_rect.x, panel_rect.y, panel_rect.width, panel_rect.height);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(panel_rect.x, panel_rect.y, panel_rect.width, panel_rect.height);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);

            gl.use_program(Some(resources.blit_program));
            gl.bind_vertex_array(Some(resources.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(resources.next_texture));
            gl.uniform_1_i32(Some(&resources.blit_uniforms.scene), 0);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
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
            .ok_or_else(|| gl_error("containment runtime resources are unavailable"))
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
    fn from_request(request: &PanelShaderRequest, width: i32, height: i32) -> Self {
        let motion_rate = request.params.motion_rate.clamp(0.2, 3.0);
        let energy_gain = request.params.energy_gain.clamp(0.2, 2.5);
        let bass_gain = request.params.bass_gain.clamp(0.2, 2.5);
        let mid_gain = request.params.mid_gain.clamp(0.2, 2.5);
        let treble_gain = request.params.treble_gain.clamp(0.2, 2.5);
        let spark_budget =
            ((request.params.particle_count.clamp(8, 96) as f32 - 8.0) / 88.0).clamp(0.0, 1.0);

        Self {
            resolution: (width as f32, height as f32),
            time: request.viewer_tick as f32 / 1000.0,
            energy: (request.analysis.energy * energy_gain).clamp(0.0, 1.0),
            bass: (request.analysis.bass * bass_gain).clamp(0.0, 1.0),
            mid: (request.analysis.mid * mid_gain).clamp(0.0, 1.0),
            treble: (request.analysis.treble * treble_gain * (0.7 + spark_budget * 0.45))
                .clamp(0.0, 1.0),
            peak: request.analysis.peak.clamp(0.0, 1.0),
            progress: request.analysis.progress_ratio.clamp(0.0, 1.0),
            motion_rate,
            ring_count: request.params.ring_count.clamp(1, 8) as f32,
            lattice_density: request.params.lattice_density.clamp(2, 12) as f32,
        }
    }
}

impl RuntimeResources {
    fn new(gl: &glow::Context) -> Result<Self, Error> {
        let scene_program =
            create_program(gl, FULLSCREEN_VERTEX_SHADER, CONTAINMENT_SCENE_FRAGMENT_SHADER)?;
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
                energy: uniform_location(gl, scene_program, "u_energy")?,
                bass: uniform_location(gl, scene_program, "u_bass")?,
                mid: uniform_location(gl, scene_program, "u_mid")?,
                treble: uniform_location(gl, scene_program, "u_treble")?,
                peak: uniform_location(gl, scene_program, "u_peak")?,
                progress: uniform_location(gl, scene_program, "u_progress")?,
                motion_rate: uniform_location(gl, scene_program, "u_motion_rate")?,
                ring_count: uniform_location(gl, scene_program, "u_ring_count")?,
                lattice_density: uniform_location(gl, scene_program, "u_lattice_density")?,
                prev_frame: uniform_location(gl, scene_program, "u_prev_frame")?,
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
            return Err(gl_error("containment framebuffer is incomplete"));
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
        .ok_or_else(|| gl_error(&format!("missing containment shader uniform {name}")))
}

fn gl_error(message: &str) -> Error {
    Error::UnableToRetrieveElementById(message.to_string())
}
