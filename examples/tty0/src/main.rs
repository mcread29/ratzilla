use std::{cell::RefCell, io, rc::Rc};

use examples_shared::backend::{BackendType, MultiBackendBuilder};
use ratzilla::{
    backend::{canvas::CanvasBackendOptions, webgl2::WebGl2BackendOptions},
    event::KeyCode,
    CursorShape, WebRenderer,
};

mod app;
mod archive;
mod archive_state;
mod audio;
mod help;
mod introstate;
mod logo_text;
mod postprocessing;
mod session;
mod session_logs;
mod shaders;
mod state;
mod terminal_state;

use app::App;
use postprocessing::PostProcessing;

fn main() -> io::Result<()> {
    let app_state = Rc::new(RefCell::new(App::new()));

    let mut terminal = MultiBackendBuilder::with_fallback(BackendType::WebGl2)
        .canvas_options(CanvasBackendOptions::new())
        .webgl2_options(
            WebGl2BackendOptions::new()
                .cursor_shape(CursorShape::SteadyUnderScore)
                .with_render_hook(PostProcessing::default()),
        )
        .build_terminal()?;
    remove_backend_footer_in_release();

    terminal.on_key_event({
        let app_state = app_state.clone();
        move |event| {
            let mut app = app_state.borrow_mut();
            app.key_press(match event.code {
                KeyCode::Char(c) => KeyCode::Char(c),
                other => other,
            });
        }
    })?;

    terminal.draw_web(move |frame| {
        let mut app = app_state.borrow_mut();
        app.update(frame);
    });

    Ok(())
}

fn remove_backend_footer_in_release() {
    #[cfg(not(debug_assertions))]
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        if let Some(footer) = document.get_element_by_id("ratzilla-backend-footer") {
            footer.remove();
        }
    }
}
