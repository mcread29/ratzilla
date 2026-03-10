use std::{cell::RefCell, io, rc::Rc};

use examples_shared::backend::{BackendType, MultiBackendBuilder};
use ratzilla::{
    backend::{canvas::CanvasBackendOptions, webgl2::WebGl2BackendOptions},
    event::KeyCode,
    widgets::GraphicsCanvasLayer,
    CursorShape, WebRenderer,
};

mod app;
mod archive;
mod archive_state;
mod audio;
mod help;
mod introstate;
mod logo_text;
mod panel_shader_visualizer;
mod postprocessing;
mod session;
mod session_logs;
mod shaders;
mod state;
mod terminal_state;
mod track_visualizer;

use app::App;
use panel_shader_visualizer::PanelShaderVisualizerLayer;
use postprocessing::PostProcessing;

fn main() -> io::Result<()> {
    let visual_layer = GraphicsCanvasLayer::new();
    let panel_shader_visualizer = PanelShaderVisualizerLayer::new();
    let app_state = Rc::new(RefCell::new(App::new(
        visual_layer.clone(),
        panel_shader_visualizer.clone(),
    )));

    let mut terminal = MultiBackendBuilder::with_fallback(BackendType::WebGl2)
        .canvas_options(
            CanvasBackendOptions::new()
                .with_render_hook(visual_layer.render_hook())
                .with_render_hook(panel_shader_visualizer.render_hook()),
        )
        .webgl2_options(
            WebGl2BackendOptions::new()
                .cursor_shape(CursorShape::SteadyUnderScore)
                .with_render_hook(visual_layer.render_hook())
                .with_render_hook(panel_shader_visualizer.render_hook())
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
