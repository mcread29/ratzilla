use std::{cell::RefCell, io, rc::Rc};

use examples_shared::backend::{BackendType, MultiBackendBuilder};
use ratzilla::{
    backend::canvas::CanvasBackendOptions,
    backend::webgl2::WebGl2BackendOptions,
    event::KeyCode,
    widgets::CanvasImageLayer,
    CursorShape, WebRenderer,
};

mod app;
mod introstate;
mod logo_text;
mod postprocessing;
mod shaders;
mod state;

use app::App;
use postprocessing::PostProcessing;

fn main() -> io::Result<()> {
    let image_layer = CanvasImageLayer::new();
    let app_state = Rc::new(RefCell::new(App::new(image_layer.clone())));

    let mut terminal = MultiBackendBuilder::with_fallback(BackendType::WebGl2)
        .canvas_options(CanvasBackendOptions::new().with_render_hook(image_layer.render_hook()))
        .webgl2_options(
            WebGl2BackendOptions::new()
                .cursor_shape(CursorShape::SteadyUnderScore)
                .with_render_hook(image_layer.render_hook())
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
