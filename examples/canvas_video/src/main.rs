use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
};

use examples_shared::backend::{BackendType, MultiBackendBuilder};
use ratzilla::{
    backend::{canvas::CanvasBackendOptions, webgl2::WebGl2BackendOptions},
    event::KeyCode,
    ratatui::{
        layout::{Constraint, Layout},
        style::{Color, Style},
        text::Line,
        widgets::{Block, Paragraph},
    },
    widgets::{CanvasVideo, CanvasVideoLayer, ImageFit, VideoPreload},
    WebRenderer,
};
use wasm_bindgen_futures::spawn_local;

const VIDEO_ID: &str = "sample";
const VIDEO_SRC: &str = "assets/sample.mp4";

fn main() -> io::Result<()> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let layer = CanvasVideoLayer::new();
    let handle = layer.handle(VIDEO_ID);
    let muted = Rc::new(Cell::new(false));
    let control_error = Rc::new(RefCell::new(None::<String>));

    let mut terminal = MultiBackendBuilder::with_fallback(BackendType::WebGl2)
        .canvas_options(
            CanvasBackendOptions::new()
                .grid_id("terminal")
                .with_render_hook(layer.render_hook()),
        )
        .webgl2_options(
            WebGl2BackendOptions::new()
                .grid_id("terminal")
                .with_render_hook(layer.render_hook()),
        )
        .build_terminal()?;

    terminal.on_key_event({
        let handle = handle.clone();
        let muted = Rc::clone(&muted);
        let control_error = Rc::clone(&control_error);
        move |event| match event.code {
            KeyCode::Char(' ') => {
                let handle = handle.clone();
                let control_error = Rc::clone(&control_error);
                spawn_local(async move {
                    if let Err(error) = handle.toggle().await {
                        *control_error.borrow_mut() = Some(error.to_string());
                    }
                });
            }
            KeyCode::Left | KeyCode::Right => {
                let delta = if event.code == KeyCode::Left {
                    -5.0
                } else {
                    5.0
                };
                let result = handle
                    .current_time()
                    .and_then(|time| handle.seek((time + delta).max(0.0)));
                if let Err(error) = result {
                    *control_error.borrow_mut() = Some(error.to_string());
                }
            }
            KeyCode::Char('m' | 'M') => {
                let next = !muted.get();
                muted.set(next);
                if let Err(error) = handle.set_muted(next) {
                    *control_error.borrow_mut() = Some(error.to_string());
                }
            }
            _ => {}
        }
    })?;

    terminal.draw_web(move |frame| {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(frame.area());

        frame.render_widget(
            Paragraph::new("CanvasVideo — focus this terminal and press Space to start playback")
                .block(Block::bordered().title("Direct media")),
            rows[0],
        );

        let panel = Block::bordered()
            .title("assets/sample.mp4")
            .border_style(Style::default().fg(Color::Cyan));
        let video_area = panel.inner(rows[1]);
        frame.render_widget(panel, rows[1]);
        frame.render_widget(
            CanvasVideo::new(layer.clone(), VIDEO_ID, VIDEO_SRC)
                .fit(ImageFit::Contain)
                .preload(VideoPreload::Metadata)
                .plays_inline(true)
                .muted(muted.get())
                .autoplay(false)
                .style(Style::default().bg(Color::Black)),
            video_area,
        );

        let status = match handle.error() {
            Ok(Some(error)) => format!(
                "media error {}: {} (is the local sample present?)",
                error.code, error.message
            ),
            Err(error) => format!("loading: {error}"),
            Ok(None) => {
                let time = handle.current_time().unwrap_or(0.0);
                let duration = handle
                    .duration()
                    .ok()
                    .flatten()
                    .map(|value| format!("{value:.1}"))
                    .unwrap_or_else(|| "--".into());
                let state = if handle.ended().unwrap_or(false) {
                    "ended"
                } else if handle.paused().unwrap_or(true) {
                    "paused"
                } else {
                    "playing"
                };
                format!(
                    "{state}  {time:.1}/{duration}s  muted: {}  ready: {}",
                    muted.get(),
                    handle.ready().unwrap_or(false)
                )
            }
        };
        let error = control_error
            .borrow()
            .clone()
            .unwrap_or_else(|| "none".into());
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Space play/pause  ←/→ seek 5s  M mute"),
                Line::from(status),
                Line::from(format!("last control error: {error}")),
            ])
            .block(Block::bordered().title("Controls / status")),
            rows[2],
        );
    });

    Ok(())
}
