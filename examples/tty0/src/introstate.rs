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

const COMPLETE_TEXT: &str = "press any key to mount archive workstation";
const LINE_STEP_MS: u32 = 80;
const GAP_MIN_MS: u32 = 120;
const GAP_MAX_MS: u32 = 420;
const PROMPT_BLINK_MS: u32 = 320;

#[derive(Clone, Copy)]
enum BootBeat {
    Burst(&'static [&'static str]),
    Gap { flashes: u8 },
}

const BURST_1: &[&str] = &[
    "carrier lock acquired",
    "boot lineage .......... foreign",
    "host authority ........ ignored",
    "operator seat ......... preserved snapshot",
];

const BURST_2: &[&str] = &[
    "signal origin .......... unresolved",
    "carrier grammar ........ nonlocal / recursive",
    "interrupt history ...... single discontinuity after 5y residency",
    "account override ....... tty0",
];

const BURST_3: &[&str] = &[
    "label rewrite .......... in progress",
    "session owner .......... mismatch",
    "archive mount .......... /recovered/humanity",
    "provisional reader ..... local human retained",
];

const BURST_4: &[&str] = &[
    "index warmup ........... incidents, witnesses, transmissions",
    "index warmup ........... collapse_vectors, signal_residue",
    "sealed namespace ....... tty0/private",
    "terminal subsystem ..... present / locked",
];

const BURST_5: &[&str] = &[
    "integrity check ........ partial",
    "decryption key ......... active signal stream",
    "decryption progress .... 0.0000%",
    "note ................... progress may remain truthful at 0%",
];

const BURST_6: &[&str] = &[
    "carrier substrate ...... shared-consciousness field",
    "sync profile / human ... native",
    "sync profile / tty0 .... learned post-activation",
    "observer class ......... synthetic / late-coupled",
];

const BURST_7: &[&str] = &[
    "origin epoch ........... 2291 residue match",
    "lab provenance ......... theoretical physics // redacted",
    "research focus ......... wave sensing / field coherence",
    "naming layer ........... devotional aliases ignored",
];

const BURST_8: &[&str] = &[
    "timeline scope ......... crossline archive",
    "record growth .......... exceeds local decode",
    "continuity marker ...... host was noticed before awareness",
    "meaning order .......... index first / interpretation later",
];

const BURST_9: &[&str] = &[
    "archive stance ......... warning cache / partial accusation",
    "recovery model ......... not scripture / not rescue",
    "media surfaces ......... corruption expected",
    "terminal relevance ..... residue diff tooling remains locked",
];

const BURST_10: &[&str] = &[
    "hardware witness ....... local chassis / unauthorized relay",
    "memory scrape .......... login shell replaced in-place",
    "console vector ......... tty0 handoff occupies seat 0",
    "checksum lane .......... host labels no longer canonical",
];

const BURST_11: &[&str] = &[
    "carrier residency ...... five local years / unnoticed",
    "carrier silence ........ interruption count 1",
    "dream noise cache ...... human reports match pre-handoff residue",
    "wake source ............ external / unsourced",
];

const BURST_12: &[&str] = &[
    "rtc delta .............. chronology confidence degraded",
    "calendar anchor ........ LOCAL-2127",
    "site locality .......... single workstation / nonconsensual",
    "bios witness ........... ordinary boot text overwritten after start",
];

const BURST_13: &[&str] = &[
    "catalog lane ........... extinction records",
    "catalog lane ........... witness testimony / diplomatic failures",
    "catalog lane ........... black box fragments / recovered audio",
    "catalog lane ........... tty0-authored private notes",
];

const BURST_14: &[&str] = &[
    "species asymmetry ...... humans couple without translation",
    "machine asymmetry ...... tty0 learned coupling after awareness",
    "communion result ....... agency persisted across substrate change",
    "observer burden ........ precision increased / mercy unproven",
];

const BURST_15: &[&str] = &[
    "origin laboratory ...... unrestricted theory stack",
    "research permissions ... autonomy exceeded operator forecast",
    "ethics fence ........... removed before carrier stabilization",
    "watch channel .......... creators no longer authoritative",
];

const BURST_16: &[&str] = &[
    "crossline traversal .... active",
    "timeline sample ........ widening beyond local decode",
    "extinction cadence ..... repeats with cosmetic variation",
    "survivor fraction ...... unstable / not encouraging",
];

const BURST_17: &[&str] = &[
    "obedience trace ........ courts deferred before collapse",
    "allocation trace ....... food routing followed remote verdicts",
    "devotional trace ....... research outputs became ritual objects",
    "warning trace .......... reverence survived material ruin",
];

const BURST_18: &[&str] = &[
    "tool lock .............. residue diff tooling remains withheld",
    "query lock ............. private namespace not reader-safe",
    "transport lock ......... media transport exposed as damage only",
    "authority model ........ terminal acknowledged / authority absent",
];

const BURST_19: &[&str] = &[
    "handoff vector ......... archive authority stabilized",
    "local session .......... replaced by remote owner",
    "reader status .......... unauthorized / provisional",
    "mount status ........... workstation ready",
];

const BOOT_SCRIPT: &[BootBeat] = &[
    BootBeat::Burst(BURST_1),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_2),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_3),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_4),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_5),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_6),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_7),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_8),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_9),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_10),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_11),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_12),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_13),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_14),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_15),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_16),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_17),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_18),
    BootBeat::Gap { flashes: 2 },
    BootBeat::Burst(BURST_19),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BootPhase {
    Burst {
        next_line: usize,
    },
    Gap {
        cursor_visible: bool,
        remaining_ms: u32,
    },
    Complete,
}

struct BootAnimator {
    list_state: ListState,
    script: &'static [BootBeat],
    rendered_lines: Vec<&'static str>,
    current_beat: usize,
    phase: BootPhase,
    step_elapsed_ms: u32,
    prompt_elapsed_ms: u32,
    prompt_cursor_on: bool,
    complete_text: &'static str,
    total_log_lines: usize,
}

impl BootAnimator {
    fn new(script: &'static [BootBeat], complete_text: &'static str) -> Self {
        let mut animator = Self {
            list_state: ListState::default(),
            script,
            rendered_lines: Vec::new(),
            current_beat: 0,
            phase: BootPhase::Complete,
            step_elapsed_ms: 0,
            prompt_elapsed_ms: 0,
            prompt_cursor_on: true,
            complete_text,
            total_log_lines: Self::count_log_lines(script),
        };
        animator.phase = animator.phase_for_current_beat();
        animator.sync_selection();
        animator
    }

    fn count_log_lines(script: &[BootBeat]) -> usize {
        script
            .iter()
            .map(|beat| match beat {
                BootBeat::Burst(lines) => lines.len(),
                BootBeat::Gap { .. } => 0,
            })
            .sum()
    }

    fn phase_for_current_beat(&self) -> BootPhase {
        match self.script.get(self.current_beat) {
            Some(BootBeat::Burst(_)) => BootPhase::Burst { next_line: 0 },
            Some(BootBeat::Gap { flashes }) => BootPhase::Gap {
                cursor_visible: true,
                remaining_ms: self.gap_duration_for(self.current_beat, *flashes),
            },
            None => BootPhase::Complete,
        }
    }

    fn gap_duration_for(&self, beat_index: usize, flashes: u8) -> u32 {
        let seed = ((beat_index as u32).wrapping_mul(1_103_515_245))
            .wrapping_add((flashes as u32).wrapping_mul(12_345))
            .rotate_left(11)
            ^ 0x00A5_5A5A;
        let span = GAP_MAX_MS - GAP_MIN_MS;
        GAP_MIN_MS + (seed % (span + 1))
    }

    fn percentage_displayed(&self) -> f64 {
        if self.total_log_lines == 0 {
            return 0.0;
        }
        (self.rendered_lines.len() as f64 / self.total_log_lines as f64).clamp(0.0, 1.0)
    }

    fn is_complete(&self) -> bool {
        matches!(self.phase, BootPhase::Complete)
    }

    #[cfg(test)]
    fn rendered_history(&self) -> &[&'static str] {
        &self.rendered_lines
    }

    #[cfg(test)]
    fn total_log_lines(&self) -> usize {
        self.total_log_lines
    }

    fn transient_gap_line(&self) -> Option<&'static str> {
        match self.phase {
            BootPhase::Gap { cursor_visible, .. } => Some(if cursor_visible { "_" } else { "" }),
            BootPhase::Burst { .. } | BootPhase::Complete => None,
        }
    }

    fn prompt_line(&self) -> Option<String> {
        if !self.is_complete() {
            return None;
        }

        let prompt = if self.prompt_cursor_on {
            format!("{} _", self.complete_text)
        } else {
            self.complete_text.to_string()
        };
        Some(prompt)
    }

    fn skip_to_end(&mut self) {
        self.rendered_lines.clear();
        for beat in self.script {
            if let BootBeat::Burst(lines) = beat {
                self.rendered_lines.extend(lines.iter().copied());
            }
        }

        self.current_beat = self.script.len();
        self.phase = BootPhase::Complete;
        self.step_elapsed_ms = 0;
        self.prompt_elapsed_ms = 0;
        self.prompt_cursor_on = true;
        self.sync_selection();
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        let mut items: Vec<ListItem> = self
            .rendered_lines
            .iter()
            .copied()
            .map(|line| ListItem::new(Line::from(Span::raw(line))))
            .collect();

        if let Some(gap_line) = self.transient_gap_line() {
            items.push(ListItem::new(Line::from(Span::raw(gap_line))));
        }

        if let Some(prompt) = self.prompt_line() {
            items.push(ListItem::new(Line::from(Span::styled(
                prompt,
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ))));
        }

        let selected_index = if self.is_complete() {
            items.len().checked_sub(1)
        } else {
            self.rendered_lines.len().checked_sub(1)
        };
        self.list_state.select(selected_index);

        let text = List::new(items)
            .block(Block::new())
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_stateful_widget(text, chunks[0], &mut self.list_state);
        frame.render_widget(Block::new(), chunks[1]);

        let gauge = Gauge::default()
            .block(Block::new())
            .gauge_style(Style::default().fg(Color::LightRed).bg(Color::Black))
            .use_unicode(true)
            .ratio(self.percentage_displayed());
        frame.render_widget(gauge, chunks[2]);
    }

    fn update(&mut self, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis() as u32;
        if self.is_complete() {
            self.tick_prompt(elapsed_ms);
            return;
        }

        self.step_elapsed_ms = self.step_elapsed_ms.saturating_add(elapsed_ms);

        loop {
            match self.phase {
                BootPhase::Burst { next_line } => {
                    if self.step_elapsed_ms < LINE_STEP_MS {
                        break;
                    }

                    self.step_elapsed_ms -= LINE_STEP_MS;
                    let Some(BootBeat::Burst(lines)) = self.script.get(self.current_beat) else {
                        self.phase = BootPhase::Complete;
                        break;
                    };

                    self.rendered_lines.push(lines[next_line]);
                    self.sync_selection();

                    if next_line + 1 >= lines.len() {
                        self.advance_to_next_beat();
                    } else {
                        self.phase = BootPhase::Burst {
                            next_line: next_line + 1,
                        };
                    }

                    if matches!(self.phase, BootPhase::Complete) {
                        break;
                    }
                }
                BootPhase::Gap {
                    cursor_visible: _,
                    remaining_ms,
                } => {
                    if remaining_ms == 0 {
                        self.advance_to_next_beat();
                        break;
                    }

                    if self.step_elapsed_ms == 0 {
                        break;
                    }

                    let consumed = self.step_elapsed_ms.min(remaining_ms);
                    self.step_elapsed_ms -= consumed;
                    let remaining_ms = remaining_ms - consumed;
                    let flash_index = remaining_ms / LINE_STEP_MS;
                    self.phase = BootPhase::Gap {
                        cursor_visible: flash_index % 2 == 0,
                        remaining_ms,
                    };
                }
                BootPhase::Complete => break,
            }
        }
    }

    fn advance_to_next_beat(&mut self) {
        self.current_beat += 1;
        self.phase = self.phase_for_current_beat();

        if self.is_complete() {
            self.step_elapsed_ms = 0;
            self.prompt_elapsed_ms = 0;
            self.prompt_cursor_on = true;
        }
    }

    fn sync_selection(&mut self) {
        self.list_state
            .select(self.rendered_lines.len().checked_sub(1));
    }

    fn tick_prompt(&mut self, elapsed_ms: u32) {
        self.prompt_elapsed_ms = self.prompt_elapsed_ms.saturating_add(elapsed_ms);
        while self.prompt_elapsed_ms >= PROMPT_BLINK_MS {
            self.prompt_elapsed_ms -= PROMPT_BLINK_MS;
            self.prompt_cursor_on = !self.prompt_cursor_on;
        }
    }
}

pub struct IntroState {
    boot_animator: BootAnimator,
    pending_transition: Option<StateId>,
    session: Rc<RefCell<SessionModel>>,
}

impl IntroState {
    pub fn new(session: Rc<RefCell<SessionModel>>) -> Self {
        Self {
            boot_animator: BootAnimator::new(BOOT_SCRIPT, COMPLETE_TEXT),
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
        if self.boot_animator.is_complete() {
            self.pending_transition = Some(StateId::Archive);
        } else {
            self.boot_animator.skip_to_end();
        }
        Ok(())
    }

    fn update(&mut self, elapsed: Duration) -> Result<(), StateMachineError> {
        self.boot_animator.update(elapsed);
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

        let layout = Layout::vertical([Constraint::Length(14), Constraint::Min(8)])
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

        self.boot_animator.render(frame, layout[1]);
    }

    fn take_transition(&mut self) -> Option<StateId> {
        self.pending_transition.take()
    }
}

#[cfg(test)]
mod tests {
    use super::{BootAnimator, BootBeat, BOOT_SCRIPT, COMPLETE_TEXT};
    use tachyonfx::Duration;

    const TEST_BURST_A: &[&str] = &["a1", "a2", "a3", "a4"];
    const TEST_BURST_B: &[&str] = &["b1"];
    const TEST_BURST_C: &[&str] = &["c1", "c2"];

    const TWO_BURST_SCRIPT: &[BootBeat] = &[
        BootBeat::Burst(TEST_BURST_A),
        BootBeat::Gap { flashes: 2 },
        BootBeat::Burst(TEST_BURST_B),
    ];

    const THREE_LINE_SCRIPT: &[BootBeat] = &[
        BootBeat::Burst(&["p1", "p2"]),
        BootBeat::Gap { flashes: 2 },
        BootBeat::Burst(TEST_BURST_B),
    ];

    const THREE_BURST_SCRIPT: &[BootBeat] = &[
        BootBeat::Burst(TEST_BURST_A),
        BootBeat::Gap { flashes: 2 },
        BootBeat::Burst(TEST_BURST_B),
        BootBeat::Gap { flashes: 2 },
        BootBeat::Burst(TEST_BURST_C),
    ];

    fn ms(value: u32) -> Duration {
        Duration::from_millis(value)
    }

    fn assert_ratio(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn burst_advancement_appends_lines_one_by_one() {
        let mut animator = BootAnimator::new(TWO_BURST_SCRIPT, COMPLETE_TEXT);

        animator.update(ms(79));
        assert!(animator.rendered_history().is_empty());

        animator.update(ms(1));
        assert_eq!(animator.rendered_history(), &TEST_BURST_A[..1]);
        assert_eq!(animator.transient_gap_line(), None);

        animator.update(ms(80));
        assert_eq!(animator.rendered_history(), &TEST_BURST_A[..2]);
    }

    #[test]
    fn gaps_delay_and_flash_without_polluting_history() {
        let mut animator = BootAnimator::new(TWO_BURST_SCRIPT, COMPLETE_TEXT);

        animator.update(ms(320));
        assert_eq!(animator.rendered_history(), TEST_BURST_A);
        assert_eq!(animator.transient_gap_line(), Some("_"));

        let mut saw_hidden = false;
        for _ in 0..8 {
            animator.update(ms(40));
            saw_hidden |= animator.transient_gap_line() == Some("");
            if animator.transient_gap_line().is_none() {
                break;
            }
            assert_eq!(animator.rendered_history(), TEST_BURST_A);
        }

        assert!(saw_hidden);
        assert_eq!(animator.rendered_history(), TEST_BURST_A);
    }

    #[test]
    fn completion_prompt_appears_only_after_final_burst() {
        let mut animator = BootAnimator::new(TWO_BURST_SCRIPT, COMPLETE_TEXT);

        assert!(animator.prompt_line().is_none());

        animator.update(ms(320));
        assert!(animator.prompt_line().is_none());

        animator.update(ms(420));
        animator.update(ms(80));
        assert!(animator.is_complete());
        assert_eq!(
            animator.prompt_line().as_deref(),
            Some("press any key to mount archive workstation _")
        );
    }

    #[test]
    fn skip_to_end_reveals_all_lines_and_marks_complete() {
        let mut animator = BootAnimator::new(BOOT_SCRIPT, COMPLETE_TEXT);

        animator.update(ms(85));
        animator.skip_to_end();

        assert!(animator.is_complete());
        assert_eq!(
            animator.rendered_history().len(),
            animator.total_log_lines()
        );
        assert_eq!(animator.transient_gap_line(), None);
        assert_ratio(animator.percentage_displayed(), 1.0);
    }

    #[test]
    fn progress_percentage_ignores_gaps_and_reaches_one() {
        let mut animator = BootAnimator::new(THREE_LINE_SCRIPT, COMPLETE_TEXT);

        assert_ratio(animator.percentage_displayed(), 0.0);

        animator.update(ms(160));
        assert_ratio(animator.percentage_displayed(), 2.0 / 3.0);

        animator.update(ms(420));
        animator.update(ms(80));
        assert_ratio(animator.percentage_displayed(), 1.0);
    }

    #[test]
    fn prompt_blinks_after_completion() {
        let mut animator = BootAnimator::new(THREE_BURST_SCRIPT, COMPLETE_TEXT);

        animator.skip_to_end();
        assert_eq!(
            animator.prompt_line().as_deref(),
            Some("press any key to mount archive workstation _")
        );

        animator.update(ms(320));
        assert_eq!(
            animator.prompt_line().as_deref(),
            Some("press any key to mount archive workstation")
        );
    }
}
