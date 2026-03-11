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
mod overlay_state;
mod panel_shader_visualizer;
mod postprocessing;
mod session;
mod session_logs;
mod shaders;
mod state;
mod terminal_state;
mod track_visualizer;
mod visualizer_editor;
mod visualizer_sequence;

use app::App;
use overlay_state::OverlayRenderState;
use panel_shader_visualizer::PanelShaderVisualizerLayer;
use postprocessing::PostProcessing;

fn main() -> io::Result<()> {
    let visual_layer = GraphicsCanvasLayer::new();
    let panel_shader_visualizer = PanelShaderVisualizerLayer::new();
    let overlay_state = OverlayRenderState::default();
    let app_state = Rc::new(RefCell::new(App::new(
        visual_layer.clone(),
        panel_shader_visualizer.clone(),
        overlay_state.clone(),
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
                .with_render_hook(PostProcessing::new(overlay_state)),
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
