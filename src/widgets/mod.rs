//! ## Widgets
//!
//! **Ratzilla** provides web-only widgets that you can use while building TUIs.

/// Canvas-first image widget and overlay hook support.
pub mod canvas_image;
/// Canvas and WebGL2 video widget and playback support.
pub mod canvas_video;
/// Canvas-backed procedural graphics widget and overlay hook support.
pub mod graphics_canvas;
pub(crate) mod hyperlink;

pub use canvas_image::{CanvasImage, CanvasImageLayer, ImageCrossOrigin, ImageFit};
pub use canvas_video::{
    CanvasVideo, CanvasVideoHandle, CanvasVideoLayer, VideoCrossOrigin, VideoError,
    VideoMediaError, VideoPreload,
};
pub use graphics_canvas::{
    GraphicsCanvas, GraphicsCanvasContext, GraphicsCanvasLayer, GraphicsCanvasRenderer,
};
pub use hyperlink::Hyperlink;
