use std::collections::HashMap;

use crate::{introstate::IntroState, state::StateMachine};
use ratzilla::{
    event::KeyCode,
    ratatui::{
        layout::Rect,
        style::{Color, Style},
        widgets::Block,
        Frame,
    },
    widgets::{CanvasImage, CanvasImageLayer, ImageFit},
};

const TEST_IMAGE_URL: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='120' viewBox='0 0 180 120'%3E%3Crect width='180' height='120' fill='%23171b22'/%3E%3Ccircle cx='58' cy='56' r='32' fill='%23ffb703' fill-opacity='0.92'/%3E%3Crect x='88' y='28' width='56' height='56' rx='10' fill='%23219653' fill-opacity='0.85'/%3E%3Cpath d='M18 102 L88 42 L138 82 L162 54 L162 102 Z' fill='%23f1f5f9' fill-opacity='0.88'/%3E%3C/svg%3E";

pub struct App {
    state_machine: Option<StateMachine>,
    last_frame: web_time::Instant,
    image_layer: CanvasImageLayer,
}

impl App {
    pub fn new(image_layer: CanvasImageLayer) -> Self {
        let mut states = HashMap::new();
        states.insert("default".to_string(), IntroState::create());

        let mut state_machine = StateMachine::new(states, "default".to_string());
        state_machine
            .init()
            .expect("state machine init must succeed");

        Self {
            state_machine: Some(state_machine),
            last_frame: web_time::Instant::now(),
            image_layer,
        }
    }

    pub fn key_press(&mut self, key: KeyCode) {
        if let Some(state_machine) = &mut self.state_machine {
            let _ = state_machine.key_press(key);
        }
    }

    pub fn update(&mut self, frame: &mut Frame) {
        let now = web_time::Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_millis() as u32;
        self.last_frame = now;

        if let Some(state_machine) = &mut self.state_machine {
            let _ =
                state_machine.update_statemachine(tachyonfx::Duration::from_millis(elapsed), frame);
        }

        self.render_canvas_image_test(frame);
    }

    fn render_canvas_image_test(&self, frame: &mut Frame) {
        let area = frame.area();
        if area.width < 34 || area.height < 14 {
            return;
        }

        let panel = Rect::new(
            area.right().saturating_sub(30),
            area.y.saturating_add(1),
            28,
            10,
        );
        let block = Block::bordered()
            .title("IMG TEST")
            .style(Style::default().bg(Color::Black))
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(panel);

        frame.render_widget(block, panel);
        frame.render_widget(
            CanvasImage::new(self.image_layer.clone(), TEST_IMAGE_URL)
                .fit(ImageFit::Contain)
                .style(Style::default().bg(Color::DarkGray))
                .smoothing(true),
            inner,
        );
    }
}
