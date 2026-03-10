use std::collections::HashMap;

use ratzilla::event::KeyCode;
use ratzilla::ratatui::Frame;
use tachyonfx::Duration;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StateId {
    Intro,
    Archive,
    Terminal,
}

#[derive(Error, Debug)]
pub enum StateMachineError {
    #[error("state {0:?} not found")]
    StateNotFound(StateId),
    #[error("state machine is not running")]
    StatemachineNotRunning,
}

pub trait StateActions {
    fn on_enter(&mut self) -> Result<(), StateMachineError> {
        Ok(())
    }

    fn handle_key(&mut self, _key: KeyCode) -> Result<(), StateMachineError> {
        Ok(())
    }

    fn update(&mut self, _elapsed: Duration) -> Result<(), StateMachineError> {
        Ok(())
    }

    fn render(&mut self, _frame: &mut Frame) {}

    fn take_transition(&mut self) -> Option<StateId> {
        None
    }
}

pub struct StateMachine {
    pub states: HashMap<StateId, Box<dyn StateActions>>,
    pub current_state: StateId,
    pub running: bool,
}

impl StateMachine {
    pub fn new(states: HashMap<StateId, Box<dyn StateActions>>, default_state: StateId) -> Self {
        Self {
            states,
            current_state: default_state,
            running: false,
        }
    }

    pub fn current_state_mut(&mut self) -> Result<&mut Box<dyn StateActions>, StateMachineError> {
        self.states
            .get_mut(&self.current_state)
            .ok_or(StateMachineError::StateNotFound(self.current_state))
    }

    pub fn init(&mut self) -> Result<(), StateMachineError> {
        self.running = true;
        self.current_state_mut()?.on_enter()?;
        Ok(())
    }

    pub fn key_press(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        self.current_state_mut()?.handle_key(key)
    }

    fn try_change_state(&mut self, state: StateId) -> Result<bool, StateMachineError> {
        if !self.running {
            return Err(StateMachineError::StatemachineNotRunning);
        }

        if self.current_state == state {
            return Ok(false);
        }

        self.current_state = state;
        self.current_state_mut()?.on_enter()?;
        Ok(true)
    }

    fn flush_transition(&mut self) -> Result<(), StateMachineError> {
        let next_state = self.current_state_mut()?.take_transition();
        if let Some(next_state) = next_state {
            self.try_change_state(next_state)?;
        }
        Ok(())
    }

    pub fn update_statemachine(
        &mut self,
        elapsed: Duration,
        frame: &mut Frame,
    ) -> Result<(), StateMachineError> {
        if !self.running {
            return Err(StateMachineError::StatemachineNotRunning);
        }

        self.flush_transition()?;
        self.current_state_mut()?.update(elapsed)?;
        self.flush_transition()?;
        self.current_state_mut()?.render(frame);
        Ok(())
    }
}
