//! ## Widgets
//!
//! **Ratzilla** provides web-only widgets that you can use while building TUIs.

/// Canvas-first image widget and overlay hook support.
pub mod canvas_image;
/// Canvas-backed procedural graphics widget and overlay hook support.
pub mod graphics_canvas;
pub(crate) mod hyperlink;

pub use canvas_image::{CanvasImage, CanvasImageLayer, ImageCrossOrigin, ImageFit};
pub use graphics_canvas::{
    GraphicsCanvas, GraphicsCanvasContext, GraphicsCanvasLayer, GraphicsCanvasRenderer,
};
pub use hyperlink::Hyperlink;
