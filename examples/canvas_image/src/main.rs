use std::io;

use ratzilla::{
    backend::canvas::CanvasBackendOptions,
    ratatui::{
        layout::{Constraint, Layout},
        style::{Color, Style},
        widgets::Block,
        Terminal,
    },
    widgets::{CanvasImage, CanvasImageLayer, ImageFit},
    CanvasBackend, WebRenderer,
};

const IMAGE_DATA_URL: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='120' viewBox='0 0 180 120'%3E%3Crect width='180' height='120' fill='%23171b22'/%3E%3Ccircle cx='58' cy='56' r='32' fill='%23ffb703' fill-opacity='0.92'/%3E%3Crect x='88' y='28' width='56' height='56' rx='10' fill='%23219653' fill-opacity='0.85'/%3E%3Cpath d='M18 102 L88 42 L138 82 L162 54 L162 102 Z' fill='%23f1f5f9' fill-opacity='0.88'/%3E%3C/svg%3E";

fn main() -> io::Result<()> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let image_layer = CanvasImageLayer::new();
    let backend = CanvasBackend::new_with_options(
        CanvasBackendOptions::new().with_render_hook(image_layer.render_hook()),
    )?;
    let terminal = Terminal::new(backend)?;

    terminal.draw_web(move |frame| {
        let vertical = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(frame.area());
        let horizontal = Layout::horizontal([
            Constraint::Length(8),
            Constraint::Min(24),
            Constraint::Length(8),
        ])
        .split(vertical[1]);
        let panel = horizontal[1];
        let block = Block::bordered()
            .title("Canvas Image")
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(panel);

        frame.render_widget(block, panel);
        frame.render_widget(
            CanvasImage::new(image_layer.clone(), IMAGE_DATA_URL)
                .fit(ImageFit::Contain)
                .style(Style::default().bg(Color::DarkGray))
                .smoothing(true),
            inner,
        );
    });

    Ok(())
}
