use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    mem,
    rc::Rc,
};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    js_sys::Float32Array,
    wasm_bindgen::JsValue,
    CanvasRenderingContext2d,
    HtmlImageElement,
    WebGl2RenderingContext,
    WebGlBuffer,
    WebGlProgram,
    WebGlShader,
    WebGlTexture,
    WebGlUniformLocation,
    WebGlVertexArrayObject,
};

use crate::{
    backend::{
        canvas::rect_to_canvas_pixels,
        hooks::{BackendKind, RenderHook, RenderHookContext, RenderHookHandle},
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

/// Shared image overlay state for image widgets rendered by canvas or WebGL backends.
#[derive(Clone, Default)]
pub struct CanvasImageLayer {
    state: Rc<RefCell<CanvasImageLayerState>>,
}

impl CanvasImageLayer {
    /// Creates a new image overlay layer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a render hook that paints queued images during `post_render()`.
    pub fn render_hook(&self) -> RenderHookHandle {
        RenderHookHandle::new(ImageLayerRenderHook {
            shared: Rc::clone(&self.state),
            webgl: None,
        })
    }
}

/// Image fitting strategy within the widget rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageFit {
    /// Stretch the image to the full rectangle.
    Fill,
    /// Preserve aspect ratio and letterbox inside the rectangle.
    Contain,
    /// Preserve aspect ratio and crop to fill the rectangle.
    Cover,
    /// Like contain, but never upscale past the intrinsic image size.
    ScaleDown,
}

impl Default for ImageFit {
    fn default() -> Self {
        Self::Contain
    }
}

/// Cross-origin setting applied before image loading starts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageCrossOrigin {
    /// Use anonymous CORS mode.
    Anonymous,
    /// Use credentialed CORS mode.
    UseCredentials,
}

/// Widget that reserves a text buffer rectangle and paints an image over it on canvas and WebGL2 backends.
pub struct CanvasImage<'a> {
    layer: CanvasImageLayer,
    src: Cow<'a, str>,
    fit: ImageFit,
    style: Style,
    smoothing: bool,
    cross_origin: Option<ImageCrossOrigin>,
}

impl<'a> CanvasImage<'a> {
    /// Creates a new canvas image widget for the given source URL or data URL.
    pub fn new(layer: CanvasImageLayer, src: impl Into<Cow<'a, str>>) -> Self {
        Self {
            layer,
            src: src.into(),
            fit: ImageFit::Contain,
            style: Style::default(),
            smoothing: true,
            cross_origin: None,
        }
    }

    /// Sets how the image should fit inside the widget rectangle.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the placeholder style written into the Ratatui buffer.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Enables or disables image smoothing during backend draws.
    pub fn smoothing(mut self, enabled: bool) -> Self {
        self.smoothing = enabled;
        self
    }

    /// Sets the browser `crossOrigin` value before the image source is assigned.
    pub fn cross_origin(mut self, value: ImageCrossOrigin) -> Self {
        self.cross_origin = Some(value);
        self
    }
}

impl Widget for CanvasImage<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        fill_area_with_spaces(buf, area, self.style);

        if area.width == 0 || area.height == 0 {
            return;
        }

        self.layer.state.borrow_mut().enqueue(ImageDrawCommand {
            src: self.src.into_owned(),
            area,
            fit: self.fit,
            style: self.style,
            smoothing: self.smoothing,
            cross_origin: self.cross_origin,
        });
    }
}

#[derive(Default)]
struct CanvasImageLayerState {
    cache: HashMap<String, CachedImage>,
    queue: Vec<ImageDrawCommand>,
}

impl CanvasImageLayerState {
    fn enqueue(&mut self, command: ImageDrawCommand) {
        self.queue.push(command);
    }

    fn drain_queue(&mut self) -> Vec<ImageDrawCommand> {
        mem::take(&mut self.queue)
    }
}

enum CachedImage {
    Loading,
    Ready(DecodedImage),
    Failed,
}

#[derive(Clone)]
struct DecodedImage {
    element: HtmlImageElement,
    width: f64,
    height: f64,
}

#[derive(Clone)]
struct ImageDrawCommand {
    src: String,
    area: Rect,
    fit: ImageFit,
    #[allow(dead_code)]
    style: Style,
    smoothing: bool,
    cross_origin: Option<ImageCrossOrigin>,
}

struct ImageLayerRenderHook {
    shared: Rc<RefCell<CanvasImageLayerState>>,
    webgl: Option<WebGlImageRenderer>,
}

struct WebGlImageRenderer {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
    viewport_uniform: WebGlUniformLocation,
    texture_uniform: WebGlUniformLocation,
    texture_cache: HashMap<String, WebGlTextureState>,
}

enum WebGlTextureState {
    Ready(WebGlTexture),
    Failed,
}

impl RenderHook for ImageLayerRenderHook {
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
                    let image = match get_cached_image(&self.shared, &command.src) {
                        Some(image) => image,
                        None => {
                            ensure_image_in_cache(
                                &self.shared,
                                &command.src,
                                command.cross_origin,
                            );
                            continue;
                        }
                    };

                    draw_image_on_canvas(canvas, &image, &command)?;
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
                    self.webgl = Some(WebGlImageRenderer::new(gl)?);
                }
                if let Some(renderer) = self.webgl.as_mut() {
                    renderer.draw_queue(gl, cell_size, &self.shared, &queue)?;
                }
            }
            BackendKind::Dom => {}
        }

        Ok(())
    }
}

impl WebGlImageRenderer {
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

        Ok(Self {
            program,
            vao,
            vbo,
            viewport_uniform,
            texture_uniform,
            texture_cache: HashMap::new(),
        })
    }

    fn draw_queue(
        &mut self,
        gl: &GL,
        cell_size: (i32, i32),
        shared: &Rc<RefCell<CanvasImageLayerState>>,
        queue: &[ImageDrawCommand],
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
            let image = match get_cached_image(shared, &command.src) {
                Some(image) => image,
                None => {
                    ensure_image_in_cache(shared, &command.src, command.cross_origin);
                    continue;
                }
            };

            let Some(texture) = self.ensure_texture(gl, &command.src, &image) else {
                continue;
            };

            let destination = rect_to_physical_pixels(command.area, cell_size);
            let placement = resolve_image_placement(
                command.fit,
                image.width as f32,
                image.height as f32,
                destination,
            );
            if placement.dst_w <= 0.0 || placement.dst_h <= 0.0 {
                continue;
            }

            gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
            apply_webgl_filtering(gl, command.smoothing);
            let vertices = quad_vertices(&placement);
            let vertices = Float32Array::from(vertices.as_slice());
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &vertices, GL::STREAM_DRAW);
            gl.draw_arrays(GL::TRIANGLES, 0, 6);
        }

        gl.bind_texture(GL::TEXTURE_2D, None);
        gl.bind_buffer(GL::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);
        gl.use_program(None);

        Ok(())
    }

    fn ensure_texture(
        &mut self,
        gl: &GL,
        src: &str,
        image: &DecodedImage,
    ) -> Option<WebGlTexture> {
        match self.texture_cache.get(src) {
            Some(WebGlTextureState::Ready(texture)) => return Some(texture.clone()),
            Some(WebGlTextureState::Failed) => return None,
            None => {}
        }

        match upload_webgl_texture(gl, image) {
            Ok(texture) => {
                self.texture_cache
                    .insert(src.to_string(), WebGlTextureState::Ready(texture.clone()));
                Some(texture)
            }
            Err(_) => {
                self.texture_cache
                    .insert(src.to_string(), WebGlTextureState::Failed);
                None
            }
        }
    }
}

fn fill_area_with_spaces(buf: &mut Buffer, area: Rect, style: Style) {
    let area = buf.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].set_symbol(" ").set_style(style);
        }
    }
}

fn get_cached_image(
    state: &Rc<RefCell<CanvasImageLayerState>>,
    src: &str,
) -> Option<DecodedImage> {
    let state = state.borrow();
    match state.cache.get(src) {
        Some(CachedImage::Ready(image)) => Some(image.clone()),
        Some(CachedImage::Loading | CachedImage::Failed) => None,
        None => None,
    }
}

fn ensure_image_in_cache(
    state: &Rc<RefCell<CanvasImageLayerState>>,
    src: &str,
    cross_origin: Option<ImageCrossOrigin>,
) -> bool {
    let src = src.to_string();
    {
        let mut state_ref = state.borrow_mut();
        match state_ref.cache.entry(src.clone()) {
            Entry::Occupied(_) => return false,
            Entry::Vacant(entry) => {
                entry.insert(CachedImage::Loading);
            }
        }
    }

    let state = Rc::clone(state);
    spawn_local(async move {
        let next_state = match load_image_element(&src, cross_origin).await {
            Ok(image) => CachedImage::Ready(image),
            Err(()) => CachedImage::Failed,
        };

        if let Some(entry) = state.borrow_mut().cache.get_mut(&src) {
            *entry = next_state;
        }
    });

    true
}

async fn load_image_element(
    src: &str,
    cross_origin: Option<ImageCrossOrigin>,
) -> Result<DecodedImage, ()> {
    let element = HtmlImageElement::new().map_err(|_| ())?;
    element.set_cross_origin(cross_origin_attribute(cross_origin));
    element.set_src(src);
    JsFuture::from(element.decode()).await.map_err(|_| ())?;

    let width = element.natural_width() as f64;
    let height = element.natural_height() as f64;
    if width <= 0.0 || height <= 0.0 {
        return Err(());
    }

    Ok(DecodedImage {
        element,
        width,
        height,
    })
}

fn cross_origin_attribute(value: Option<ImageCrossOrigin>) -> Option<&'static str> {
    match value {
        Some(ImageCrossOrigin::Anonymous) => Some("anonymous"),
        Some(ImageCrossOrigin::UseCredentials) => Some("use-credentials"),
        None => None,
    }
}

fn draw_image_on_canvas(
    context: &CanvasRenderingContext2d,
    image: &DecodedImage,
    command: &ImageDrawCommand,
) -> Result<(), Error> {
    let widget_rect = rect_to_canvas_pixels(command.area);
    let placement = resolve_image_placement(
        command.fit,
        image.width as f32,
        image.height as f32,
        (
            widget_rect.0 as f32,
            widget_rect.1 as f32,
            widget_rect.2 as f32,
            widget_rect.3 as f32,
        ),
    );
    if placement.dst_w <= 0.0 || placement.dst_h <= 0.0 {
        return Ok(());
    }

    let (src_x, src_y, src_w, src_h) = source_rect_from_placement(&placement, image);
    context.save();
    context.set_image_smoothing_enabled(command.smoothing);

    if matches!(
        command.fit,
        ImageFit::Fill | ImageFit::Contain | ImageFit::Cover
    ) {
        context.begin_path();
        context.rect(widget_rect.0, widget_rect.1, widget_rect.2, widget_rect.3);
        context.clip();
    }

    let draw_result =
        context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &image.element,
            src_x,
            src_y,
            src_w,
            src_h,
            placement.dst_x as f64,
            placement.dst_y as f64,
            placement.dst_w as f64,
            placement.dst_h as f64,
        );

    context.restore();
    draw_result.map_err(Error::from)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ImagePlacement {
    dst_x: f32,
    dst_y: f32,
    dst_w: f32,
    dst_h: f32,
    uv_min_x: f32,
    uv_min_y: f32,
    uv_max_x: f32,
    uv_max_y: f32,
}

fn resolve_image_placement(
    fit: ImageFit,
    image_width: f32,
    image_height: f32,
    destination: (f32, f32, f32, f32),
) -> ImagePlacement {
    let (dst_x, dst_y, dst_w, dst_h) = destination;
    if image_width <= 0.0 || image_height <= 0.0 || dst_w <= 0.0 || dst_h <= 0.0 {
        return ImagePlacement {
            dst_x,
            dst_y,
            dst_w: 0.0,
            dst_h: 0.0,
            uv_min_x: 0.0,
            uv_min_y: 0.0,
            uv_max_x: 0.0,
            uv_max_y: 0.0,
        };
    }

    match fit {
        ImageFit::Fill => ImagePlacement {
            dst_x,
            dst_y,
            dst_w,
            dst_h,
            uv_min_x: 0.0,
            uv_min_y: 0.0,
            uv_max_x: 1.0,
            uv_max_y: 1.0,
        },
        ImageFit::Contain => contain_placement(image_width, image_height, destination, false),
        ImageFit::ScaleDown => contain_placement(image_width, image_height, destination, true),
        ImageFit::Cover => {
            let scale = f32::max(dst_w / image_width, dst_h / image_height);
            let rendered_w = image_width * scale;
            let rendered_h = image_height * scale;
            let uv_width = dst_w / rendered_w;
            let uv_height = dst_h / rendered_h;
            let uv_min_x = (1.0 - uv_width) / 2.0;
            let uv_min_y = (1.0 - uv_height) / 2.0;
            ImagePlacement {
                dst_x,
                dst_y,
                dst_w,
                dst_h,
                uv_min_x,
                uv_min_y,
                uv_max_x: uv_min_x + uv_width,
                uv_max_y: uv_min_y + uv_height,
            }
        }
    }
}

fn contain_placement(
    image_width: f32,
    image_height: f32,
    destination: (f32, f32, f32, f32),
    clamp_upscale: bool,
) -> ImagePlacement {
    let (dst_x, dst_y, dst_w, dst_h) = destination;
    let mut scale = f32::min(dst_w / image_width, dst_h / image_height);
    if clamp_upscale {
        scale = scale.min(1.0);
    }

    let rendered_w = image_width * scale;
    let rendered_h = image_height * scale;
    ImagePlacement {
        dst_x: dst_x + (dst_w - rendered_w) / 2.0,
        dst_y: dst_y + (dst_h - rendered_h) / 2.0,
        dst_w: rendered_w,
        dst_h: rendered_h,
        uv_min_x: 0.0,
        uv_min_y: 0.0,
        uv_max_x: 1.0,
        uv_max_y: 1.0,
    }
}

fn source_rect_from_placement(
    placement: &ImagePlacement,
    image: &DecodedImage,
) -> (f64, f64, f64, f64) {
    (
        placement.uv_min_x as f64 * image.width,
        placement.uv_min_y as f64 * image.height,
        (placement.uv_max_x - placement.uv_min_x) as f64 * image.width,
        (placement.uv_max_y - placement.uv_min_y) as f64 * image.height,
    )
}

fn rect_to_physical_pixels(area: Rect, cell_size: (i32, i32)) -> (f32, f32, f32, f32) {
    (
        area.x as f32 * cell_size.0 as f32,
        area.y as f32 * cell_size.1 as f32,
        area.width as f32 * cell_size.0 as f32,
        area.height as f32 * cell_size.1 as f32,
    )
}

fn quad_vertices(placement: &ImagePlacement) -> [f32; 24] {
    let x0 = placement.dst_x;
    let y0 = placement.dst_y;
    let x1 = placement.dst_x + placement.dst_w;
    let y1 = placement.dst_y + placement.dst_h;
    let u0 = placement.uv_min_x;
    let v0 = placement.uv_min_y;
    let u1 = placement.uv_max_x;
    let v1 = placement.uv_max_y;

    [
        x0, y0, u0, v0, x1, y0, u1, v0, x1, y1, u1, v1, x0, y0, u0, v0, x1, y1, u1, v1, x0, y1,
        u0, v1,
    ]
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

fn upload_webgl_texture(gl: &GL, image: &DecodedImage) -> Result<WebGlTexture, Error> {
    let texture = gl
        .create_texture()
        .ok_or_else(|| make_error("Failed to create WebGL texture"))?;
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
    let result = gl.tex_image_2d_with_u32_and_u32_and_html_image_element(
        GL::TEXTURE_2D,
        0,
        GL::RGBA as i32,
        GL::RGBA,
        GL::UNSIGNED_BYTE,
        &image.element,
    );
    if let Err(error) = result {
        gl.bind_texture(GL::TEXTURE_2D, None);
        gl.delete_texture(Some(&texture));
        return Err(make_js_error("Failed to upload WebGL image texture", error));
    }
    gl.bind_texture(GL::TEXTURE_2D, None);
    Ok(texture)
}

fn apply_webgl_filtering(gl: &GL, smoothing: bool) {
    let filter = if smoothing { GL::LINEAR } else { GL::NEAREST } as i32;
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, filter);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, filter);
}

#[cfg(test)]
fn texture_upload_needed(state: Option<&WebGlTextureState>) -> bool {
    matches!(state, None)
}

fn make_error(message: impl Into<String>) -> Error {
    Error::UnableToRetrieveElementById(message.into())
}

fn make_js_error(prefix: &str, error: JsValue) -> Error {
    make_error(format!("{prefix}: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_fill_uses_full_destination_and_uvs() {
        let placement =
            resolve_image_placement(ImageFit::Fill, 100.0, 50.0, (10.0, 20.0, 80.0, 40.0));
        assert_eq!(
            placement,
            ImagePlacement {
                dst_x: 10.0,
                dst_y: 20.0,
                dst_w: 80.0,
                dst_h: 40.0,
                uv_min_x: 0.0,
                uv_min_y: 0.0,
                uv_max_x: 1.0,
                uv_max_y: 1.0,
            }
        );
    }

    #[test]
    fn fit_contain_preserves_aspect_ratio() {
        let placement =
            resolve_image_placement(ImageFit::Contain, 200.0, 100.0, (5.0, 10.0, 50.0, 50.0));
        assert_eq!(placement.dst_x, 5.0);
        assert_eq!(placement.dst_y, 22.5);
        assert_eq!(placement.dst_w, 50.0);
        assert_eq!(placement.dst_h, 25.0);
        assert_eq!(placement.uv_min_x, 0.0);
        assert_eq!(placement.uv_max_y, 1.0);
    }

    #[test]
    fn fit_cover_crops_uvs() {
        let placement =
            resolve_image_placement(ImageFit::Cover, 200.0, 100.0, (0.0, 0.0, 50.0, 50.0));
        assert_eq!(placement.dst_w, 50.0);
        assert_eq!(placement.dst_h, 50.0);
        assert_eq!(placement.uv_min_x, 0.25);
        assert_eq!(placement.uv_min_y, 0.0);
        assert_eq!(placement.uv_max_x, 0.75);
        assert_eq!(placement.uv_max_y, 1.0);
    }

    #[test]
    fn fit_scale_down_never_upscales() {
        let placement =
            resolve_image_placement(ImageFit::ScaleDown, 20.0, 10.0, (10.0, 20.0, 100.0, 80.0));
        assert_eq!(placement.dst_x, 50.0);
        assert_eq!(placement.dst_y, 55.0);
        assert_eq!(placement.dst_w, 20.0);
        assert_eq!(placement.dst_h, 10.0);
    }

    #[test]
    fn command_order_is_stable() {
        let mut state = CanvasImageLayerState::default();
        state.enqueue(test_command("first"));
        state.enqueue(test_command("second"));
        state.enqueue(test_command("third"));

        let drained = state.drain_queue();
        let sources: Vec<_> = drained.iter().map(|command| command.src.as_str()).collect();
        assert_eq!(sources, vec!["first", "second", "third"]);
    }

    #[test]
    fn post_render_drains_queue_even_without_backend_context() {
        let state = Rc::new(RefCell::new(CanvasImageLayerState::default()));
        state.borrow_mut().enqueue(test_command("pending"));
        let mut hook = ImageLayerRenderHook {
            shared: Rc::clone(&state),
            webgl: None,
        };

        hook.post_render(&RenderHookContext::new(BackendKind::WebGl2, 80, 24))
            .expect("post render should succeed");

        assert!(state.borrow().queue.is_empty());
        assert!(state.borrow().cache.is_empty());
    }

    #[test]
    fn non_canvas_backend_is_a_no_op() {
        let state = Rc::new(RefCell::new(CanvasImageLayerState::default()));
        state.borrow_mut().enqueue(test_command("pending"));
        let mut hook = ImageLayerRenderHook {
            shared: Rc::clone(&state),
            webgl: None,
        };

        hook.post_render(&RenderHookContext::new(BackendKind::Dom, 80, 24))
            .expect("dom post render should no-op");

        assert!(state.borrow().queue.is_empty());
        assert!(state.borrow().cache.is_empty());
    }

    #[test]
    fn failed_texture_state_suppresses_reupload() {
        assert!(texture_upload_needed(None));
        assert!(!texture_upload_needed(Some(&WebGlTextureState::Failed)));
    }

    fn test_command(src: &str) -> ImageDrawCommand {
        ImageDrawCommand {
            src: src.to_string(),
            area: Rect::new(0, 0, 4, 2),
            fit: ImageFit::Contain,
            style: Style::default(),
            smoothing: true,
            cross_origin: None,
        }
    }
}

#[cfg(test)]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;
    use web_sys::{
        js_sys::Promise,
        wasm_bindgen::JsCast,
        window,
        HtmlCanvasElement,
    };

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    const VALID_DATA_URL: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='4' height='2' viewBox='0 0 4 2'%3E%3Crect width='4' height='2' fill='red'/%3E%3C/svg%3E";
    const INVALID_DATA_URL: &str = "data:image/png;base64,not-valid";

    #[wasm_bindgen_test(async)]
    async fn data_url_transitions_from_loading_to_ready() {
        let state = Rc::new(RefCell::new(CanvasImageLayerState::default()));
        assert!(ensure_image_in_cache(&state, VALID_DATA_URL, None));
        assert!(matches!(
            state.borrow().cache.get(VALID_DATA_URL),
            Some(CachedImage::Loading)
        ));

        wait_for_cache_state(&state, VALID_DATA_URL, CacheState::Ready).await;
        assert!(matches!(
            state.borrow().cache.get(VALID_DATA_URL),
            Some(CachedImage::Ready(_))
        ));
    }

    #[wasm_bindgen_test(async)]
    async fn webgl_post_render_draws_without_error_after_image_is_ready() {
        let loaded = load_image_element(VALID_DATA_URL, None)
            .await
            .expect("data url should decode");
        let gl = create_webgl2_context();
        let state = Rc::new(RefCell::new(CanvasImageLayerState::default()));
        state
            .borrow_mut()
            .cache
            .insert(VALID_DATA_URL.to_string(), CachedImage::Ready(loaded));
        state.borrow_mut().enqueue(ImageDrawCommand {
            src: VALID_DATA_URL.to_string(),
            area: Rect::new(0, 0, 6, 4),
            fit: ImageFit::Contain,
            style: Style::default(),
            smoothing: true,
            cross_origin: None,
        });

        let mut hook = ImageLayerRenderHook {
            shared: Rc::clone(&state),
            webgl: None,
        };
        hook.post_render(
            &RenderHookContext::new(BackendKind::WebGl2, 160, 120)
                .with_cell_size(10, 10)
                .with_webgl2_context(&gl),
        )
        .expect("webgl draw should succeed");

        let renderer = hook.webgl.as_ref().expect("renderer should initialize");
        assert_eq!(renderer.texture_cache.len(), 1);
        assert!(matches!(
            renderer.texture_cache.get(VALID_DATA_URL),
            Some(WebGlTextureState::Ready(_))
        ));
    }

    #[wasm_bindgen_test(async)]
    async fn invalid_url_transitions_to_failed_without_panicking() {
        let state = Rc::new(RefCell::new(CanvasImageLayerState::default()));
        assert!(ensure_image_in_cache(&state, INVALID_DATA_URL, None));
        wait_for_cache_state(&state, INVALID_DATA_URL, CacheState::Failed).await;
        assert!(matches!(
            state.borrow().cache.get(INVALID_DATA_URL),
            Some(CachedImage::Failed)
        ));
    }

    #[wasm_bindgen_test(async)]
    async fn repeated_webgl_renders_reuse_the_same_texture() {
        let loaded = load_image_element(VALID_DATA_URL, None)
            .await
            .expect("data url should decode");
        let gl = create_webgl2_context();
        let state = Rc::new(RefCell::new(CanvasImageLayerState::default()));
        state
            .borrow_mut()
            .cache
            .insert(VALID_DATA_URL.to_string(), CachedImage::Ready(loaded));
        let mut hook = ImageLayerRenderHook {
            shared: Rc::clone(&state),
            webgl: None,
        };

        state.borrow_mut().enqueue(test_command());
        hook.post_render(
            &RenderHookContext::new(BackendKind::WebGl2, 160, 120)
                .with_cell_size(10, 10)
                .with_webgl2_context(&gl),
        )
        .expect("first webgl draw should succeed");
        assert_eq!(
            hook.webgl
                .as_ref()
                .expect("renderer should initialize")
                .texture_cache
                .len(),
            1
        );

        state.borrow_mut().enqueue(test_command());
        hook.post_render(
            &RenderHookContext::new(BackendKind::WebGl2, 160, 120)
                .with_cell_size(10, 10)
                .with_webgl2_context(&gl),
        )
        .expect("second webgl draw should succeed");
        assert_eq!(
            hook.webgl
                .as_ref()
                .expect("renderer should persist")
                .texture_cache
                .len(),
            1
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CacheState {
        Loading,
        Ready,
        Failed,
        Missing,
    }

    async fn wait_for_cache_state(
        state: &Rc<RefCell<CanvasImageLayerState>>,
        src: &str,
        expected: CacheState,
    ) {
        for _ in 0..100 {
            if current_cache_state(state, src) == expected {
                return;
            }
            next_tick().await;
        }

        panic!(
            "timed out waiting for {src} to reach {expected:?}; current state: {:?}",
            current_cache_state(state, src)
        );
    }

    fn current_cache_state(state: &Rc<RefCell<CanvasImageLayerState>>, src: &str) -> CacheState {
        match state.borrow().cache.get(src) {
            Some(CachedImage::Loading) => CacheState::Loading,
            Some(CachedImage::Ready(_)) => CacheState::Ready,
            Some(CachedImage::Failed) => CacheState::Failed,
            None => CacheState::Missing,
        }
    }

    async fn next_tick() {
        let promise = Promise::new(&mut |resolve, _reject| {
            window()
                .expect("window should exist")
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
                .expect("timeout should schedule");
        });
        let _ = JsFuture::from(promise).await;
    }

    fn create_webgl2_context() -> WebGl2RenderingContext {
        let document = window()
            .expect("window should exist")
            .document()
            .expect("document should exist");
        let canvas = document
            .create_element("canvas")
            .expect("canvas element should be created")
            .dyn_into::<HtmlCanvasElement>()
            .expect("element should be canvas");
        canvas.set_width(160);
        canvas.set_height(120);
        canvas
            .get_context("webgl2")
            .expect("context lookup should succeed")
            .expect("webgl2 context should exist")
            .dyn_into::<WebGl2RenderingContext>()
            .expect("context should be webgl2")
    }

    fn test_command() -> ImageDrawCommand {
        ImageDrawCommand {
            src: VALID_DATA_URL.to_string(),
            area: Rect::new(0, 0, 4, 2),
            fit: ImageFit::Contain,
            style: Style::default(),
            smoothing: true,
            cross_origin: None,
        }
    }
}
