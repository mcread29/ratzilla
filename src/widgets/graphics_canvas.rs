//! Canvas-backed procedural graphics widget support.
//!
//! This widget reserves a Ratatui rectangle and then paints real pixels into that
//! rectangle during backend post-render hooks. It is intended for effects,
//! visualizers, and other custom graphics that should not be quantized into
//! terminal glyphs.

use std::{cell::RefCell, mem, rc::Rc};

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};
use web_sys::{
    wasm_bindgen::{JsCast, JsValue},
    CanvasRenderingContext2d, HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram,
    WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

use crate::{
    backend::{
        canvas::rect_to_canvas_pixels,
        hooks::{BackendKind, RenderHook, RenderHookContext, RenderHookHandle},
        utils::{get_canvas_color, get_document},
    },
    error::Error,
};

type GL = WebGl2RenderingContext;

const WEBGL_VERTEX_SHADER_SOURCE: &str = r#"#version 300 es
layout(location = 0) in vec2 a_position_px;
layout(location = 1) in vec2 a_uv;

uniform vec2 u_viewport_px;

out vec2 v_uv;

void main() {
    vec2 zero_to_one = a_position_px / u_viewport_px;
    vec2 clip_space = vec2(zero_to_one.x * 2.0 - 1.0, 1.0 - zero_to_one.y * 2.0);
    gl_Position = vec4(clip_space, 0.0, 1.0);
    v_uv = a_uv;
}
"#;

const WEBGL_FRAGMENT_SHADER_SOURCE: &str = r#"#version 300 es
precision mediump float;

uniform sampler2D u_texture;

in vec2 v_uv;
out vec4 out_color;

void main() {
    out_color = texture(u_texture, v_uv);
}
"#;

/// Callback trait used to draw into a [`GraphicsCanvas`] surface.
pub trait GraphicsCanvasRenderer {
    /// Draws the current frame into the provided canvas context wrapper.
    fn draw(&self, canvas: &GraphicsCanvasContext<'_>) -> Result<(), Error>;
}

impl<F> GraphicsCanvasRenderer for F
where
    F: for<'a> Fn(&GraphicsCanvasContext<'a>) -> Result<(), Error>,
{
    fn draw(&self, canvas: &GraphicsCanvasContext<'_>) -> Result<(), Error> {
        self(canvas)
    }
}

#[derive(Clone, Default)]
/// Shared overlay state for [`GraphicsCanvas`] widgets.
pub struct GraphicsCanvasLayer {
    state: Rc<RefCell<GraphicsCanvasLayerState>>,
}

impl GraphicsCanvasLayer {
    /// Creates a new graphics canvas layer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a render hook that paints queued graphics widgets after the text pass.
    pub fn render_hook(&self) -> RenderHookHandle {
        RenderHookHandle::new(GraphicsLayerRenderHook {
            shared: Rc::clone(&self.state),
            webgl: None,
        })
    }
}

/// Widget that reserves a text-buffer rectangle and paints procedural graphics over it.
pub struct GraphicsCanvas {
    layer: GraphicsCanvasLayer,
    renderer: Rc<dyn GraphicsCanvasRenderer>,
    style: Style,
    smoothing: bool,
}

impl GraphicsCanvas {
    /// Creates a new graphics canvas widget from a draw callback or renderer type.
    pub fn new<R>(layer: GraphicsCanvasLayer, renderer: R) -> Self
    where
        R: GraphicsCanvasRenderer + 'static,
    {
        Self {
            layer,
            renderer: Rc::new(renderer),
            style: Style::default(),
            smoothing: true,
        }
    }

    /// Sets the placeholder style written into the Ratatui buffer.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Enables or disables smoothing when the widget is composited through WebGL.
    pub fn smoothing(mut self, smoothing: bool) -> Self {
        self.smoothing = smoothing;
        self
    }
}

impl Widget for GraphicsCanvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        fill_area_with_spaces(buf, area, self.style);

        if area.width == 0 || area.height == 0 {
            return;
        }

        self.layer.state.borrow_mut().enqueue(GraphicsDrawCommand {
            area,
            style: self.style,
            smoothing: self.smoothing,
            renderer: self.renderer,
        });
    }
}

/// Draw-time wrapper around a browser 2D canvas context.
pub struct GraphicsCanvasContext<'a> {
    context: &'a CanvasRenderingContext2d,
    width: f64,
    height: f64,
}

impl<'a> GraphicsCanvasContext<'a> {
    /// Returns the raw browser 2D canvas context for direct drawing operations.
    pub fn context(&self) -> &CanvasRenderingContext2d {
        self.context
    }

    /// Returns the local drawable width in pixels.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns the local drawable height in pixels.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Sets the context fill color using Ratatui colors.
    pub fn set_fill_color(&self, color: ratatui::style::Color, fallback: ratatui::style::Color) {
        self.context
            .set_fill_style_str(&get_canvas_color(color, fallback));
    }

    /// Sets the context stroke color using Ratatui colors.
    pub fn set_stroke_color(&self, color: ratatui::style::Color, fallback: ratatui::style::Color) {
        self.context
            .set_stroke_style_str(&get_canvas_color(color, fallback));
    }
}

#[derive(Default)]
struct GraphicsCanvasLayerState {
    queue: Vec<GraphicsDrawCommand>,
}

impl GraphicsCanvasLayerState {
    fn enqueue(&mut self, command: GraphicsDrawCommand) {
        self.queue.push(command);
    }

    fn drain_queue(&mut self) -> Vec<GraphicsDrawCommand> {
        mem::take(&mut self.queue)
    }
}

#[derive(Clone)]
struct GraphicsDrawCommand {
    area: Rect,
    style: Style,
    smoothing: bool,
    renderer: Rc<dyn GraphicsCanvasRenderer>,
}

struct GraphicsLayerRenderHook {
    shared: Rc<RefCell<GraphicsCanvasLayerState>>,
    webgl: Option<WebGlGraphicsRenderer>,
}

struct WebGlGraphicsRenderer {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
    viewport_uniform: WebGlUniformLocation,
    texture_uniform: WebGlUniformLocation,
    texture: Option<WebGlTexture>,
    offscreen_canvas: HtmlCanvasElement,
    offscreen_context: CanvasRenderingContext2d,
}

impl RenderHook for GraphicsLayerRenderHook {
    fn post_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        let queue = self.shared.borrow_mut().drain_queue();
        if queue.is_empty() {
            return Ok(());
        }

        match context.backend() {
            BackendKind::Canvas => {
                let Some(canvas) = context.canvas_2d_context() else {
                    return Ok(());
                };
                for command in queue {
                    draw_graphics_on_canvas(canvas, &command)?;
                }
            }
            BackendKind::WebGl2 => {
                let Some(gl) = context.webgl2_context() else {
                    return Ok(());
                };
                let Some(cell_size) = context.cell_size() else {
                    return Ok(());
                };
                if self.webgl.is_none() {
                    self.webgl = Some(WebGlGraphicsRenderer::new(gl)?);
                }
                if let Some(renderer) = &mut self.webgl {
                    renderer.draw_queue(gl, cell_size, &queue)?;
                }
            }
            BackendKind::Dom => {}
        }

        Ok(())
    }
}

impl WebGlGraphicsRenderer {
    fn new(gl: &GL) -> Result<Self, Error> {
        let vertex_shader =
            compile_webgl_shader(gl, GL::VERTEX_SHADER, WEBGL_VERTEX_SHADER_SOURCE)?;
        let fragment_shader =
            compile_webgl_shader(gl, GL::FRAGMENT_SHADER, WEBGL_FRAGMENT_SHADER_SOURCE)?;
        let program = link_webgl_program(gl, &vertex_shader, &fragment_shader)?;
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| make_error("Failed to create WebGL vertex array"))?;
        let vbo = gl
            .create_buffer()
            .ok_or_else(|| make_error("Failed to create WebGL vertex buffer"))?;
        let viewport_uniform = gl
            .get_uniform_location(&program, "u_viewport_px")
            .ok_or_else(|| make_error("Failed to find WebGL viewport uniform"))?;
        let texture_uniform = gl
            .get_uniform_location(&program, "u_texture")
            .ok_or_else(|| make_error("Failed to find WebGL texture uniform"))?;

        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 16, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 16, 8);
        gl.bind_buffer(GL::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);
        gl.delete_shader(Some(&vertex_shader));
        gl.delete_shader(Some(&fragment_shader));

        let (offscreen_canvas, offscreen_context) = create_detached_canvas_2d_context()?;

        Ok(Self {
            program,
            vao,
            vbo,
            viewport_uniform,
            texture_uniform,
            texture: None,
            offscreen_canvas,
            offscreen_context,
        })
    }

    fn draw_queue(
        &mut self,
        gl: &GL,
        cell_size: (i32, i32),
        queue: &[GraphicsDrawCommand],
    ) -> Result<(), Error> {
        let canvas_width = gl.drawing_buffer_width();
        let canvas_height = gl.drawing_buffer_height();
        if canvas_width <= 0 || canvas_height <= 0 || cell_size.0 <= 0 || cell_size.1 <= 0 {
            return Ok(());
        }

        gl.use_program(Some(&self.program));
        gl.bind_vertex_array(Some(&self.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        gl.viewport(0, 0, canvas_width, canvas_height);
        gl.uniform2f(
            Some(&self.viewport_uniform),
            canvas_width as f32,
            canvas_height as f32,
        );
        gl.uniform1i(Some(&self.texture_uniform), 0);
        gl.active_texture(GL::TEXTURE0);
        gl.disable(GL::DEPTH_TEST);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        for command in queue {
            let destination = rect_to_physical_pixels(command.area, cell_size);
            if destination.2 <= 0.0 || destination.3 <= 0.0 {
                continue;
            }

            render_command_to_offscreen(
                &self.offscreen_canvas,
                &self.offscreen_context,
                command,
                destination.2 as u32,
                destination.3 as u32,
            )?;
            let texture = self.ensure_texture(gl)?;
            gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
            apply_webgl_filtering(gl, command.smoothing);
            upload_canvas_texture(gl, &self.offscreen_canvas)?;

            let vertices = quad_vertices(destination);
            let vertices = web_sys::js_sys::Float32Array::from(vertices.as_slice());
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &vertices, GL::STREAM_DRAW);
            gl.draw_arrays(GL::TRIANGLES, 0, 6);
        }

        gl.bind_texture(GL::TEXTURE_2D, None);
        gl.bind_buffer(GL::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);
        gl.use_program(None);

        Ok(())
    }

    fn ensure_texture(&mut self, gl: &GL) -> Result<WebGlTexture, Error> {
        if let Some(texture) = &self.texture {
            return Ok(texture.clone());
        }

        let texture = gl
            .create_texture()
            .ok_or_else(|| make_error("Failed to create WebGL texture"))?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        gl.bind_texture(GL::TEXTURE_2D, None);
        self.texture = Some(texture.clone());
        Ok(texture)
    }
}

fn draw_graphics_on_canvas(
    context: &CanvasRenderingContext2d,
    command: &GraphicsDrawCommand,
) -> Result<(), Error> {
    let widget_rect = rect_to_canvas_pixels(command.area);
    context.save();
    context.begin_path();
    context.rect(widget_rect.0, widget_rect.1, widget_rect.2, widget_rect.3);
    context.clip();
    context.translate(widget_rect.0, widget_rect.1)?;
    paint_command(context, command, widget_rect.2, widget_rect.3)?;
    context.restore();
    Ok(())
}

fn render_command_to_offscreen(
    canvas: &HtmlCanvasElement,
    context: &CanvasRenderingContext2d,
    command: &GraphicsDrawCommand,
    width: u32,
    height: u32,
) -> Result<(), Error> {
    if canvas.width() != width {
        canvas.set_width(width);
    }
    if canvas.height() != height {
        canvas.set_height(height);
    }
    context.set_image_smoothing_enabled(command.smoothing);
    paint_command(context, command, width as f64, height as f64)
}

fn paint_command(
    context: &CanvasRenderingContext2d,
    command: &GraphicsDrawCommand,
    width: f64,
    height: f64,
) -> Result<(), Error> {
    let bg = get_canvas_color(
        command.style.bg.unwrap_or(ratatui::style::Color::Black),
        ratatui::style::Color::Black,
    );
    context.save();
    context.set_fill_style_str(&bg);
    context.fill_rect(0.0, 0.0, width, height);
    let canvas = GraphicsCanvasContext {
        context,
        width,
        height,
    };
    command.renderer.draw(&canvas)?;
    context.restore();
    Ok(())
}

fn create_detached_canvas_2d_context(
) -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), Error> {
    let canvas = get_document()?
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| make_error("Failed to cast detached canvas element"))?;
    let context = canvas
        .get_context("2d")?
        .ok_or(Error::UnableToRetrieveCanvasContext)?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| make_error("Failed to cast detached canvas context"))?;
    Ok((canvas, context))
}

fn upload_canvas_texture(gl: &GL, canvas: &HtmlCanvasElement) -> Result<(), Error> {
    gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
        GL::TEXTURE_2D,
        0,
        GL::RGBA as i32,
        GL::RGBA,
        GL::UNSIGNED_BYTE,
        canvas,
    )
    .map_err(|error| make_js_error("Failed to upload WebGL canvas texture", error))
}

fn apply_webgl_filtering(gl: &GL, smoothing: bool) {
    let filter = if smoothing { GL::LINEAR } else { GL::NEAREST } as i32;
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, filter);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, filter);
}

fn rect_to_physical_pixels(area: Rect, cell_size: (i32, i32)) -> (f32, f32, f32, f32) {
    (
        area.x as f32 * cell_size.0 as f32,
        area.y as f32 * cell_size.1 as f32,
        area.width as f32 * cell_size.0 as f32,
        area.height as f32 * cell_size.1 as f32,
    )
}

fn quad_vertices(destination: (f32, f32, f32, f32)) -> [f32; 24] {
    let (x0, y0, w, h) = destination;
    let x1 = x0 + w;
    let y1 = y0 + h;

    [
        x0, y0, 0.0, 0.0, x1, y0, 1.0, 0.0, x1, y1, 1.0, 1.0, x0, y0, 0.0, 0.0, x1, y1, 1.0, 1.0,
        x0, y1, 0.0, 1.0,
    ]
}

fn fill_area_with_spaces(buf: &mut Buffer, area: Rect, style: Style) {
    let area = buf.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].set_symbol(" ").set_style(style);
        }
    }
}

fn compile_webgl_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, Error> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| make_error("Failed to create WebGL shader"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        let info = gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown WebGL shader compile error".to_string());
        gl.delete_shader(Some(&shader));
        Err(make_error(info))
    }
}

fn link_webgl_program(
    gl: &GL,
    vertex_shader: &WebGlShader,
    fragment_shader: &WebGlShader,
) -> Result<WebGlProgram, Error> {
    let program = gl
        .create_program()
        .ok_or_else(|| make_error("Failed to create WebGL program"))?;
    gl.attach_shader(&program, vertex_shader);
    gl.attach_shader(&program, fragment_shader);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        let info = gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown WebGL program link error".to_string());
        gl.delete_program(Some(&program));
        Err(make_error(info))
    }
}

fn make_error(message: impl Into<String>) -> Error {
    Error::UnableToRetrieveElementById(message.into())
}

fn make_js_error(prefix: &str, error: JsValue) -> Error {
    make_error(format!("{prefix}: {error:?}"))
}
