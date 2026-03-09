use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    archive::ArchiveLoader,
    archive_state::ArchiveState,
    introstate::IntroState,
    session::SessionModel,
    state::{StateId, StateMachine},
    terminal_state::TerminalState,
};
use ratzilla::{event::KeyCode, ratatui::Frame};

pub struct App {
    state_machine: Option<StateMachine>,
    last_frame: web_time::Instant,
}

impl App {
    pub fn new() -> Self {
        let archive = Rc::new(
            ArchiveLoader::load_embedded().expect("tty0 embedded archive must load successfully"),
        );
        let session = Rc::new(RefCell::new(SessionModel::new(archive)));

        let mut states = HashMap::new();
        states.insert(StateId::Intro, IntroState::create(Rc::clone(&session)));
        states.insert(StateId::Archive, ArchiveState::create(Rc::clone(&session)));
        states.insert(StateId::Terminal, TerminalState::create(session));

        let mut state_machine = StateMachine::new(states, StateId::Intro);
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
