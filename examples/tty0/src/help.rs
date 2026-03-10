use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
    Frame,
};
use web_sys::{window, Storage};

use crate::state::StateId;

const STORAGE_KEY: &str = "tty0.help_seen.v1";
const TEXT: Color = Color::Rgb(196, 214, 198);
const DIM: Color = Color::Rgb(118, 128, 120);
const BORDER: Color = Color::DarkGray;
const CYAN: Color = Color::Rgb(110, 220, 212);
const AMBER: Color = Color::Rgb(234, 182, 92);
const RED: Color = Color::Rgb(240, 104, 96);
const PANEL: Color = Color::Rgb(8, 12, 10);

pub enum HelpKeyOutcome {
    Consumed,
    PassThrough,
}

pub struct HelpOverlay {
    is_open: bool,
    pending_first_visit_help: bool,
    has_seen_help: bool,
    storage: Option<Storage>,
}

impl HelpOverlay {
    pub fn new() -> Self {
        let storage = window().and_then(|window| window.local_storage().ok().flatten());
        let has_seen_help = storage
            .as_ref()
            .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
            .as_deref()
            == Some("1");

        Self {
            is_open: false,
            pending_first_visit_help: !has_seen_help,
            has_seen_help,
            storage,
        }
    }

    #[cfg(test)]
    fn with_seen_flag(has_seen_help: bool) -> Self {
        Self {
            is_open: false,
            pending_first_visit_help: !has_seen_help,
            has_seen_help,
            storage: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn open(&mut self) {
        self.mark_seen();
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn handle_key(&mut self, key: &KeyCode) -> HelpKeyOutcome {
        if self.is_open {
            return match key {
                KeyCode::Esc => {
                    self.close();
                    HelpKeyOutcome::Consumed
                }
                KeyCode::Char('h') | KeyCode::Char('H') => HelpKeyOutcome::Consumed,
                _ => HelpKeyOutcome::Consumed,
            };
        }

        match key {
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.open();
                HelpKeyOutcome::Consumed
            }
            _ => HelpKeyOutcome::PassThrough,
        }
    }

    pub fn maybe_open_for_state(&mut self, current_state: StateId) {
        if self.pending_first_visit_help && current_state == StateId::Archive {
            self.open();
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = modal_rect(frame.area());
        let wide = frame.area().width >= 110 && frame.area().height >= 34;
        let block = Block::bordered()
            .title(" archive help ")
            .border_type(if wide {
                BorderType::Double
            } else {
                BorderType::Plain
            })
            .style(Style::default().bg(PANEL))
            .border_style(Style::default().fg(BORDER));

        let lines = vec![
            section_heading("purpose"),
            body_line("This workstation was mounted by tty0 as a recovered analysis surface."),
            body_line("It is an evidence browser for extinction records, not a general shell."),
            body_line("Some surfaces remain partial, corrupted, or withheld on purpose."),
            blank_line(),
            section_heading("how to read"),
            body_line("Left side: record index and recovered shelves."),
            body_line("Center: selected record, page tabs, and active document surface."),
            body_line("Right side: metadata, playback state, and live archive residue."),
            body_line("Mounted tracks may expose reactive visual evidence on the media page."),
            body_line("Read each record as evidence, not as a complete archive."),
            blank_line(),
            section_heading("navigation"),
            key_line("↑ / ↓", "select records"),
            key_line("← / →", "change record page tabs"),
            key_line("P", "play or pause mounted audio"),
            key_line("H", "reopen this help"),
            key_line("Esc", "close help"),
            blank_line(),
            note_line(
                "boot",
                "On the handoff screen, any key advances once the handoff is ready. H opens help without advancing.",
            ),
        ];

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(TEXT).bg(PANEL)),
            inner_modal_rect(area),
        );
    }

    fn mark_seen(&mut self) {
        if self.has_seen_help {
            self.pending_first_visit_help = false;
            return;
        }

        self.has_seen_help = true;
        self.pending_first_visit_help = false;

        if let Some(storage) = &self.storage {
            let _ = storage.set_item(STORAGE_KEY, "1");
        }
    }
}

fn modal_rect(area: Rect) -> Rect {
    if area.width < 76 || area.height < 20 {
        return Rect::new(
            area.x.saturating_add(1),
            area.y.saturating_add(1),
            area.width.saturating_sub(2).max(1),
            area.height.saturating_sub(2).max(1),
        );
    }

    let width = area.width.min(76).max(1);
    let height = area.height.min(18).max(1);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

fn inner_modal_rect(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(2),
        area.y.saturating_add(1),
        area.width.saturating_sub(4).max(1),
        area.height.saturating_sub(2).max(1),
    )
}

fn section_heading(label: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        label.to_ascii_uppercase(),
        Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
    ))
}

fn body_line(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(text, Style::default().fg(TEXT)))
}

fn key_line(key: &'static str, detail: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<8}"), Style::default().fg(AMBER)),
        Span::styled(detail, Style::default().fg(TEXT)),
    ])
}

fn note_line(label: &'static str, detail: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<8}"), Style::default().fg(RED)),
        Span::styled(detail, Style::default().fg(DIM)),
    ])
}

fn blank_line() -> Line<'static> {
    Line::from("")
}

#[cfg(test)]
mod tests {
    use super::{HelpKeyOutcome, HelpOverlay};
    use crate::state::StateId;
    use ratzilla::event::KeyCode;

    #[test]
    fn first_run_starts_pending() {
        let overlay = HelpOverlay::with_seen_flag(false);

        assert!(!overlay.is_open());
        assert!(overlay.pending_first_visit_help);
    }

    #[test]
    fn stored_flag_disables_first_visit_open() {
        let overlay = HelpOverlay::with_seen_flag(true);

        assert!(!overlay.pending_first_visit_help);
    }

    #[test]
    fn manual_open_marks_seen_and_clears_pending() {
        let mut overlay = HelpOverlay::with_seen_flag(false);

        assert!(matches!(
            overlay.handle_key(&KeyCode::Char('h')),
            HelpKeyOutcome::Consumed
        ));

        assert!(overlay.is_open());
        assert!(overlay.has_seen_help);
        assert!(!overlay.pending_first_visit_help);
    }

    #[test]
    fn escape_closes_help() {
        let mut overlay = HelpOverlay::with_seen_flag(false);
        overlay.open();

        assert!(matches!(
            overlay.handle_key(&KeyCode::Esc),
            HelpKeyOutcome::Consumed
        ));
        assert!(!overlay.is_open());
    }

    #[test]
    fn non_help_keys_are_swallowed_when_open() {
        let mut overlay = HelpOverlay::with_seen_flag(false);
        overlay.open();

        assert!(matches!(
            overlay.handle_key(&KeyCode::Left),
            HelpKeyOutcome::Consumed
        ));
        assert!(overlay.is_open());
    }

    #[test]
    fn first_visit_auto_opens_only_in_archive() {
        let mut overlay = HelpOverlay::with_seen_flag(false);

        overlay.maybe_open_for_state(StateId::Intro);
        assert!(!overlay.is_open());

        overlay.maybe_open_for_state(StateId::Archive);
        assert!(overlay.is_open());
        assert!(!overlay.pending_first_visit_help);
    }
}
