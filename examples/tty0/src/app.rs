use std::collections::HashMap;

use crate::{introstate::IntroState, state::StateMachine};
use ratzilla::{event::KeyCode, ratatui::Frame};

pub struct App {
    state_machine: Option<StateMachine>,
    last_frame: web_time::Instant,
}

impl App {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        states.insert("default".to_string(), IntroState::create());

        let mut state_machine = StateMachine::new(states, "default".to_string());
        state_machine
            .init()
            .expect("state machine init must succeed");

        Self {
            state_machine: Some(state_machine),
            last_frame: web_time::Instant::now(),
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
    }
}
