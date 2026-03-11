use std::collections::HashMap;

use glow::{self, HasContext, PixelUnpackData};
use ratzilla::{
    backend::hooks::{RenderHook, RenderHookContext},
    error::Error,
};

use crate::{
    overlay_state::OverlayRenderState,
    shaders::{FRAGMENT_SHADER_SOURCE, VERTEX_SHADER_SOURCE},
};

struct PostProcessingShaderOptions {
    vertex_shader_source: String,
    fragment_shader_source: String,
}

impl PostProcessingShaderOptions {
    fn new(vertex_shader_source: String, fragment_shader_source: String) -> Self {
        Self {
            vertex_shader_source,
            fragment_shader_source,
        }
    }
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

    let success = unsafe { gl.get_shader_compile_status(shader) };
    if !success {
        let info = unsafe { gl.get_shader_info_log(shader) };
        unsafe {
            gl.delete_shader(shader);
        }
        return Err(Error::UnableToRetrieveElementById(info));
    }

    Ok(shader)
}

#[derive(Default)]
pub struct PostProcessing {
    overlay_state: OverlayRenderState,
    frame_buffer: Option<glow::Framebuffer>,
    texture: Option<glow::Texture>,
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    uniform_map: HashMap<String, glow::UniformLocation>,
    width: i32,
    height: i32,
    start_time: Option<web_time::Instant>,
}

const UNIFORM_NAMES: [&str; 9] = [
    "u_scene",
    "u_time",
    "u_resolution",
    "u_curvature",
    "u_scanline_strength",
    "u_mask_strength",
    "u_vignette_strength",
    "u_aberration",
    "u_bloom",
];

impl PostProcessing {
    pub fn new(overlay_state: OverlayRenderState) -> Self {
        Self {
            overlay_state,
            frame_buffer: None,
            texture: None,
            program: None,
            vao: None,
            uniform_map: HashMap::new(),
            width: 0,
            height: 0,
            start_time: None,
        }
    }

    fn ensure_initialized(&mut self, gl: &glow::Context) -> Result<(), Error> {
        if self.program.is_some() {
            return Ok(());
        }
        *self = Self::initialize(self.overlay_state.clone(), gl)?;
        Ok(())
    }

    fn initialize(overlay_state: OverlayRenderState, gl: &glow::Context) -> Result<Self, Error> {
        let frame_buffer = unsafe {
            gl.create_framebuffer()
                .map_err(Error::UnableToRetrieveElementById)?
        };
        let texture = Self::create_tex(gl, 0, 0)?;
        let program = Self::create_program(
            gl,
            PostProcessingShaderOptions::new(
                VERTEX_SHADER_SOURCE.to_string(),
                FRAGMENT_SHADER_SOURCE.to_string(),
            ),
        )?;
        let vao = Self::create_vao(gl)?;

        let mut uniform_map = HashMap::new();
        for uniform_name in UNIFORM_NAMES {
            let location = unsafe { gl.get_uniform_location(program, uniform_name) }.ok_or(
                Error::UnableToRetrieveElementById(format!(
                    "Failed to get uniform location for {uniform_name}"
                )),
            )?;
            uniform_map.insert(uniform_name.to_string(), location);
        }

        Ok(Self {
            overlay_state,
            frame_buffer: Some(frame_buffer),
            texture: Some(texture),
            program: Some(program),
            vao: Some(vao),
            uniform_map,
            width: 0,
            height: 0,
            start_time: Some(web_time::Instant::now()),
        })
    }

    fn create_tex(gl: &glow::Context, width: i32, height: i32) -> Result<glow::Texture, Error> {
        let texture = unsafe {
            gl.create_texture()
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
        }

        Ok(texture)
    }

    fn create_program(
        gl: &glow::Context,
        shader_options: PostProcessingShaderOptions,
    ) -> Result<glow::Program, Error> {
        let program = unsafe {
            gl.create_program()
                .map_err(Error::UnableToRetrieveElementById)?
        };
        let vertex_shader = create_shader(
            gl,
            ShaderType::Vertex,
            shader_options.vertex_shader_source.as_str(),
        )?;
        let fragment_shader = create_shader(
            gl,
            ShaderType::Fragment,
            shader_options.fragment_shader_source.as_str(),
        )?;

        unsafe {
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);
        }

        let success = unsafe { gl.get_program_link_status(program) };
        if !success {
            let info = unsafe { gl.get_program_info_log(program) };
            unsafe {
                gl.delete_program(program);
            }
            return Err(Error::UnableToRetrieveElementById(info));
        }

        unsafe {
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);
        }

        Ok(program)
    }

    fn create_vao(gl: &glow::Context) -> Result<glow::VertexArray, Error> {
        unsafe {
            gl.create_vertex_array()
                .map_err(Error::UnableToRetrieveElementById)
        }
    }

    fn resize(&mut self, gl: &glow::Context, width: i32, height: i32) -> Result<(), Error> {
        if width == self.width && height == self.height {
            return Ok(());
        }

        self.width = width;
        self.height = height;

        if let Some(frame_buffer) = self.frame_buffer.take() {
            unsafe {
                gl.delete_framebuffer(frame_buffer);
            }
        }
        if let Some(texture) = self.texture.take() {
            unsafe {
                gl.delete_texture(texture);
            }
        }

        let texture = Self::create_tex(gl, width, height)?;
        let frame_buffer = unsafe {
            gl.create_framebuffer()
                .map_err(Error::UnableToRetrieveElementById)?
        };

        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(frame_buffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        self.texture = Some(texture);
        self.frame_buffer = Some(frame_buffer);
        Ok(())
    }

    fn setup_scene(&mut self, gl: &glow::Context, width: i32, height: i32) -> Result<(), Error> {
        self.resize(gl, width, height)?;

        let frame_buffer = self.frame_buffer.ok_or(Error::UnableToRetrieveElementById(
            "Failed to get frame buffer".to_string(),
        ))?;

        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(frame_buffer));
            gl.viewport(0, 0, self.width, self.height);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        Ok(())
    }

    fn present_scene(&self, gl: &glow::Context, canvas_width: i32, canvas_height: i32) {
        let Some(program) = self.program else {
            return;
        };
        let Some(vao) = self.vao else {
            return;
        };
        let Some(texture) = self.texture else {
            return;
        };

        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, canvas_width, canvas_height);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);

            gl.use_program(Some(program));
            gl.bind_vertex_array(Some(vao));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.uniform_1_i32(self.uniform_map.get("u_scene"), 0);

            gl.uniform_2_f32(
                self.uniform_map.get("u_resolution"),
                self.width as f32,
                self.height as f32,
            );

            let elapsed = web_time::Instant::now()
                .duration_since(self.start_time.unwrap_or_else(web_time::Instant::now))
                .as_secs_f32();
            gl.uniform_1_f32(self.uniform_map.get("u_time"), elapsed);

            gl.uniform_1_f32(self.uniform_map.get("u_curvature"), 0.025);
            gl.uniform_1_f32(self.uniform_map.get("u_scanline_strength"), 0.9);
            gl.uniform_1_f32(self.uniform_map.get("u_mask_strength"), 0.9);
            gl.uniform_1_f32(self.uniform_map.get("u_vignette_strength"), 0.45);
            gl.uniform_1_f32(self.uniform_map.get("u_aberration"), 0.9);
            gl.uniform_1_f32(self.uniform_map.get("u_bloom"), 0.8);

            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
}

impl RenderHook for PostProcessing {
    fn pre_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        if self.overlay_state.editor_open() {
            return Ok(());
        }
        let gl = context.webgl_context().ok_or_else(|| {
            Error::UnableToRetrieveElementById("WebGL context unavailable".into())
        })?;
        let (canvas_width, canvas_height) = context.canvas_size();
        self.ensure_initialized(gl)?;
        self.setup_scene(gl, canvas_width, canvas_height)
    }

    fn post_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        if self.overlay_state.editor_open() {
            return Ok(());
        }
        let gl = context.webgl_context().ok_or_else(|| {
            Error::UnableToRetrieveElementById("WebGL context unavailable".into())
        })?;
        let (canvas_width, canvas_height) = context.canvas_size();
        self.ensure_initialized(gl)?;
        self.present_scene(gl, canvas_width, canvas_height);
        Ok(())
    }
}
