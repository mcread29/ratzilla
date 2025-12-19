use ratzilla::event::KeyCode;
use ratzilla::ratatui::Frame;
use tachyonfx::Duration;
use thiserror::Error;

use std::collections::HashMap;

#[derive(Error, Debug)]
pub enum StateMachineError {
    // #[error("")]
    #[error("State {0} not found")]
    StateNotFound(String),
    #[error("Statemachine is not running")]
    StatemachineNotRunning,
}

pub trait StateActions {
    // fn new(state_machine: &'a mut StateMachine) -> Self
    // where
    //     Self: Sized;
    fn can_enter_state(&self) -> Result<bool, StateMachineError>;
    fn enter_state(&self) -> Result<(), StateMachineError>;
    fn should_exit_state(&self) -> Result<&str, StateMachineError>;
    fn can_exit_state(&self) -> Result<bool, StateMachineError>;
    fn exit_state(&self) -> Result<(), StateMachineError>;
    fn update_state(&mut self, elapsed: Duration) -> Result<(), StateMachineError>;
    fn render_state(&self, frame: &mut Frame);
    fn key_press(&mut self, key: KeyCode) -> Result<(), StateMachineError>;
}

pub struct StateMachine {
    pub states: HashMap<String, Box<dyn StateActions>>,
    pub current_state: String,
    pub running: bool,
}

impl StateMachine {
    pub fn new(states: HashMap<String, Box<dyn StateActions>>, default_state: String) -> Self {
        Self {
            states,
            current_state: default_state,
            running: false,
        }
    }

    pub fn current_state(&self) -> Result<&Box<dyn StateActions>, StateMachineError> {
        self.states
            .get(&self.current_state)
            .ok_or(StateMachineError::StateNotFound(self.current_state.clone()))
    }

    pub fn current_state_mut(&mut self) -> Result<&mut Box<dyn StateActions>, StateMachineError> {
        self.states
            .get_mut(&self.current_state)
            .ok_or(StateMachineError::StateNotFound(self.current_state.clone()))
    }

    pub fn key_press(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        self.current_state_mut()?.key_press(key)?;
        Ok(())
    }

    pub fn get_state(&self, state: &str) -> Result<&Box<dyn StateActions>, StateMachineError> {
        self.states
            .get(state)
            .ok_or(StateMachineError::StateNotFound(state.to_string()))
    }

    pub fn init(&mut self) -> Result<(), StateMachineError> {
        self.running = true;
        self.current_state()?.enter_state()?;
        Ok(())
    }

    pub fn try_change_state(&mut self, state: &str) -> Result<bool, StateMachineError> {
        if !self.running {
            return Err(StateMachineError::StatemachineNotRunning);
        }

        if self.current_state.clone() == state.to_string() {
            return Ok(false);
        }

        let current_state = self.current_state()?;
        if !current_state.can_exit_state()? {
            return Ok(false);
        }

        let next_state = self.get_state(state)?;
        if !next_state.can_enter_state()? {
            return Ok(false);
        }

        current_state.exit_state()?;
        next_state.enter_state()?;
        self.current_state = state.to_string();
        Ok(true)
    }

    pub fn update_statemachine(
        &mut self,
        elapsed: Duration,
        frame: &mut Frame,
    ) -> Result<(), StateMachineError> {
        if !self.running {
            return Err(StateMachineError::StatemachineNotRunning);
        }

        let exit_state = {
            let current_state = self.current_state().unwrap();
            let exit_state = current_state.should_exit_state()?;
            if !exit_state.is_empty() && current_state.can_exit_state()? {
                Some(exit_state.to_string())
            } else {
                None
            }
        };

        if let Some(exit_state) = exit_state {
            self.try_change_state(&exit_state)?;
            return Ok(());
        }

        let current_state = self.current_state_mut()?;
        current_state.update_state(elapsed)?;
        current_state.render_state(frame);
        Ok(())
    }
}
