use std::{cell::RefCell, fmt, rc::Rc};

use glow;
use web_sys::{CanvasRenderingContext2d, WebGl2RenderingContext};

use crate::error::Error;

/// Rendering backend kind associated with hook execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendKind {
    /// DOM backend.
    Dom,
    /// Canvas 2D backend.
    Canvas,
    /// WebGL2 backend.
    WebGl2,
}

/// Context provided to render hooks during each flush.
///
/// Size values are backend-dependent:
/// - WebGl2: render target/canvas buffer dimensions used for rendering.
/// - Canvas and Dom: display (CSS/logical) dimensions at flush time.
/// Hook implementations should account for this when doing pixel-sensitive math.
#[derive(Clone, Copy)]
pub struct RenderHookContext<'a> {
    backend: BackendKind,
    canvas_width: i32,
    canvas_height: i32,
    cell_size: Option<(i32, i32)>,
    canvas_2d_context: Option<&'a CanvasRenderingContext2d>,
    webgl2_context: Option<&'a WebGl2RenderingContext>,
    webgl_context: Option<&'a glow::Context>,
}

impl<'a> RenderHookContext<'a> {
    /// Creates a hook context for a backend flush.
    pub fn new(backend: BackendKind, canvas_width: i32, canvas_height: i32) -> Self {
        Self {
            backend,
            canvas_width,
            canvas_height,
            cell_size: None,
            canvas_2d_context: None,
            webgl2_context: None,
            webgl_context: None,
        }
    }

    /// Attaches optional physical cell size capability.
    pub fn with_cell_size(mut self, cell_width: i32, cell_height: i32) -> Self {
        self.cell_size = Some((cell_width, cell_height));
        self
    }

    /// Attaches an optional Canvas 2D context capability.
    pub fn with_canvas_2d_context(
        mut self,
        canvas_2d_context: &'a CanvasRenderingContext2d,
    ) -> Self {
        self.canvas_2d_context = Some(canvas_2d_context);
        self
    }

    /// Attaches an optional raw WebGL2 context capability.
    pub fn with_webgl2_context(mut self, webgl2_context: &'a WebGl2RenderingContext) -> Self {
        self.webgl2_context = Some(webgl2_context);
        self
    }

    /// Attaches an optional WebGL context capability.
    pub fn with_webgl_context(mut self, webgl_context: &'a glow::Context) -> Self {
        self.webgl_context = Some(webgl_context);
        self
    }

    /// Returns the current backend kind.
    pub fn backend(&self) -> BackendKind {
        self.backend
    }

    /// Returns backend-reported render/display dimensions.
    pub fn canvas_size(&self) -> (i32, i32) {
        (self.canvas_width, self.canvas_height)
    }

    /// Returns physical cell size when available.
    pub fn cell_size(&self) -> Option<(i32, i32)> {
        self.cell_size
    }

    /// Returns Canvas 2D context capability when available.
    pub fn canvas_2d_context(&self) -> Option<&'a CanvasRenderingContext2d> {
        self.canvas_2d_context
    }

    /// Returns raw WebGL2 context capability when available.
    pub fn webgl2_context(&self) -> Option<&'a WebGl2RenderingContext> {
        self.webgl2_context
    }

    /// Returns WebGL context capability when available.
    pub fn webgl_context(&self) -> Option<&'a glow::Context> {
        self.webgl_context
    }
}

/// Hook trait for custom render steps around backend flush.
pub trait RenderHook {
    /// Called before a backend flush render pass.
    fn pre_render(&mut self, _context: &RenderHookContext<'_>) -> Result<(), Error> {
        Ok(())
    }

    /// Called after a backend flush render pass.
    fn post_render(&mut self, _context: &RenderHookContext<'_>) -> Result<(), Error> {
        Ok(())
    }
}

/// Shared render hook handle for backend option structs.
#[derive(Clone)]
pub struct RenderHookHandle {
    hook: Rc<RefCell<dyn RenderHook>>,
}

impl RenderHookHandle {
    /// Wraps a render hook for backend options.
    pub fn new<H>(hook: H) -> Self
    where
        H: RenderHook + 'static,
    {
        Self {
            hook: Rc::new(RefCell::new(hook)),
        }
    }
}

impl RenderHook for RenderHookHandle {
    fn pre_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        self.hook.borrow_mut().pre_render(context)
    }

    fn post_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
        self.hook.borrow_mut().post_render(context)
    }
}

impl fmt::Debug for RenderHookHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RenderHookHandle(..)")
    }
}

pub(crate) fn run_pre_render_hooks(
    render_hooks: &[RenderHookHandle],
    context: RenderHookContext<'_>,
) -> Result<(), Error> {
    for hook in render_hooks {
        hook.hook.borrow_mut().pre_render(&context)?;
    }

    Ok(())
}

pub(crate) fn run_post_render_hooks(
    render_hooks: &[RenderHookHandle],
    context: RenderHookContext<'_>,
) -> Result<(), Error> {
    for hook in render_hooks {
        hook.hook.borrow_mut().post_render(&context)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    struct RecordingHook {
        calls: Rc<RefCell<Vec<(bool, BackendKind, (i32, i32))>>>,
    }

    impl RenderHook for RecordingHook {
        fn pre_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
            self.calls
                .borrow_mut()
                .push((true, context.backend(), context.canvas_size()));
            Ok(())
        }

        fn post_render(&mut self, context: &RenderHookContext<'_>) -> Result<(), Error> {
            self.calls
                .borrow_mut()
                .push((false, context.backend(), context.canvas_size()));
            Ok(())
        }
    }

    #[test]
    fn runs_hooks_for_each_backend_kind() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let hook = RecordingHook {
            calls: calls.clone(),
        };
        let handle = RenderHookHandle::new(hook);
        let hooks = vec![handle];

        let dom = RenderHookContext::new(BackendKind::Dom, 80, 24);
        let canvas = RenderHookContext::new(BackendKind::Canvas, 120, 40);
        let webgl = RenderHookContext::new(BackendKind::WebGl2, 160, 50);

        run_pre_render_hooks(&hooks, dom).expect("dom pre hook should run");
        run_post_render_hooks(&hooks, dom).expect("dom post hook should run");
        run_pre_render_hooks(&hooks, canvas).expect("canvas pre hook should run");
        run_post_render_hooks(&hooks, canvas).expect("canvas post hook should run");
        run_pre_render_hooks(&hooks, webgl).expect("webgl pre hook should run");
        run_post_render_hooks(&hooks, webgl).expect("webgl post hook should run");

        let calls = calls.borrow();

        assert_eq!(calls.len(), 6);
        assert_eq!(calls[0], (true, BackendKind::Dom, (80, 24)));
        assert_eq!(calls[1], (false, BackendKind::Dom, (80, 24)));
        assert_eq!(calls[2], (true, BackendKind::Canvas, (120, 40)));
        assert_eq!(calls[3], (false, BackendKind::Canvas, (120, 40)));
        assert_eq!(calls[4], (true, BackendKind::WebGl2, (160, 50)));
        assert_eq!(calls[5], (false, BackendKind::WebGl2, (160, 50)));
    }

    #[test]
    fn returns_attached_cell_size() {
        let context = RenderHookContext::new(BackendKind::WebGl2, 160, 50).with_cell_size(9, 18);
        assert_eq!(context.cell_size(), Some((9, 18)));
    }
}

#[cfg(test)]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;
    use web_sys::{wasm_bindgen::JsCast, window, HtmlCanvasElement};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn returns_attached_canvas_2d_context() {
        let document = window()
            .expect("window should exist")
            .document()
            .expect("document should exist");
        let canvas = document
            .create_element("canvas")
            .expect("canvas element should be created")
            .dyn_into::<HtmlCanvasElement>()
            .expect("element should be canvas");
        let context = canvas
            .get_context("2d")
            .expect("context lookup should succeed")
            .expect("2d context should exist")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("context should be 2d");

        let hook_context =
            RenderHookContext::new(BackendKind::Canvas, 100, 40).with_canvas_2d_context(&context);

        assert!(hook_context.canvas_2d_context().is_some());
    }
}
