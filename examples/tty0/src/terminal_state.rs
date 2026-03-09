use std::{cell::RefCell, rc::Rc};

use crate::{
    archive::ARCHIVE,
    session::SessionModel,
    state::{StateActions, StateId, StateMachineError},
};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};
use tachyonfx::Duration;

pub struct TerminalState {
    session: Rc<RefCell<SessionModel>>,
    pending_transition: Option<StateId>,
}

impl TerminalState {
    pub fn new(session: Rc<RefCell<SessionModel>>) -> Self {
        Self {
            session,
            pending_transition: None,
        }
    }

    pub fn create(session: Rc<RefCell<SessionModel>>) -> Box<dyn StateActions> {
        Box::new(Self::new(session))
    }
}

impl StateActions for TerminalState {
    fn on_enter(&mut self) -> Result<(), StateMachineError> {
        self.pending_transition = None;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        match key {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace => {
                let return_state = self.session.borrow().terminal_return_state;
                self.pending_transition = Some(return_state);
            }
            _ => {}
        }
        Ok(())
    }

    fn update(&mut self, elapsed: Duration) -> Result<(), StateMachineError> {
        self.session.borrow_mut().tick(elapsed);
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let root = Block::bordered()
            .title(" restricted subsystem ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::LightRed));
        let inner = root.inner(area);
        frame.render_widget(root, area);

        let layout = Layout::vertical([
            Constraint::Length(6),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(inner);

        let session = self.session.borrow();
        let record = session.current_record(ARCHIVE);
        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                "TERMINAL ACCESS DENIED",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("subsystem id // tty0.term.lock"),
            Line::from("reason // archive visible before shell authority is granted"),
        ])
        .block(
            Block::bordered()
                .title(" denial ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(header, layout[0]);

        let body = Paragraph::new(vec![
            Line::from(
                "The terminal exists in this workstation as a boundary, not as a fake prompt.",
            ),
            Line::from("No unlock logic is implemented in this build."),
            Line::from(""),
            Line::from(Span::styled(
                "Current record hint",
                Style::default().fg(Color::LightCyan),
            )),
            Line::from(record.terminal_hint),
            Line::from(""),
            Line::from(Span::styled(
                "Why it is visible",
                Style::default().fg(Color::LightCyan),
            )),
            Line::from("The archive wants the operator to know deeper subsystems exist."),
            Line::from("The archive does not yet trust the operator with them."),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::bordered()
                .title(" subsystem notice ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(body, layout[1]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(
                " Esc ",
                Style::default().fg(Color::Black).bg(Color::LightGreen),
            ),
            Span::raw(" return "),
            Span::styled(
                " Enter ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ),
            Span::raw(" acknowledge denial "),
            Span::styled(
                " Backspace ",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::raw(format!(" previous view // {}", session.corruption_label())),
        ]))
        .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(footer, layout[2]);
    }

    fn take_transition(&mut self) -> Option<StateId> {
        self.pending_transition.take()
    }
}
