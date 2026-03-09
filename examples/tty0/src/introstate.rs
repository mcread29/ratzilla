use std::{cell::RefCell, rc::Rc};

use crate::{
    logo_text::render_logo_text,
    session::SessionModel,
    state::{StateActions, StateId, StateMachineError},
};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tachyonfx::Duration;

const BOOT: &str = r#"
carrier lock acquired
signal origin .......... unresolved
session owner .......... mismatch
account override ....... tty0
archive mount .......... /recovered/humanity
integrity check ........ partial
decryption key ......... active signal stream
decryption progress .... 0.0000%
note ................... progress may remain truthful at 0%
terminal subsystem ..... present / locked
index warmup ........... witnesses, transmissions, residues
handoff ................ unauthorized reader granted provisional access
"#;

const COMPLETE_TEXT: &str = "press any key to mount archive workstation";

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }
}

struct LineByLine<'a> {
    blink_speed: Duration,
    last_blink: web_time::Instant,
    time_per_line: Duration,
    last_line_time: web_time::Instant,
    lines: StatefulList<&'a str>,
    current_line: usize,
    complete_text: &'a str,
    cursor_on: bool,
}

impl<'a> LineByLine<'a> {
    fn new(lines: &'a str, cursor_on: bool, complete_text: &'a str) -> Self {
        Self {
            blink_speed: Duration::from_millis(900),
            last_blink: web_time::Instant::now(),
            time_per_line: Duration::from_millis(130),
            last_line_time: web_time::Instant::now(),
            lines: StatefulList::with_items(lines.lines().collect()),
            current_line: 0,
            cursor_on,
            complete_text,
        }
    }

    fn percentage_displayed(&self) -> f64 {
        let total = self.lines.items.len() as f64;
        if total == 0.0 {
            return 0.0;
        }
        (self.current_line as f64 / total).clamp(0.0, 1.0)
    }

    fn is_complete(&self) -> bool {
        self.current_line >= self.lines.items.len()
    }

    fn skip_to_end(&mut self) {
        self.current_line = self.lines.items.len();
        self.lines
            .state
            .select(self.lines.items.len().checked_sub(1));
    }

    fn up_to_current_line(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

        let mut list: Vec<ListItem> = self
            .lines
            .items
            .iter()
            .take(self.current_line)
            .map(|line| ListItem::new(vec![Line::from(Span::raw(*line))]))
            .collect();

        if self.is_complete() {
            let prompt = if self.cursor_on {
                format!("{} _", self.complete_text)
            } else {
                self.complete_text.to_string()
            };
            list.push(ListItem::new(vec![Line::from(Span::styled(
                prompt,
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ))]));
        }

        let text = List::new(list)
            .block(Block::new())
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_stateful_widget(text, chunks[0], &mut self.lines.state);

        let gauge = Gauge::default()
            .block(Block::new())
            .gauge_style(Style::default().fg(Color::LightRed).bg(Color::Black))
            .use_unicode(true)
            .ratio(self.percentage_displayed());
        frame.render_widget(gauge, chunks[1]);
    }

    fn update(&mut self, _elapsed: Duration) {
        let now = web_time::Instant::now();
        let elapsed = now.duration_since(self.last_line_time);
        if elapsed.as_millis() as u32 > self.time_per_line.as_millis() && !self.is_complete() {
            self.current_line += 1;
            self.last_line_time = now;
        }

        let blink_elapsed = web_time::Instant::now().duration_since(self.last_blink);
        if blink_elapsed.as_millis() as u32 > self.blink_speed.as_millis() {
            self.cursor_on = !self.cursor_on;
            self.last_blink = web_time::Instant::now();
        }

        self.lines.state.select(if self.current_line == 0 {
            None
        } else {
            Some(self.current_line.saturating_sub(1))
        });
    }
}

pub struct IntroState {
    line_by_line: LineByLine<'static>,
    pending_transition: Option<StateId>,
    session: Rc<RefCell<SessionModel>>,
}

impl IntroState {
    pub fn new(session: Rc<RefCell<SessionModel>>) -> Self {
        Self {
            line_by_line: LineByLine::new(BOOT, true, COMPLETE_TEXT),
            pending_transition: None,
            session,
        }
    }

    pub fn create(session: Rc<RefCell<SessionModel>>) -> Box<dyn StateActions> {
        Box::new(IntroState::new(session))
    }
}

impl StateActions for IntroState {
    fn on_enter(&mut self) -> Result<(), StateMachineError> {
        self.pending_transition = None;
        self.session.borrow_mut().decryption_status =
            "signal stream attached // archive mounted // decryption stalled at 0%";
        Ok(())
    }

    fn handle_key(&mut self, _key: KeyCode) -> Result<(), StateMachineError> {
        if self.line_by_line.is_complete() {
            self.pending_transition = Some(StateId::Archive);
        } else {
            self.line_by_line.skip_to_end();
        }
        Ok(())
    }

    fn update(&mut self, elapsed: Duration) -> Result<(), StateMachineError> {
        self.line_by_line.update(elapsed);
        self.session.borrow_mut().tick(elapsed);
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let outer = Block::bordered()
            .title(" tty0 boot handoff ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);

        let layout = Layout::vertical([Constraint::Length(9), Constraint::Min(8)])
            .margin(1)
            .split(inner);
        let logo_area = render_logo_text(frame, layout[0]);

        let summary = Paragraph::new(vec![
            Line::from("LOCAL USER SESSION WAS REPLACED BY REMOTE ARCHIVE OWNER"),
            Line::from("AUTHORITY: tty0"),
            Line::from("SUBSYSTEMS: archive online • terminal locked • media corruption expected"),
        ])
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Rgb(196, 214, 198)));
        frame.render_widget(summary, logo_area);

        self.line_by_line.up_to_current_line(frame, layout[1]);
    }

    fn take_transition(&mut self) -> Option<StateId> {
        self.pending_transition.take()
    }
}
