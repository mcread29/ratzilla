//! ## Widgets
//!
//! **Ratzilla** provides web-only widgets that you can use while building TUIs.

/// Canvas-first image widget and overlay hook support.
pub mod canvas_image;
pub(crate) mod hyperlink;

pub use canvas_image::{CanvasImage, CanvasImageLayer, ImageCrossOrigin, ImageFit};
pub use hyperlink::Hyperlink;
