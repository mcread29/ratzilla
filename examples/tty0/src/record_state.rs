use std::{cell::RefCell, rc::Rc};

use crate::{
    archive::{ArchiveRecord, PlaybackAvailability, ARCHIVE},
    session::SessionModel,
    state::{StateActions, StateId, StateMachineError},
};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};
use tachyonfx::Duration;

pub struct RecordState {
    session: Rc<RefCell<SessionModel>>,
    pending_transition: Option<StateId>,
    last_record_id: Option<&'static str>,
}

impl RecordState {
    pub fn new(session: Rc<RefCell<SessionModel>>) -> Self {
        Self {
            session,
            pending_transition: None,
            last_record_id: None,
        }
    }

    pub fn create(session: Rc<RefCell<SessionModel>>) -> Box<dyn StateActions> {
        Box::new(Self::new(session))
    }
}

impl StateActions for RecordState {
    fn on_enter(&mut self) -> Result<(), StateMachineError> {
        self.pending_transition = None;
        let mut session = self.session.borrow_mut();
        let record_id = session.current_record(ARCHIVE).id;
        if self.last_record_id != Some(record_id) {
            session.detail_scroll = 0;
            self.last_record_id = Some(record_id);
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        let mut session = self.session.borrow_mut();
        match key {
            KeyCode::Esc | KeyCode::Backspace => {
                self.pending_transition = Some(StateId::Archive);
            }
            KeyCode::Up => session.scroll_detail(-1),
            KeyCode::Down => session.scroll_detail(1),
            KeyCode::Char('t') | KeyCode::Char('T') => {
                session.open_terminal_from(StateId::Record);
                self.pending_transition = Some(StateId::Terminal);
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
            .title(" tty0 focused record ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = root.inner(area);
        frame.render_widget(root, area);

        let layout = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(inner);
        self.render_header(frame, layout[0]);

        let body = if area.width < 100 {
            Layout::vertical([Constraint::Min(10), Constraint::Length(14)]).split(layout[1])
        } else {
            Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1])
        };

        self.render_body(frame, body[0]);
        self.render_sidebar(frame, body[1]);
        self.render_status_bar(frame, layout[2]);
    }

    fn take_transition(&mut self) -> Option<StateId> {
        self.pending_transition.take()
    }
}

impl RecordState {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let category = session.current_category(ARCHIVE);
        let record = session.current_record(ARCHIVE);
        let text = vec![
            Line::from(vec![
                Span::styled(category.label, Style::default().fg(Color::LightCyan)),
                Span::raw(" / "),
                Span::styled(record.id, Style::default().fg(Color::Yellow)),
                Span::raw(" / "),
                Span::styled(record.title, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(format!(
                "{} // {} // {}",
                record.timeline, record.recovered_source, record.signal_integrity
            )),
        ];
        frame.render_widget(
            Paragraph::new(text).block(
                Block::bordered()
                    .title(" breadcrumb ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_body(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let record = session.current_record(ARCHIVE);

        let mut lines = vec![
            Line::from(Span::styled(
                "Synopsis",
                Style::default().fg(Color::LightCyan),
            )),
            Line::from(record.summary),
            Line::from(""),
        ];

        for section in record.sections {
            lines.push(Line::from(Span::styled(
                section.title,
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            )));
            for body_line in section.body.lines() {
                lines.push(Line::from(body_line));
            }
            lines.push(Line::from(""));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((session.detail_scroll as u16, 0))
                .block(
                    Block::bordered()
                        .title(" recovered evidence ")
                        .border_style(Style::default().fg(Color::DarkGray)),
                ),
            area,
        );
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let record = session.current_record(ARCHIVE);
        let blocks = Layout::vertical([
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Min(8),
        ])
        .split(area);

        self.render_metadata(frame, blocks[0], record);
        self.render_transport(frame, blocks[1], record);
        self.render_notes(frame, blocks[2], record, &session);
    }

    fn render_metadata(&self, frame: &mut Frame, area: Rect, record: &ArchiveRecord) {
        let lines = vec![
            Line::from(format!("ERROR CODE: {}", record.id)),
            Line::from(format!("INCIDENT NAME: {}", record.title)),
            Line::from(format!("TIMELINE: {}", record.timeline)),
            Line::from(format!("RECOVERED SOURCE: {}", record.recovered_source)),
            Line::from(format!("COLLAPSE VECTOR: {}", record.collapse_vector)),
            Line::from(format!("SIGNAL INTEGRITY: {}", record.signal_integrity)),
            Line::from(format!("SUMMARY: {}", record.summary)),
            Line::from(format!("TTY0 ANNOTATION: {}", record.tty0_annotation)),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" metadata template ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_transport(&self, frame: &mut Frame, area: Rect, record: &ArchiveRecord) {
        let PlaybackAvailability::Unavailable {
            audio_source,
            reason,
        } = record.playback;
        let lines = vec![
            Line::from("transport // unavailable"),
            Line::from(format!(
                "audio_source // {}",
                audio_source.unwrap_or("none")
            )),
            Line::from(format!("reason // {}", reason)),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" transport ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_notes(
        &self,
        frame: &mut Frame,
        area: Rect,
        record: &ArchiveRecord,
        session: &SessionModel,
    ) {
        let lines = vec![
            Line::from(format!("status // {}", record.status_label)),
            Line::from(format!("decryption // {}", session.decryption_status)),
            Line::from(format!("signal drift // {}", session.corruption_label())),
            Line::from(""),
            Line::from("terminal lock remains visible from this view"),
            Line::from(record.terminal_hint),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" operator notes ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " Esc ",
                    Style::default().fg(Color::Black).bg(Color::LightGreen),
                ),
                Span::raw(" return to archive "),
                Span::styled(
                    " Up/Down ",
                    Style::default().fg(Color::Black).bg(Color::Yellow),
                ),
                Span::raw(" scroll record "),
                Span::styled(" T ", Style::default().fg(Color::Black).bg(Color::LightRed)),
                Span::raw(" inspect locked terminal "),
            ]))
            .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray))),
            area,
        );
    }
}
