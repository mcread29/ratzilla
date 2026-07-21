//! Canvas and WebGL2 video widget support.
//!
//! The video element is owned by [`CanvasVideoLayer`] and keyed by the caller supplied
//! ID, so rebuilding a [`CanvasVideo`] every Ratatui frame does not restart playback.

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    mem,
    rc::Rc,
};

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};
use web_sys::{
    js_sys::{Float32Array, Int32Array},
    wasm_bindgen::{JsCast, JsValue},
    CanvasRenderingContext2d, HtmlVideoElement, MediaError, WebGl2RenderingContext, WebGlBuffer,
    WebGlProgram, WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

use crate::{
    backend::{
        canvas::rect_to_canvas_pixels,
        hooks::{BackendKind, RenderHook, RenderHookContext, RenderHookHandle},
    },
    error::Error,
};

use super::ImageFit;

type GL = WebGl2RenderingContext;
type PlaybackErrorCell = Rc<RefCell<Option<JsValue>>>;

const VERTEX_SHADER: &str = r#"#version 300 es
layout(location = 0) in vec2 a_position_px;
layout(location = 1) in vec2 a_uv;
uniform vec2 u_viewport_px;
out vec2 v_uv;
void main() {
    vec2 p = a_position_px / u_viewport_px;
    gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
    v_uv = a_uv;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
uniform sampler2D u_texture;
in vec2 v_uv;
out vec4 out_color;
void main() { out_color = texture(u_texture, v_uv); }
"#;

/// Cross-origin mode assigned before the video's `src`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VideoCrossOrigin {
    /// Send no credentials.
    Anonymous,
    /// Send credentials when the server permits them.
    UseCredentials,
}

/// Browser preload hint.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VideoPreload {
    /// Do not preload media data.
    None,
    /// Preload metadata only.
    Metadata,
    /// Let the browser preload media data.
    #[default]
    Auto,
}

impl VideoPreload {
    fn attribute(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Metadata => "metadata",
            Self::Auto => "auto",
        }
    }
}

/// A browser media error snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VideoMediaError {
    /// Browser `MediaError.code` value.
    pub code: u16,
    /// Browser-provided diagnostic message, when available.
    pub message: String,
}

/// Errors returned by video controls.
#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    /// No widget with this ID has reached its render hook yet.
    #[error("video instance is not initialized")]
    NotInitialized,
    /// A numeric control value was outside its valid range.
    #[error("invalid video control value: {0}")]
    InvalidValue(&'static str),
    /// The browser rejected the operation (including autoplay-policy rejections).
    #[error("browser rejected video operation: {0:?}")]
    Browser(JsValue),
}

impl From<JsValue> for VideoError {
    fn from(value: JsValue) -> Self {
        Self::Browser(value)
    }
}

/// Shared video state and render-hook factory.
#[derive(Clone, Default)]
pub struct CanvasVideoLayer {
    state: Rc<RefCell<VideoLayerState>>,
}

impl CanvasVideoLayer {
    /// Creates an empty video layer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a controller for an independently identified video.
    pub fn handle(&self, id: impl Into<String>) -> CanvasVideoHandle {
        CanvasVideoHandle {
            id: id.into(),
            state: Rc::clone(&self.state),
        }
    }

    /// Returns a hook which renders queued video widgets.
    pub fn render_hook(&self) -> RenderHookHandle {
        RenderHookHandle::new(VideoRenderHook {
            shared: Rc::clone(&self.state),
            webgl: None,
        })
    }

    /// Releases an element and schedules deletion of its cached WebGL texture.
    pub fn release(&self, id: &str) -> bool {
        let mut state = self.state.borrow_mut();
        let existed = state.instances.remove(id).is_some_and(|instance| {
            release_video_element(&instance.element);
            true
        });
        state.queue.retain(|command| command.id != id);
        state.released.insert(id.to_string());
        existed
    }

    /// Releases all video elements and cached backend resources.
    pub fn clear(&self) {
        let mut state = self.state.borrow_mut();
        let instances = mem::take(&mut state.instances);
        state.released.extend(instances.keys().cloned());
        for instance in instances.into_values() {
            release_video_element(&instance.element);
        }
        state.queue.clear();
        state.clear_resources = true;
    }
}

/// Playback and status access for one video ID.
#[derive(Clone)]
pub struct CanvasVideoHandle {
    id: String,
    state: Rc<RefCell<VideoLayerState>>,
}

impl CanvasVideoHandle {
    fn element(&self) -> Result<HtmlVideoElement, VideoError> {
        self.state
            .borrow()
            .instances
            .get(&self.id)
            .map(|instance| instance.element.clone())
            .ok_or(VideoError::NotInitialized)
    }

    fn playback_parts(&self) -> Result<(HtmlVideoElement, PlaybackErrorCell), VideoError> {
        self.state
            .borrow()
            .instances
            .get(&self.id)
            .map(|instance| {
                (
                    instance.element.clone(),
                    Rc::clone(&instance.last_playback_error),
                )
            })
            .ok_or(VideoError::NotInitialized)
    }

    /// Starts playback and awaits the browser Promise.
    ///
    /// Autoplay-policy rejection is returned to the caller.
    pub async fn play(&self) -> Result<(), VideoError> {
        let (element, last_error) = self.playback_parts()?;
        *last_error.borrow_mut() = None;
        let promise = element.play()?;
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(_) => Ok(()),
            Err(error) => {
                *last_error.borrow_mut() = Some(error.clone());
                Err(VideoError::Browser(error))
            }
        }
    }

    /// Pauses playback.
    pub fn pause(&self) -> Result<(), VideoError> {
        self.element()?.pause()?;
        Ok(())
    }

    /// Plays a paused video, or pauses a playing video.
    pub async fn toggle(&self) -> Result<(), VideoError> {
        if self.paused()? {
            self.play().await
        } else {
            self.pause()
        }
    }

    /// Seeks to an absolute time in seconds.
    pub fn seek(&self, seconds: f64) -> Result<(), VideoError> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(VideoError::InvalidValue(
                "seek time must be finite and non-negative",
            ));
        }
        self.element()?.set_current_time(seconds);
        Ok(())
    }

    /// Sets volume in the inclusive range `0.0..=1.0`.
    pub fn set_volume(&self, volume: f64) -> Result<(), VideoError> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(VideoError::InvalidValue("volume must be between 0 and 1"));
        }
        self.element()?.set_volume(volume);
        Ok(())
    }

    /// Sets the muted state.
    pub fn set_muted(&self, muted: bool) -> Result<(), VideoError> {
        self.element()?.set_muted(muted);
        Ok(())
    }

    /// Returns the current playback position in seconds.
    pub fn current_time(&self) -> Result<f64, VideoError> {
        Ok(self.element()?.current_time())
    }

    /// Returns the duration when metadata provides a finite duration.
    pub fn duration(&self) -> Result<Option<f64>, VideoError> {
        let duration = self.element()?.duration();
        Ok(duration.is_finite().then_some(duration))
    }

    /// Returns whether playback is paused.
    pub fn paused(&self) -> Result<bool, VideoError> {
        Ok(self.element()?.paused())
    }

    /// Returns whether playback reached its end.
    pub fn ended(&self) -> Result<bool, VideoError> {
        Ok(self.element()?.ended())
    }

    /// Returns whether a current frame is available for rendering.
    pub fn ready(&self) -> Result<bool, VideoError> {
        Ok(self.element()?.ready_state() >= 2)
    }

    /// Returns the current media loading/decode error, if any.
    pub fn error(&self) -> Result<Option<VideoMediaError>, VideoError> {
        Ok(self.element()?.error().map(media_error_snapshot))
    }

    /// Returns whether the video is muted.
    pub fn muted(&self) -> Result<bool, VideoError> {
        Ok(self.element()?.muted())
    }

    /// Returns the most recent explicit-play or configured-autoplay rejection.
    pub fn playback_error(&self) -> Result<Option<JsValue>, VideoError> {
        let (_, error) = self.playback_parts()?;
        let snapshot = error.borrow().clone();
        Ok(snapshot)
    }
}

/// A Ratatui widget which paints a direct video file through a backend render hook.
pub struct CanvasVideo<'a> {
    layer: CanvasVideoLayer,
    id: Cow<'a, str>,
    src: Cow<'a, str>,
    fit: ImageFit,
    style: Style,
    smoothing: bool,
    cross_origin: Option<VideoCrossOrigin>,
    looping: bool,
    muted: bool,
    preload: VideoPreload,
    plays_inline: bool,
    autoplay: bool,
}

impl<'a> CanvasVideo<'a> {
    /// Creates a video widget. `id` distinguishes persistent playback instances.
    pub fn new(
        layer: CanvasVideoLayer,
        id: impl Into<Cow<'a, str>>,
        src: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            layer,
            id: id.into(),
            src: src.into(),
            fit: ImageFit::Contain,
            style: Style::default(),
            smoothing: true,
            cross_origin: None,
            looping: false,
            muted: false,
            preload: VideoPreload::Auto,
            plays_inline: true,
            autoplay: false,
        }
    }

    /// Sets the fill, contain, cover, or scale-down fitting strategy.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    /// Sets the Ratatui placeholder style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    /// Enables or disables texture/image smoothing.
    pub fn smoothing(mut self, enabled: bool) -> Self {
        self.smoothing = enabled;
        self
    }
    /// Sets CORS mode. This is applied before `src` whenever the source changes.
    pub fn cross_origin(mut self, mode: VideoCrossOrigin) -> Self {
        self.cross_origin = Some(mode);
        self
    }
    /// Enables or disables looping.
    pub fn looping(mut self, enabled: bool) -> Self {
        self.looping = enabled;
        self
    }
    /// Sets the initial and declarative muted state.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
    /// Sets the browser preload hint.
    pub fn preload(mut self, preload: VideoPreload) -> Self {
        self.preload = preload;
        self
    }
    /// Configures inline mobile playback (enabled by default).
    pub fn plays_inline(mut self, enabled: bool) -> Self {
        self.plays_inline = enabled;
        self
    }
    /// Requests autoplay (disabled by default). Browser policy may still reject playback.
    pub fn autoplay(mut self, enabled: bool) -> Self {
        self.autoplay = enabled;
        self
    }
}

impl Widget for CanvasVideo<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        fill_area(buf, area, self.style);
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.layer.state.borrow_mut().queue.push(VideoCommand {
            id: self.id.into_owned(),
            src: self.src.into_owned(),
            area,
            fit: self.fit,
            smoothing: self.smoothing,
            cross_origin: self.cross_origin,
            looping: self.looping,
            muted: self.muted,
            preload: self.preload,
            plays_inline: self.plays_inline,
            autoplay: self.autoplay,
        });
    }
}

#[derive(Default)]
struct VideoLayerState {
    instances: HashMap<String, VideoInstance>,
    queue: Vec<VideoCommand>,
    released: HashSet<String>,
    clear_resources: bool,
    next_generation: u64,
}

struct VideoInstance {
    element: HtmlVideoElement,
    src: String,
    cross_origin: Option<VideoCrossOrigin>,
    configured_muted: bool,
    last_playback_error: PlaybackErrorCell,
    generation: u64,
}

#[derive(Clone)]
struct VideoCommand {
    id: String,
    src: String,
    area: Rect,
    fit: ImageFit,
    smoothing: bool,
    cross_origin: Option<VideoCrossOrigin>,
    looping: bool,
    muted: bool,
    preload: VideoPreload,
    plays_inline: bool,
    autoplay: bool,
}

struct VideoRenderHook {
    shared: Rc<RefCell<VideoLayerState>>,
    webgl: Option<WebGlVideoRenderer>,
}

impl RenderHook for VideoRenderHook {
    fn post_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        let (queue, released, clear_resources) = {
            let mut state = self.shared.borrow_mut();
            (
                mem::take(&mut state.queue),
                mem::take(&mut state.released),
                mem::take(&mut state.clear_resources),
            )
        };

        if let Some(renderer) = &mut self.webgl {
            if let Some(gl) = context.webgl2_context() {
                if clear_resources {
                    renderer.clear(gl);
                }
                for id in released {
                    renderer.release(gl, &id);
                }
            }
        }
        if queue.is_empty() {
            return Ok(());
        }

        let mut drawable = Vec::with_capacity(queue.len());
        for command in queue {
            match ensure_instance(&self.shared, &command) {
                Ok(video) => drawable.push((command, video)),
                Err(_) => continue,
            }
        }

        match context.backend() {
            BackendKind::Canvas => {
                if let Some(canvas) = context.canvas_2d_context() {
                    for (command, video) in &drawable {
                        draw_canvas(canvas, command, video)?;
                    }
                }
            }
            BackendKind::WebGl2 => {
                let (Some(gl), Some(cell_size)) = (context.webgl2_context(), context.cell_size())
                else {
                    return Ok(());
                };
                if gl.is_context_lost() {
                    return Ok(());
                }
                if self
                    .webgl
                    .as_ref()
                    .is_some_and(|renderer| !gl.is_program(Some(&renderer.program)))
                {
                    self.webgl = None;
                }
                if self.webgl.is_none() {
                    self.webgl = Some(WebGlVideoRenderer::new(gl)?);
                }
                if let Some(renderer) = &mut self.webgl {
                    renderer.draw(gl, cell_size, &drawable)?;
                }
            }
            BackendKind::Dom => {}
        }
        Ok(())
    }
}

fn ensure_instance(
    shared: &Rc<RefCell<VideoLayerState>>,
    command: &VideoCommand,
) -> Result<VideoSnapshot, Error> {
    let mut state = shared.borrow_mut();
    if !state.instances.contains_key(&command.id) {
        let element = crate::backend::utils::get_document()?
            .create_element("video")?
            .dyn_into::<HtmlVideoElement>()
            .map_err(|_| error("failed to create video element"))?;
        element.set_controls(false);
        state.next_generation += 1;
        let generation = state.next_generation;
        let last_playback_error = Rc::new(RefCell::new(None));
        configure_source(&element, command, &last_playback_error);
        state.instances.insert(
            command.id.clone(),
            VideoInstance {
                element,
                src: command.src.clone(),
                cross_origin: command.cross_origin,
                configured_muted: command.muted,
                last_playback_error,
                generation,
            },
        );
    }

    let needs_source = state.instances.get(&command.id).is_some_and(|instance| {
        source_changed(
            &instance.src,
            instance.cross_origin,
            &command.src,
            command.cross_origin,
        )
    });
    if needs_source {
        state.next_generation += 1;
        let generation = state.next_generation;
        let Some(instance) = state.instances.get_mut(&command.id) else {
            return Err(error("video instance disappeared during source update"));
        };
        let _ = instance.element.pause();
        *instance.last_playback_error.borrow_mut() = None;
        configure_source(&instance.element, command, &instance.last_playback_error);
        instance.src.clone_from(&command.src);
        instance.cross_origin = command.cross_origin;
        instance.configured_muted = command.muted;
        instance.generation = generation;
    }
    let Some(instance) = state.instances.get_mut(&command.id) else {
        return Err(error("video instance was not created"));
    };
    apply_non_playback_properties(&instance.element, command);
    if instance.configured_muted != command.muted {
        instance.element.set_muted(command.muted);
        instance.element.set_default_muted(command.muted);
        instance.configured_muted = command.muted;
    }
    Ok(VideoSnapshot {
        element: instance.element.clone(),
        generation: instance.generation,
    })
}

fn release_video_element(element: &HtmlVideoElement) {
    let _ = element.pause();
    let _ = element.remove_attribute("src");
    element.load();
}

fn source_changed(
    current_src: &str,
    current_cors: Option<VideoCrossOrigin>,
    next_src: &str,
    next_cors: Option<VideoCrossOrigin>,
) -> bool {
    current_src != next_src || current_cors != next_cors
}

fn configure_source(
    element: &HtmlVideoElement,
    command: &VideoCommand,
    last_playback_error: &PlaybackErrorCell,
) {
    element.set_cross_origin(command.cross_origin.map(|mode| match mode {
        VideoCrossOrigin::Anonymous => "anonymous",
        VideoCrossOrigin::UseCredentials => "use-credentials",
    }));
    apply_non_playback_properties(element, command);
    element.set_muted(command.muted);
    element.set_default_muted(command.muted);
    element.set_src(&command.src);
    element.load();
    if command.autoplay {
        match element.play() {
            Ok(promise) => {
                let last_playback_error = Rc::clone(last_playback_error);
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(error) = wasm_bindgen_futures::JsFuture::from(promise).await {
                        *last_playback_error.borrow_mut() = Some(error);
                    }
                });
            }
            Err(error) => *last_playback_error.borrow_mut() = Some(error),
        }
    }
}

fn apply_non_playback_properties(element: &HtmlVideoElement, command: &VideoCommand) {
    element.set_loop(command.looping);
    element.set_preload(command.preload.attribute());
    if command.plays_inline {
        let _ = element.set_attribute("playsinline", "");
    } else {
        let _ = element.remove_attribute("playsinline");
    }
    element.set_autoplay(command.autoplay);
}

#[derive(Clone)]
struct VideoSnapshot {
    element: HtmlVideoElement,
    generation: u64,
}

fn media_error_snapshot(error: MediaError) -> VideoMediaError {
    VideoMediaError {
        code: error.code(),
        message: error.message(),
    }
}

fn draw_canvas(
    context: &CanvasRenderingContext2d,
    command: &VideoCommand,
    video: &VideoSnapshot,
) -> Result<(), Error> {
    if video.element.ready_state() < 2 {
        return Ok(());
    }
    let dimensions = (
        video.element.video_width() as f32,
        video.element.video_height() as f32,
    );
    let rect = rect_to_canvas_pixels(command.area);
    let placement = resolve_placement(
        command.fit,
        dimensions.0,
        dimensions.1,
        (rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32),
    );
    if placement.w <= 0.0 || placement.h <= 0.0 {
        return Ok(());
    }
    let source = placement.source_rect(dimensions.0 as f64, dimensions.1 as f64);
    context.save();
    context.begin_path();
    context.rect(rect.0, rect.1, rect.2, rect.3);
    context.clip();
    context.set_image_smoothing_enabled(command.smoothing);
    let result = context
        .draw_image_with_html_video_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &video.element,
            source.0,
            source.1,
            source.2,
            source.3,
            placement.x as f64,
            placement.y as f64,
            placement.w as f64,
            placement.h as f64,
        );
    context.restore();
    result.map_err(Error::from)
}

struct WebGlVideoRenderer {
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
    viewport: WebGlUniformLocation,
    sampler: WebGlUniformLocation,
    textures: HashMap<String, VideoTexture>,
}

struct VideoTexture {
    texture: WebGlTexture,
    generation: u64,
    last_time: Option<f64>,
    uploaded: bool,
    upload_failed: bool,
}

impl WebGlVideoRenderer {
    fn new(gl: &GL) -> Result<Self, Error> {
        let vertex = compile_shader(gl, GL::VERTEX_SHADER, VERTEX_SHADER)?;
        let fragment = compile_shader(gl, GL::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
        let program = link_program(gl, &vertex, &fragment)?;
        gl.delete_shader(Some(&vertex));
        gl.delete_shader(Some(&fragment));
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| error("failed to create video VAO"))?;
        let vbo = gl
            .create_buffer()
            .ok_or_else(|| error("failed to create video VBO"))?;
        let viewport = gl
            .get_uniform_location(&program, "u_viewport_px")
            .ok_or_else(|| error("missing video viewport uniform"))?;
        let sampler = gl
            .get_uniform_location(&program, "u_texture")
            .ok_or_else(|| error("missing video sampler uniform"))?;
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 16, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 16, 8);
        gl.bind_buffer(GL::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);
        Ok(Self {
            program,
            vao,
            vbo,
            viewport,
            sampler,
            textures: HashMap::new(),
        })
    }

    fn draw(
        &mut self,
        gl: &GL,
        cell: (i32, i32),
        items: &[(VideoCommand, VideoSnapshot)],
    ) -> Result<(), Error> {
        let (width, height) = (gl.drawing_buffer_width(), gl.drawing_buffer_height());
        if width <= 0 || height <= 0 || cell.0 <= 0 || cell.1 <= 0 {
            return Ok(());
        }
        let previous_viewport = gl
            .get_parameter(GL::VIEWPORT)
            .ok()
            .map(|value| Int32Array::new(&value).to_vec());
        let previous_program = parameter_object::<WebGlProgram>(gl, GL::CURRENT_PROGRAM);
        let previous_vao = parameter_object::<WebGlVertexArrayObject>(gl, GL::VERTEX_ARRAY_BINDING);
        let previous_array_buffer = parameter_object::<WebGlBuffer>(gl, GL::ARRAY_BUFFER_BINDING);
        let blend_was_enabled = gl.is_enabled(GL::BLEND);
        let blend_src_rgb = parameter_u32(gl, GL::BLEND_SRC_RGB, GL::ONE);
        let blend_dst_rgb = parameter_u32(gl, GL::BLEND_DST_RGB, GL::ZERO);
        let blend_src_alpha = parameter_u32(gl, GL::BLEND_SRC_ALPHA, GL::ONE);
        let blend_dst_alpha = parameter_u32(gl, GL::BLEND_DST_ALPHA, GL::ZERO);
        let depth_was_enabled = gl.is_enabled(GL::DEPTH_TEST);
        let scissor_was_enabled = gl.is_enabled(GL::SCISSOR_TEST);
        let active_texture = gl
            .get_parameter(GL::ACTIVE_TEXTURE)
            .ok()
            .and_then(|value| value.as_f64())
            .map(|value| value as u32)
            .unwrap_or(GL::TEXTURE0);
        let unpack_premultiplied = gl
            .get_parameter(GL::UNPACK_PREMULTIPLY_ALPHA_WEBGL)
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        gl.active_texture(GL::TEXTURE0);
        let previous_texture = parameter_object::<WebGlTexture>(gl, GL::TEXTURE_BINDING_2D);

        gl.use_program(Some(&self.program));
        gl.bind_vertex_array(Some(&self.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        gl.viewport(0, 0, width, height);
        gl.uniform2f(Some(&self.viewport), width as f32, height as f32);
        gl.uniform1i(Some(&self.sampler), 0);
        gl.active_texture(GL::TEXTURE0);
        gl.disable(GL::DEPTH_TEST);
        gl.disable(GL::SCISSOR_TEST);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
        gl.pixel_storei(GL::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);

        for (command, video) in items {
            if video.element.ready_state() < 2 {
                continue;
            }
            let iw = video.element.video_width() as f32;
            let ih = video.element.video_height() as f32;
            let destination = (
                command.area.x as f32 * cell.0 as f32,
                command.area.y as f32 * cell.1 as f32,
                command.area.width as f32 * cell.0 as f32,
                command.area.height as f32 * cell.1 as f32,
            );
            let placement = resolve_placement(command.fit, iw, ih, destination);
            if placement.w <= 0.0 || placement.h <= 0.0 {
                continue;
            }
            let Some(texture) = self.texture(gl, &command.id, video) else {
                continue;
            };
            gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
            let filter = if command.smoothing {
                GL::LINEAR
            } else {
                GL::NEAREST
            } as i32;
            gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, filter);
            gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, filter);
            let vertices = Float32Array::from(quad(&placement).as_slice());
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &vertices, GL::STREAM_DRAW);
            gl.draw_arrays(GL::TRIANGLES, 0, 6);
        }
        gl.bind_texture(GL::TEXTURE_2D, previous_texture.as_ref());
        gl.bind_buffer(GL::ARRAY_BUFFER, previous_array_buffer.as_ref());
        gl.bind_vertex_array(previous_vao.as_ref());
        gl.use_program(previous_program.as_ref());
        gl.pixel_storei(
            GL::UNPACK_PREMULTIPLY_ALPHA_WEBGL,
            i32::from(unpack_premultiplied),
        );
        if let Some(viewport) = previous_viewport.filter(|viewport| viewport.len() == 4) {
            gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
        }
        gl.active_texture(active_texture);
        gl.blend_func_separate(
            blend_src_rgb,
            blend_dst_rgb,
            blend_src_alpha,
            blend_dst_alpha,
        );
        restore_capability(gl, GL::BLEND, blend_was_enabled);
        restore_capability(gl, GL::DEPTH_TEST, depth_was_enabled);
        restore_capability(gl, GL::SCISSOR_TEST, scissor_was_enabled);
        Ok(())
    }

    fn texture(&mut self, gl: &GL, id: &str, video: &VideoSnapshot) -> Option<WebGlTexture> {
        if !self.textures.contains_key(id) {
            let texture = gl.create_texture()?;
            gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
            gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
            gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
            self.textures.insert(
                id.to_string(),
                VideoTexture {
                    texture,
                    generation: video.generation,
                    last_time: None,
                    uploaded: false,
                    upload_failed: false,
                },
            );
        }
        let state = self.textures.get_mut(id)?;
        if state.generation != video.generation {
            state.generation = video.generation;
            state.last_time = None;
            state.uploaded = false;
            state.upload_failed = false;
        }
        if state.upload_failed {
            return None;
        }
        let time = video.element.current_time();
        let changed = !state.uploaded || state.last_time != Some(time);
        if changed {
            gl.bind_texture(GL::TEXTURE_2D, Some(&state.texture));
            if gl
                .tex_image_2d_with_u32_and_u32_and_html_video_element(
                    GL::TEXTURE_2D,
                    0,
                    GL::RGBA as i32,
                    GL::RGBA,
                    GL::UNSIGNED_BYTE,
                    &video.element,
                )
                .is_err()
            {
                state.upload_failed = true;
                return None;
            }
            state.uploaded = true;
            state.last_time = Some(time);
        }
        Some(state.texture.clone())
    }

    fn release(&mut self, gl: &GL, id: &str) {
        if let Some(state) = self.textures.remove(id) {
            gl.delete_texture(Some(&state.texture));
        }
    }
    fn clear(&mut self, gl: &GL) {
        for (_, state) in self.textures.drain() {
            gl.delete_texture(Some(&state.texture));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Placement {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

impl Placement {
    fn source_rect(self, width: f64, height: f64) -> (f64, f64, f64, f64) {
        (
            self.u0 as f64 * width,
            self.v0 as f64 * height,
            (self.u1 - self.u0) as f64 * width,
            (self.v1 - self.v0) as f64 * height,
        )
    }
}

fn resolve_placement(fit: ImageFit, iw: f32, ih: f32, dst: (f32, f32, f32, f32)) -> Placement {
    let (x, y, w, h) = dst;
    if iw <= 0.0 || ih <= 0.0 || w <= 0.0 || h <= 0.0 {
        return Placement {
            x,
            y,
            w: 0.0,
            h: 0.0,
            u0: 0.0,
            v0: 0.0,
            u1: 0.0,
            v1: 0.0,
        };
    }
    match fit {
        ImageFit::Fill => Placement {
            x,
            y,
            w,
            h,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        },
        ImageFit::Contain | ImageFit::ScaleDown => {
            let mut scale = (w / iw).min(h / ih);
            if fit == ImageFit::ScaleDown {
                scale = scale.min(1.0);
            }
            let (rw, rh) = (iw * scale, ih * scale);
            Placement {
                x: x + (w - rw) / 2.0,
                y: y + (h - rh) / 2.0,
                w: rw,
                h: rh,
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            }
        }
        ImageFit::Cover => {
            let scale = (w / iw).max(h / ih);
            let (uw, uh) = (w / (iw * scale), h / (ih * scale));
            Placement {
                x,
                y,
                w,
                h,
                u0: (1.0 - uw) / 2.0,
                v0: (1.0 - uh) / 2.0,
                u1: (1.0 + uw) / 2.0,
                v1: (1.0 + uh) / 2.0,
            }
        }
    }
}

fn quad(p: &Placement) -> [f32; 24] {
    let (x1, y1) = (p.x + p.w, p.y + p.h);
    [
        p.x, p.y, p.u0, p.v0, x1, p.y, p.u1, p.v0, x1, y1, p.u1, p.v1, p.x, p.y, p.u0, p.v0, x1,
        y1, p.u1, p.v1, p.x, y1, p.u0, p.v1,
    ]
}

fn parameter_object<T>(gl: &GL, parameter: u32) -> Option<T>
where
    T: JsCast,
{
    gl.get_parameter(parameter).ok()?.dyn_into().ok()
}

fn parameter_u32(gl: &GL, parameter: u32, fallback: u32) -> u32 {
    gl.get_parameter(parameter)
        .ok()
        .and_then(|value| value.as_f64())
        .map(|value| value as u32)
        .unwrap_or(fallback)
}

fn restore_capability(gl: &GL, capability: u32, enabled: bool) {
    if enabled {
        gl.enable(capability);
    } else {
        gl.disable(capability);
    }
}

fn compile_shader(gl: &GL, kind: u32, source: &str) -> Result<WebGlShader, Error> {
    let shader = gl
        .create_shader(kind)
        .ok_or_else(|| error("failed to create video shader"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        let message = gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "unknown shader error".into());
        gl.delete_shader(Some(&shader));
        Err(error(message))
    }
}

fn link_program(
    gl: &GL,
    vertex: &WebGlShader,
    fragment: &WebGlShader,
) -> Result<WebGlProgram, Error> {
    let program = gl
        .create_program()
        .ok_or_else(|| error("failed to create video program"))?;
    gl.attach_shader(&program, vertex);
    gl.attach_shader(&program, fragment);
    gl.link_program(&program);
    if gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        let message = gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "unknown link error".into());
        gl.delete_program(Some(&program));
        Err(error(message))
    }
}

fn fill_area(buf: &mut Buffer, area: Rect, style: Style) {
    let area = buf.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].set_symbol(" ").set_style(style);
        }
    }
}

fn error(message: impl Into<String>) -> Error {
    Error::UnableToRetrieveElementById(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_and_crop_uvs() {
        let fill = resolve_placement(ImageFit::Fill, 200.0, 100.0, (10.0, 20.0, 50.0, 50.0));
        assert_eq!(
            fill,
            Placement {
                x: 10.0,
                y: 20.0,
                w: 50.0,
                h: 50.0,
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0
            }
        );
        let contain = resolve_placement(ImageFit::Contain, 200.0, 100.0, (0.0, 0.0, 50.0, 50.0));
        assert_eq!(
            (contain.x, contain.y, contain.w, contain.h),
            (0.0, 12.5, 50.0, 25.0)
        );
        let cover = resolve_placement(ImageFit::Cover, 200.0, 100.0, (0.0, 0.0, 50.0, 50.0));
        assert_eq!(
            (cover.u0, cover.v0, cover.u1, cover.v1),
            (0.25, 0.0, 0.75, 1.0)
        );
        let down = resolve_placement(ImageFit::ScaleDown, 20.0, 10.0, (0.0, 0.0, 100.0, 80.0));
        assert_eq!((down.x, down.y, down.w, down.h), (40.0, 35.0, 20.0, 10.0));
    }

    #[test]
    fn source_replacement_is_predictable() {
        assert!(!source_changed("movie.mp4", None, "movie.mp4", None));
        assert!(source_changed("movie.mp4", None, "next.webm", None));
        assert!(source_changed(
            "movie.mp4",
            None,
            "movie.mp4",
            Some(VideoCrossOrigin::Anonymous)
        ));
    }

    #[test]
    fn source_rect_uses_crop_uvs() {
        let p = resolve_placement(ImageFit::Cover, 200.0, 100.0, (0.0, 0.0, 50.0, 50.0));
        assert_eq!(p.source_rect(200.0, 100.0), (50.0, 0.0, 100.0, 100.0));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;
    use web_sys::wasm_bindgen::JsValue;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn creates_and_reuses_a_detached_video_element() {
        let state = Rc::new(RefCell::new(VideoLayerState::default()));
        let mut command = command("first.mp4");
        let first = ensure_instance(&state, &command).expect("video element should be created");
        let second = ensure_instance(&state, &command).expect("video element should be reused");
        assert!(web_sys::js_sys::Object::is(
            &JsValue::from(first.element),
            &JsValue::from(second.element)
        ));

        let generation = second.generation;
        command.src = "second.webm".into();
        let replaced = ensure_instance(&state, &command).expect("source should be replaced");
        assert!(replaced.generation > generation);
        assert_eq!(state.borrow().instances.len(), 1);
    }

    fn command(src: &str) -> VideoCommand {
        VideoCommand {
            id: "player".into(),
            src: src.into(),
            area: Rect::new(0, 0, 4, 2),
            fit: ImageFit::Contain,
            smoothing: true,
            cross_origin: None,
            looping: false,
            muted: true,
            preload: VideoPreload::Metadata,
            plays_inline: true,
            autoplay: false,
        }
    }
}
