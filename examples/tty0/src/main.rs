use std::{cell::RefCell, io, rc::Rc};

use examples_shared::backend::{BackendType, MultiBackendBuilder};
use ratzilla::{
    backend::{postprocessing::PostProcessing, webgl2::WebGl2BackendOptions},
    event::KeyCode,
    CursorShape, WebRenderer,
};

mod app;
mod introstate;
mod logo_text;
mod state;

use app::App;

fn main() -> io::Result<()> {
    let app_state = Rc::new(RefCell::new(App::new()));

    let mut terminal = MultiBackendBuilder::with_fallback(BackendType::WebGl2)
        .webgl2_options(
            WebGl2BackendOptions::new()
                .cursor_shape(CursorShape::SteadyUnderScore)
                .with_render_hook(PostProcessing::default()),
        )
        .build_terminal()?;

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
