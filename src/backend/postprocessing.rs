use crate::error::Error;
use web_sys::{WebGl2RenderingContext, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture};

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

#[derive(Clone, Default)]
/// Post-processing shader.
pub struct PostProcessingShader {
    /// Vertex shader.
    vertex_shader: Option<WebGlShader>,
    /// Fragment shader.
    fragment_shader: Option<WebGlShader>,
}

impl PostProcessingShader {
    /// Constructs a new [`PostProcessingShader`].
    pub fn new(gl: &WebGl2RenderingContext) -> Self {
        let vertex_shader = gl.create_shader(WebGl2RenderingContext::VERTEX_SHADER);
        let fragment_shader = gl.create_shader(WebGl2RenderingContext::FRAGMENT_SHADER);
        Self {
            vertex_shader,
            fragment_shader,
        }
    }

    /// Compiles the shader.
    pub fn compile(
        &mut self,
        gl: &WebGl2RenderingContext,
        options: PostProcessingShaderOptions,
    ) -> Result<(), Error> {
        let vertex_shader = self.vertex_shader.as_ref().unwrap();
        gl.shader_source(vertex_shader, options.vertex_shader_source.as_str());
        gl.compile_shader(vertex_shader);
        if !gl.get_shader_parameter(vertex_shader, WebGl2RenderingContext::COMPILE_STATUS) {
            gl.delete_shader(Some(vertex_shader));
            return Err(Error::UnableToRetrieveElementById(
                gl.get_shader_info_log(vertex_shader)
                    .unwrap_or_else(|| "Vertex shader compilation failed".to_string()),
            ));
        }

        let fragment_shader = self.fragment_shader.as_ref().unwrap();
        gl.shader_source(fragment_shader, options.fragment_shader_source.as_str());
        gl.compile_shader(fragment_shader);
        if !gl.get_shader_parameter(fragment_shader, WebGl2RenderingContext::COMPILE_STATUS) {
            gl.delete_shader(Some(fragment_shader));
            return Err(Error::UnableToRetrieveElementById(
                gl.get_shader_info_log(fragment_shader)
                    .unwrap_or_else(|| "Fragment shader compilation failed".to_string()),
            ));
        }
        Ok(())
    }
}

/// Post-processing.
#[derive(Default)]
pub struct PostProcessing {
    /// enabled
    enabled: bool,
    /// Frame buffer.
    frame_buffer: Option<WebGlFramebuffer>,
    /// Texture.
    texture: Option<WebGlTexture>,
    /// shader source.
    // shader_source: String,
    /// shader.
    shader: Option<PostProcessingShader>,
    /// program.
    program: Option<WebGlProgram>,
}

impl PostProcessing {
    /// Constructs a new [`PostProcessing`].
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            frame_buffer: None,
            texture: None,
            shader: None,
            program: None,
            enabled: false,
        })
    }

    /// Enables post-processing.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Creates a frame buffer
    pub fn create_frame_buffer(&mut self, gl: &WebGl2RenderingContext) -> Result<&mut Self, Error> {
        let frame_buffer = gl
            .create_framebuffer()
            .ok_or(Error::UnableToRetrieveElementById(
                "Failed to create frame buffer".to_string(),
            ))?;
        self.frame_buffer = Some(frame_buffer);
        Ok(self)
    }

    /// Creates a texture
    pub fn create_texture(&mut self, gl: &WebGl2RenderingContext) -> Result<&mut Self, Error> {
        let texture = gl.create_texture();
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, texture.as_ref());
        // set texture filtering to linear
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
        gl.copy_tex_image_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA,
            0,
            0,
            1,
            1,
            0,
        );
        self.texture = Some(texture.ok_or(Error::UnableToRetrieveElementById(
            "Failed to create texture".to_string(),
        ))?);

        Ok(self)
    }

    /// Initializes the post-processing.
    pub fn create_program(
        &mut self,
        gl: &WebGl2RenderingContext,
        shader_options: PostProcessingShaderOptions,
    ) -> Result<&mut Self, Error> {
        let program = gl.create_program();
        let mut shader = PostProcessingShader::new(gl);
        shader.compile(gl, shader_options)?;
        self.shader = Some(shader);
        if let Some(shader) = self.shader.as_mut() {
            gl.attach_shader(
                program.as_ref().unwrap(),
                shader.vertex_shader.as_ref().unwrap(),
            );
            gl.attach_shader(
                program.as_ref().unwrap(),
                shader.fragment_shader.as_ref().unwrap(),
            );
        } else {
            return Err(Error::UnableToRetrieveElementById(
                "Shader not found".to_string(),
            ));
        }

        gl.link_program(program.as_ref().unwrap());
        if !gl.get_program_parameter(
            program.as_ref().unwrap(),
            WebGl2RenderingContext::LINK_STATUS,
        ) {
            gl.delete_program(Some(program.as_ref().unwrap()));
            return Err(Error::UnableToRetrieveElementById(
                gl.get_program_info_log(program.as_ref().unwrap())
                    .unwrap_or_else(|| "Program linking failed".to_string()),
            ));
        }
        self.program = program;
        Ok(self)
    }

    /// Render Functions

    /// Sets up the post-processing.
    pub fn setup(&mut self, gl: &WebGl2RenderingContext) -> Result<&mut Self, Error> {
        if let Some(frame_buffer) = &self.frame_buffer {
            gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&frame_buffer));
            gl.framebuffer_texture_2d(
                WebGl2RenderingContext::FRAMEBUFFER,
                WebGl2RenderingContext::COLOR_ATTACHMENT0,
                WebGl2RenderingContext::TEXTURE_2D,
                self.texture.as_ref(),
                0,
            );
        } else {
            return Err(Error::UnableToRetrieveElementById(
                "Frame buffer not found".to_string(),
            ));
        }
        Ok(self)
    }

    /// Cleans up the post-processing.
    pub fn apply_post_processing(&self, gl: &WebGl2RenderingContext) -> Result<(), Error> {
        gl.bind_buffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        Ok(())
    }
}
