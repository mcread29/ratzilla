use std::{cell::RefCell, rc::Rc};

use crate::{
    archive::{ArchiveRecord, PlaybackAvailability, RecordKind, ARCHIVE},
    session::SessionModel,
    state::{StateActions, StateId, StateMachineError},
};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tachyonfx::Duration;

pub struct ArchiveState {
    session: Rc<RefCell<SessionModel>>,
    pending_transition: Option<StateId>,
}

impl ArchiveState {
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

impl StateActions for ArchiveState {
    fn on_enter(&mut self) -> Result<(), StateMachineError> {
        self.pending_transition = None;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        let mut session = self.session.borrow_mut();
        match key {
            KeyCode::Left => session.move_category(-1, ARCHIVE),
            KeyCode::Right => session.move_category(1, ARCHIVE),
            KeyCode::Up => session.move_record(-1, ARCHIVE),
            KeyCode::Down => session.move_record(1, ARCHIVE),
            KeyCode::Enter => {
                let record = session.current_record(ARCHIVE);
                if !record.locked && !record.placeholder {
                    session.detail_scroll = 0;
                    self.pending_transition = Some(StateId::Record);
                } else {
                    session.decryption_status =
                        "selected object is index-only or sealed // no focused record available";
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                session.open_terminal_from(StateId::Archive);
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
        if area.width < 90 || area.height < 28 {
            self.render_compact(frame, area);
        } else {
            self.render_wide(frame, area);
        }
    }

    fn take_transition(&mut self) -> Option<StateId> {
        self.pending_transition.take()
    }
}

impl ArchiveState {
    fn render_wide(&self, frame: &mut Frame, area: Rect) {
        let root = Block::bordered()
            .title(" tty0 recovered analysis workstation ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = root.inner(area);
        frame.render_widget(root, area);

        let layout = Layout::vertical([Constraint::Min(8), Constraint::Length(3)])
            .margin(1)
            .split(inner);
        let body = Layout::horizontal([
            Constraint::Percentage(28),
            Constraint::Percentage(40),
            Constraint::Percentage(32),
        ])
        .split(layout[0]);

        let left = Layout::vertical([Constraint::Length(10), Constraint::Min(8)]).split(body[0]);
        self.render_categories(frame, left[0]);
        self.render_records(frame, left[1]);
        self.render_preview(frame, body[1]);
        self.render_sidebar(frame, body[2]);
        self.render_status_bar(frame, layout[1]);
    }

    fn render_compact(&self, frame: &mut Frame, area: Rect) {
        let root = Block::bordered()
            .title(" tty0 archive ")
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = root.inner(area);
        frame.render_widget(root, area);

        let layout = Layout::vertical([
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(inner);

        self.render_categories(frame, layout[0]);
        self.render_records(frame, layout[1]);
        self.render_preview(frame, layout[2]);
        self.render_status_bar(frame, layout[3]);
    }

    fn render_categories(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let mut state = ListState::default();
        state.select(Some(session.selected_category));

        let items = ARCHIVE
            .iter()
            .map(|category| {
                let lines = vec![
                    Line::from(Span::styled(
                        category.label,
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        category.description,
                        Style::default().fg(Color::Gray),
                    )),
                ];
                ListItem::new(lines)
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(" archive groups ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_records(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let category = session.current_category(ARCHIVE);
        let mut state = ListState::default();
        state.select(Some(session.current_record_index()));

        let items = category
            .records
            .iter()
            .map(|record| {
                let badge = if record.locked {
                    "LOCKED"
                } else if record.placeholder {
                    "INDEX"
                } else {
                    record.status_label
                };
                ListItem::new(vec![
                    Line::from(Span::styled(
                        record.id,
                        Style::default().fg(Color::LightGreen),
                    )),
                    Line::from(record.title),
                    Line::from(Span::styled(
                        format!("{} // {}", badge, record.timeline),
                        Style::default().fg(Color::Gray),
                    )),
                ])
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(format!(" records // {}", category.path))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let category = session.current_category(ARCHIVE);
        let record = session.current_record(ARCHIVE);
        let block = Block::bordered()
            .title(format!(" dossier // {}", record.id))
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(Span::styled(
                format!("{} [{}]", record.title, kind_label(record.kind)),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                record.subtitle,
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Archive path",
                Style::default().fg(Color::LightCyan),
            )),
            Line::from(category.path),
            Line::from(""),
            Line::from(Span::styled(
                "Summary",
                Style::default().fg(Color::LightCyan),
            )),
            Line::from(record.summary),
            Line::from(""),
            Line::from(Span::styled(
                if record.placeholder {
                    "Recovered index only"
                } else {
                    record.sections[0].title
                },
                Style::default().fg(Color::LightCyan),
            )),
            Line::from(record.sections[0].body),
            Line::from(""),
            Line::from(Span::styled(
                "TTY0 annotation",
                Style::default().fg(Color::LightCyan),
            )),
            Line::from(record.tty0_annotation),
        ];

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, inner);
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let record = session.current_record(ARCHIVE);
        let sidebar = Layout::vertical([
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(7),
        ])
        .split(area);

        self.render_metadata_card(frame, sidebar[0], record);
        self.render_timeline_card(frame, sidebar[1], record);
        self.render_corruption_card(frame, sidebar[2], &session);
        self.render_playback_card(frame, sidebar[3], record);
        self.render_terminal_card(frame, sidebar[4], record, &session);
    }

    fn render_metadata_card(&self, frame: &mut Frame, area: Rect, record: &ArchiveRecord) {
        let lines = vec![
            Line::from(format!("ERROR CODE: {}", record.id)),
            Line::from(format!("INCIDENT NAME: {}", record.title)),
            Line::from(format!("TIMELINE: {}", record.timeline)),
            Line::from(format!("RECOVERED SOURCE: {}", record.recovered_source)),
            Line::from(format!("COLLAPSE VECTOR: {}", record.collapse_vector)),
            Line::from(format!("SIGNAL INTEGRITY: {}", record.signal_integrity)),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" metadata ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_timeline_card(&self, frame: &mut Frame, area: Rect, record: &ArchiveRecord) {
        let lines = vec![
            Line::from(format!("timeline // {}", record.timeline)),
            Line::from(format!("status   // {}", record.status_label)),
            Line::from(format!("source   // {}", record.recovered_source)),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::bordered()
                    .title(" timeline ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_corruption_card(&self, frame: &mut Frame, area: Rect, session: &SessionModel) {
        let lines = vec![
            Line::from(format!("signal drift   // {}", session.corruption_label())),
            Line::from(format!("decrypt state  // {}", session.decryption_status)),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" corruption ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_playback_card(&self, frame: &mut Frame, area: Rect, record: &ArchiveRecord) {
        let PlaybackAvailability::Unavailable {
            audio_source,
            reason,
        } = record.playback;
        let source = audio_source.unwrap_or("none");
        let lines = vec![
            Line::from("transport // disabled"),
            Line::from(format!("source    // {}", source)),
            Line::from(format!("reason    // {}", reason)),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" playback ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_terminal_card(
        &self,
        frame: &mut Frame,
        area: Rect,
        record: &ArchiveRecord,
        session: &SessionModel,
    ) {
        let lines = vec![
            Line::from(Span::styled(
                if session.terminal_locked {
                    "terminal subsystem // LOCKED"
                } else {
                    "terminal subsystem // OPEN"
                },
                Style::default().fg(Color::LightRed),
            )),
            Line::from("visible early, restricted by design"),
            Line::from(""),
            Line::from(record.terminal_hint),
            Line::from(""),
            Line::from("press `t` to inspect denial panel"),
        ];
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" terminal ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let category = session.current_category(ARCHIVE);
        let record = session.current_record(ARCHIVE);
        let text = Line::from(vec![
            Span::styled(
                " Left/Right ",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::raw(" switch group "),
            Span::styled(
                " Up/Down ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ),
            Span::raw(" move record "),
            Span::styled(
                " Enter ",
                Style::default().fg(Color::Black).bg(Color::LightGreen),
            ),
            Span::raw(" inspect recovered record "),
            Span::styled(" T ", Style::default().fg(Color::Black).bg(Color::LightRed)),
            Span::raw(format!(
                " terminal lock // {} // {}",
                category.id.as_slug(),
                record.status_label
            )),
        ]);
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray))),
            area,
        );
    }
}

fn kind_label(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Incident => "incident",
        RecordKind::WitnessTestimony => "witness",
        RecordKind::Transmission => "transmission",
        RecordKind::Analysis => "analysis",
        RecordKind::Residue => "residue",
        RecordKind::PrivateLog => "private",
        RecordKind::Tooling => "tooling",
    }
}
