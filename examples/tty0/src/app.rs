use crate::state::{StateMachine, StateMachineError};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::Frame;
use std::collections::HashMap;
use tachyonfx::Duration;

use crate::introstate::IntroState;

pub struct App {
    pub state_machine: Option<StateMachine>,
    pub last_frame: web_time::Instant,
}

impl App {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        states.insert("default".to_string(), IntroState::create());
        let mut state_machine = StateMachine::new(states, "default".to_string());
        state_machine.init().unwrap();

        Self {
            state_machine: Some(state_machine),
            last_frame: web_time::Instant::now(),
        }
    }

    pub fn key_press(&mut self, key_code: KeyCode) -> Result<(), StateMachineError> {
        if let Some(state_machine) = &mut self.state_machine {
            state_machine.key_press(key_code)?;
        }
        Ok(())
    }

    pub fn update(&mut self, frame: &mut Frame) -> Result<(), StateMachineError> {
        let now = web_time::Instant::now();
        let elapsed_ms = now.duration_since(self.last_frame).as_millis() as u32;
        let elapsed = Duration::from_millis(elapsed_ms);
        self.last_frame = now;

        if let Some(state_machine) = &mut self.state_machine {
            state_machine.update_statemachine(elapsed, frame)?;
        }
        Ok(())
    }
}
