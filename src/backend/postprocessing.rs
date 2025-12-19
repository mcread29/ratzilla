use crate::backend::shaders::{FRAGMENT_SHADER_SOURCE, VERTEX_SHADER_SOURCE};
use crate::error::Error;
use std::collections::HashMap;
use web_sys::{
    WebGl2RenderingContext, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

/// Options for the [`PostProcessingShader`].
pub struct PostProcessingShaderOptions {
    /// vertex shader source.
    vertex_shader_source: String,
    /// fragment shader source.
    fragment_shader_source: String,
}

impl PostProcessingShaderOptions {
    /// Constructs a new [`PostProcessingShader`].
    pub fn new(vertex_shader_source: String, fragment_shader_source: String) -> Self {
        Self {
            vertex_shader_source,
            fragment_shader_source,
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// Shader type.
pub enum ShaderType {
    /// Vertex shader.
    Vertex,
    /// Fragment shader.
    Fragment,
}

impl ShaderType {
    /// Returns the GL shader type.
    pub fn as_gl_shader_type(&self) -> u32 {
        match self {
            ShaderType::Vertex => WebGl2RenderingContext::VERTEX_SHADER as u32,
            ShaderType::Fragment => WebGl2RenderingContext::FRAGMENT_SHADER as u32,
        }
    }
}

/// Creates a new shader.
fn create_shader(
    gl: &WebGl2RenderingContext,
    shader_type: ShaderType,
    shader_source: &str,
) -> Result<WebGlShader, Error> {
    let shader = gl.create_shader(shader_type.as_gl_shader_type()).ok_or(
        Error::UnableToRetrieveElementById("Failed to create shader".to_string()),
    )?;
    gl.shader_source(shader.as_ref(), shader_source);
    gl.compile_shader(shader.as_ref());
    if !gl.get_shader_parameter(shader.as_ref(), WebGl2RenderingContext::COMPILE_STATUS) {
        gl.delete_shader(Some(shader.as_ref()));
        return Err(Error::UnableToRetrieveElementById(
            gl.get_shader_info_log(shader.as_ref())
                .unwrap_or_else(|| "Shader compilation failed".to_string()),
        ));
    }
    Ok(shader)
}

/// Post-processing.
#[derive(Default)]
pub struct PostProcessing {
    /// Frame buffer.
    frame_buffer: Option<WebGlFramebuffer>,
    /// Texture.
    texture: Option<WebGlTexture>,
    /// program.
    program: Option<WebGlProgram>,
    /// vao.
    vao: Option<WebGlVertexArrayObject>,
    /// uniform map
    uniform_map: HashMap<String, WebGlUniformLocation>,
    /// width.
    width: i32,
    /// height.
    height: i32,
    /// time.
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
    /// Constructs a new [`PostProcessing`].
    pub fn new(gl: &WebGl2RenderingContext) -> Result<Self, Error> {
        let frame_buffer = gl
            .create_framebuffer()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to create frame buffer".to_string(),
            ))?;

        let texture = Self::create_tex(gl, 0, 0)?;

        let program = Self::create_program(
            gl,
            PostProcessingShaderOptions::new(
                VERTEX_SHADER_SOURCE.to_string(),
                FRAGMENT_SHADER_SOURCE.to_string(),
            ),
        )?;

        let vao = Self::create_vao(gl)?;

        let mut uniform_map: HashMap<String, WebGlUniformLocation> = HashMap::new();
        for uniform_name in UNIFORM_NAMES {
            uniform_map.insert(
                uniform_name.to_string(),
                gl.get_uniform_location(program.as_ref(), uniform_name)
                    .ok_or(Error::UnableToRetrieveElementById(format!(
                        "Failed to get uniform location for {}",
                        uniform_name
                    )))?,
            );
        }

        Ok(Self {
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

    /// Creates a new texture.
    fn create_tex(
        gl: &WebGl2RenderingContext,
        width: i32,
        height: i32,
    ) -> Result<WebGlTexture, Error> {
        let texture = gl
            .create_texture()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to create texture".to_string(),
            ))?;
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        // set texture filtering to linear (both min and mag, matches TypeScript reference)
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        // set texture wrapping to clamp to edge
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        // allocate texture storage - use RGBA8 as internal format (matches TypeScript reference)
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA8 as i32,
            width,
            height,
            0,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            None,
        )?;
        Ok(texture)
    }

    /// Initializes the post-processing.
    pub fn create_program(
        gl: &WebGl2RenderingContext,
        shader_options: PostProcessingShaderOptions,
    ) -> Result<WebGlProgram, Error> {
        let program = gl
            .create_program()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to create program".to_string(),
            ))?;
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
        gl.attach_shader(program.as_ref(), vertex_shader.as_ref());
        gl.attach_shader(program.as_ref(), fragment_shader.as_ref());

        gl.link_program(program.as_ref());
        if !gl.get_program_parameter(program.as_ref(), WebGl2RenderingContext::LINK_STATUS) {
            gl.delete_program(Some(program.as_ref()));
            return Err(Error::UnableToRetrieveElementById(
                gl.get_program_info_log(program.as_ref())
                    .unwrap_or_else(|| "Program linking failed".to_string()),
            ));
        }
        gl.delete_shader(Some(vertex_shader.as_ref()));
        gl.delete_shader(Some(fragment_shader.as_ref()));
        Ok(program)
    }

    /// Creates a new vertex array object.
    fn create_vao(gl: &WebGl2RenderingContext) -> Result<WebGlVertexArrayObject, Error> {
        let prev_vao = gl
            .get_parameter(WebGl2RenderingContext::VERTEX_ARRAY_BINDING)
            .ok()
            .map(|v| WebGlVertexArrayObject::from(v));
        let vao = gl
            .create_vertex_array()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to create vertex array".to_string(),
            ))?;
        gl.bind_vertex_array(Some(vao.as_ref()));
        gl.bind_vertex_array(prev_vao.as_ref());
        Ok(vao)
    }
    /// Render Functions

    pub fn resize(
        &mut self,
        gl: &WebGl2RenderingContext,
        width: i32,
        height: i32,
    ) -> Result<&Self, Error> {
        if width == self.width && height == self.height {
            return Ok(self);
        }
        self.width = width;
        self.height = height;

        if let Some(frame_buffer) = &self.frame_buffer {
            gl.delete_framebuffer(Some(frame_buffer));
        }
        if let Some(texture) = &self.texture {
            gl.delete_texture(Some(texture));
        }
        self.texture = Some(Self::create_tex(gl, width, height)?);
        let fb = gl
            .create_framebuffer()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to create frame buffer".to_string(),
            ))?;
        // attach texture to framebuffer
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&fb));
        let texture = self
            .texture
            .as_ref()
            .ok_or(Error::UnableToRetrieveElementById(
                "Texture not created".to_string(),
            ))?;
        gl.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT0,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(texture),
            0,
        );
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.frame_buffer = Some(fb);
        Ok(self)
    }

    /// set up post processing scene
    pub fn setup_scene(
        &mut self,
        gl: &WebGl2RenderingContext,
        width: i32,
        height: i32,
    ) -> Result<(), Error> {
        self.resize(gl, width, height)?;
        let fb = self
            .frame_buffer
            .as_ref()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to get frame buffer".to_string(),
            ))?;
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(fb));
        gl.viewport(0, 0, self.width, self.height);
        // clear the framebuffer before rendering
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        Ok(())
    }

    /// present scene
    pub fn present_scene(
        &self,
        gl: &WebGl2RenderingContext,
        canvas_width: i32,
        canvas_height: i32,
    ) {
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        gl.viewport(0, 0, canvas_width, canvas_height);
        // clear the canvas before drawing the post-processed texture
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        gl.disable(WebGl2RenderingContext::DEPTH_TEST);
        gl.disable(WebGl2RenderingContext::BLEND);

        gl.use_program(self.program.as_ref());
        gl.bind_vertex_array(self.vao.as_ref());

        gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.texture.as_ref());
        gl.uniform1i(self.uniform_map.get("u_scene"), 0);

        gl.uniform2f(
            self.uniform_map.get("u_resolution"),
            self.width as f32,
            self.height as f32,
        );

        let now = web_time::Instant::now();
        let elapsed = now.duration_since(self.start_time.unwrap()).as_secs_f32();
        gl.uniform1f(self.uniform_map.get("u_time"), elapsed);

        gl.uniform1f(self.uniform_map.get("u_curvature"), 0.025);
        gl.uniform1f(self.uniform_map.get("u_scanline_strength"), 0.9);
        gl.uniform1f(self.uniform_map.get("u_mask_strength"), 0.9);
        gl.uniform1f(self.uniform_map.get("u_vignette_strength"), 0.45);
        gl.uniform1f(self.uniform_map.get("u_aberration"), 0.9);
        gl.uniform1f(self.uniform_map.get("u_bloom"), 0.8);

        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);

        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
        gl.bind_vertex_array(None);
        gl.use_program(None);
    }
}
