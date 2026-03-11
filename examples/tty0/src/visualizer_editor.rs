use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    archive::{
        ChromaticBulgeGridAutomation, ChromaticBulgeGridAutomationLanes,
        ChromaticBulgeGridResolvedState, ChromaticBulgeGridShaderState,
        ChromaticBulgeGridShaderStates, ColorKeyframe, FloatKeyframe, InterpolationMode,
        PlaybackClock, RecordDocument, TrackVisualizerConfig, TrackVisualizerMode,
    },
    session::SessionModel,
};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use serde_json::to_string_pretty;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{window, RequestInit, RequestMode, Response};

const BG: Color = Color::Black;
const TEXT: Color = Color::Rgb(196, 214, 198);
const DIM: Color = Color::Rgb(118, 128, 120);
const BORDER: Color = Color::DarkGray;
const CYAN: Color = Color::Rgb(110, 220, 212);
const AMBER: Color = Color::Rgb(234, 182, 92);
const RED: Color = Color::Rgb(240, 104, 96);
const GREEN: Color = Color::Rgb(130, 208, 132);
const WHITE: Color = Color::Rgb(232, 238, 232);

const MIN_EDITOR_WIDTH: u16 = 120;
const MIN_EDITOR_HEIGHT: u16 = 34;
const SAVE_BRIDGE_URL: &str = "http://127.0.0.1:4777/save_visualizer";

pub struct VisualizerEditorOverlay {
    enabled: bool,
    is_open: bool,
    drafts: EditorDraftStore,
    focus_area: EditorFocusArea,
    tab: EditorTab,
    state_target: StateTarget,
    state_field_index: usize,
    lane_index: usize,
    keyframe_index: usize,
    inspector_field: InspectorField,
    numeric_input: Option<NumericInputState>,
    status_message: Rc<RefCell<Option<String>>>,
}

pub struct EditorDraftStore {
    drafts: HashMap<String, ChromaticBulgeGridEditorDraft>,
}

#[derive(Clone)]
pub struct ChromaticBulgeGridEditorDraft {
    pub record_id: String,
    pub original: TrackVisualizerConfig,
    pub working: TrackVisualizerConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorFocusArea {
    StateTarget,
    StateParams,
    TimelineLanes,
    TimelineGraph,
    TimelineInspector,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTab {
    State,
    Timeline,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateTarget {
    Playing,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorField {
    Bpm,
    Measures,
    Beat,
    Value,
    Interpolation,
    Red,
    Green,
    Blue,
}

#[derive(Clone, Debug)]
pub struct NumericInputState {
    pub target: NumericEditTarget,
    pub buffer: String,
    pub replace_on_type: bool,
}

#[derive(Clone, Debug)]
pub enum NumericEditTarget {
    StateField(ParameterField),
    Bpm,
    Measures,
    KeyframeBeat(LaneId),
    KeyframeValue(LaneId),
    KeyframeColor(LaneId, usize),
}

impl NumericEditTarget {
    fn label(&self) -> &'static str {
        match self {
            Self::StateField(field) => field.label(),
            Self::Bpm => "bpm",
            Self::Measures => "measures",
            Self::KeyframeBeat(_) => "beat",
            Self::KeyframeValue(lane) => lane.label(),
            Self::KeyframeColor(lane, channel) => match (lane, channel) {
                (LaneId::ColdColor, 0) => "cold_color.r",
                (LaneId::ColdColor, 1) => "cold_color.g",
                (LaneId::ColdColor, _) => "cold_color.b",
                (LaneId::HotColor, 0) => "hot_color.r",
                (LaneId::HotColor, 1) => "hot_color.g",
                (LaneId::HotColor, _) => "hot_color.b",
                _ => lane.label(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterField {
    MotionRate,
    LatticeDensity,
    CircleRadius,
    CircleFalloffStart,
    CircleFalloffEnd,
    BulgeAmount,
    RimGuard,
    RimExponent,
    RimWarp,
    SpacingMaxPx,
    SpacingMinPx,
    DotSize,
    OuterDotScale,
    EdgeSoftness,
    ChromaticAberration,
    ScrollBase,
    ScrollMotionScale,
    ScrollMotionFloor,
    ScrollMotionCeiling,
    ColdColorR,
    ColdColorG,
    ColdColorB,
    HotColorR,
    HotColorG,
    HotColorB,
    ColorCycleRate,
    InnerAlpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneId {
    MotionRate,
    LatticeDensity,
    CircleRadius,
    CircleFalloffStart,
    CircleFalloffEnd,
    BulgeAmount,
    RimGuard,
    RimExponent,
    RimWarp,
    SpacingMaxPx,
    SpacingMinPx,
    DotSize,
    OuterDotScale,
    EdgeSoftness,
    ChromaticAberration,
    ScrollBase,
    ScrollMotionScale,
    ScrollMotionFloor,
    ScrollMotionCeiling,
    ColdColor,
    HotColor,
    ColorCycleRate,
    InnerAlpha,
}

impl VisualizerEditorOverlay {
    pub fn new() -> Self {
        Self {
            enabled: editor_flag_enabled(),
            is_open: false,
            drafts: EditorDraftStore {
                drafts: HashMap::new(),
            },
            focus_area: EditorFocusArea::StateParams,
            tab: EditorTab::State,
            state_target: StateTarget::Playing,
            state_field_index: 0,
            lane_index: 0,
            keyframe_index: 0,
            inspector_field: InspectorField::Beat,
            numeric_input: None,
            status_message: Rc::new(RefCell::new(None)),
        }
    }

    pub fn preview_visualizer(&self, record: &RecordDocument) -> Option<TrackVisualizerConfig> {
        self.drafts
            .draft_for(record.id.as_str())
            .map(|draft| draft.working.clone())
            .or_else(|| record.visualizer().cloned())
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn toggle_for_record(&mut self, record: &RecordDocument) {
        if !self.enabled {
            return;
        }

        if self.is_open {
            self.is_open = false;
            self.numeric_input = None;
            return;
        }

        let Some(config) = record.visualizer() else {
            self.set_status("selected record has no visualizer");
            return;
        };
        if config.mode != TrackVisualizerMode::ChromaticBulgeGrid {
            self.set_status("editor only supports chromatic bulge grid");
            return;
        }

        self.drafts.ensure(record);
        self.is_open = true;
        self.focus_area = EditorFocusArea::TimelineLanes;
        self.tab = EditorTab::Timeline;
        self.numeric_input = None;
        self.sync_selection(record.id.as_str());
    }

    pub fn handle_key(&mut self, key: KeyCode, session: &mut SessionModel) -> bool {
        if !self.is_open {
            return false;
        }

        let record_id = session.current_record().id.clone();
        if self.numeric_input.is_some() {
            return self.handle_numeric_key(key, &record_id);
        }

        match key {
            KeyCode::Esc => {
                self.is_open = false;
                true
            }
            KeyCode::Tab => {
                self.focus_area = next_focus_area(self.tab, self.focus_area);
                true
            }
            KeyCode::Char('1') => {
                self.tab = EditorTab::State;
                self.focus_area = EditorFocusArea::StateParams;
                true
            }
            KeyCode::Char('2') => {
                self.tab = EditorTab::Timeline;
                self.focus_area = EditorFocusArea::TimelineLanes;
                true
            }
            KeyCode::Char('3') => {
                self.tab = EditorTab::Export;
                self.focus_area = EditorFocusArea::Export;
                true
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                let should_play = !session.playback_clock().is_playing;
                session.set_playback(should_play);
                true
            }
            KeyCode::Left => {
                self.scrub_beats(session, -1.0);
                true
            }
            KeyCode::Right => {
                self.scrub_beats(session, 1.0);
                true
            }
            KeyCode::Char('[') => {
                self.scrub_measures(session, -1.0);
                true
            }
            KeyCode::Char(']') => {
                self.scrub_measures(session, 1.0);
                true
            }
            _ => self.handle_non_numeric_key(key, session, &record_id),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, session: &SessionModel) {
        if !self.is_open {
            return;
        }

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(6, 10, 12))),
            area,
        );
        let overlay = Block::bordered()
            .title(" tty0 shader track editor ")
            .border_type(BorderType::Double)
            .style(Style::default().bg(Color::Rgb(10, 14, 16)))
            .border_style(Style::default().fg(CYAN));
        let inner = overlay.inner(area);
        frame.render_widget(overlay, area);

        if area.width < MIN_EDITOR_WIDTH || area.height < MIN_EDITOR_HEIGHT {
            frame.render_widget(
                Paragraph::new("editor requires a wider viewport in dev mode")
                    .style(Style::default().fg(AMBER).bg(BG))
                    .block(Block::default()),
                inner,
            );
            return;
        }

        let Some(draft) = self.drafts.draft_for(session.current_record().id.as_str()) else {
            frame.render_widget(
                Paragraph::new("editor draft unavailable").style(Style::default().fg(RED).bg(BG)),
                inner,
            );
            return;
        };

        let layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(inner);
        self.render_header(frame, layout[0], draft, session);
        self.render_tabs(frame, layout[1]);
        match self.tab {
            EditorTab::State => self.render_state_tab(frame, layout[2], draft),
            EditorTab::Timeline => self.render_timeline_tab(frame, layout[2], draft, session),
            EditorTab::Export => self.render_export_tab(frame, layout[2], draft),
        }
        self.render_footer(frame, layout[3], draft);
    }

    fn render_header(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let playback = session.playback_clock();
        let automation = draft.automation();
        let authored = automation.duration_secs();
        let mismatch = playback
            .duration_secs
            .map(|duration| (duration - authored).abs())
            .unwrap_or_default();
        let warning = if mismatch > 1.5 {
            format!("  drift warning {:.1}s", mismatch)
        } else {
            String::new()
        };
        let lines = vec![Line::from(vec![
            Span::styled(draft.record_id.as_str(), Style::default().fg(CYAN)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled(
                if draft.is_dirty() { "dirty" } else { "clean" },
                Style::default().fg(if draft.is_dirty() { AMBER } else { GREEN }),
            ),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "time {:>5.1}s beat {:>5.2} measure {:>4.2}  bpm {:.1}  {} bars{}",
                    playback.current_time_secs,
                    current_beat(playback, automation),
                    current_measure(playback, automation),
                    automation.bpm,
                    automation.measures,
                    warning
                ),
                Style::default().fg(TEXT),
            ),
        ])];

        frame.render_widget(
            Paragraph::new(lines)
                .style(Style::default().fg(TEXT).bg(Color::Rgb(10, 14, 16)))
                .block(Block::bordered().border_style(Style::default().fg(BORDER))),
            area,
        );
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let labels = [
            ("1 State", EditorTab::State),
            ("2 Timeline", EditorTab::Timeline),
            ("3 Export", EditorTab::Export),
        ]
        .into_iter()
        .map(|(label, tab)| {
            let selected = self.tab == tab;
            Span::styled(
                format!(" {} ", label),
                Style::default()
                    .fg(if selected { BG } else { TEXT })
                    .bg(if selected {
                        CYAN
                    } else {
                        Color::Rgb(18, 24, 22)
                    })
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )
        })
        .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(Line::from(labels))
                .style(Style::default().bg(Color::Rgb(10, 14, 16)))
                .block(Block::bordered().border_style(Style::default().fg(BORDER))),
            area,
        );
    }

    fn render_state_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let layout = Layout::horizontal([Constraint::Length(24), Constraint::Fill(1)]).split(area);
        let target_items = vec![ListItem::new("playing base"), ListItem::new("idle")];
        let mut target_state = ListState::default();
        target_state.select(Some(match self.state_target {
            StateTarget::Playing => 0,
            StateTarget::Idle => 1,
        }));
        frame.render_stateful_widget(
            List::new(target_items)
                .block(
                    Block::bordered()
                        .title(" state ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::StateTarget {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                )
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(Style::default().bg(Color::Rgb(18, 35, 33))),
            layout[0],
            &mut target_state,
        );

        let state = draft.state(self.state_target);
        let items = ParameterField::ALL
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let value = format_parameter_value(state, *field);
                let selected = index == self.state_field_index;
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<22}", field.label()),
                        Style::default().fg(if selected { WHITE } else { TEXT }),
                    ),
                    Span::styled(value, Style::default().fg(CYAN)),
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(self.state_field_index));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .title(" parameters ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::StateParams {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                )
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(Style::default().bg(Color::Rgb(18, 35, 33))),
            layout[1],
            &mut state,
        );
    }

    fn render_timeline_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let layout = Layout::horizontal([
            Constraint::Length(28),
            Constraint::Fill(2),
            Constraint::Length(36),
        ])
        .split(area);
        self.render_lane_list(frame, layout[0], draft);
        self.render_timeline_graph(frame, layout[1], draft, session);
        self.render_inspector(frame, layout[2], draft, session);
    }

    fn render_lane_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let items = LaneId::ALL
            .iter()
            .map(|lane| {
                let count = draft.keyframe_count(*lane);
                ListItem::new(format!("{:<22} {:>2} keys", lane.label(), count))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(self.lane_index));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .title(" lanes ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::TimelineLanes {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                )
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(Style::default().bg(Color::Rgb(18, 35, 33))),
            area,
            &mut state,
        );
    }

    fn render_timeline_graph(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let lane = self.selected_lane();
        let playback = session.playback_clock();
        let automation = draft.automation();
        let beat = current_beat(playback, automation);
        let total_beats = automation.total_beats().max(1.0);
        let graph_width = area.width.saturating_sub(6).max(16) as usize;
        let mut lines = vec![
            Line::from(format!(
                "lane: {}  playhead beat {:>5.2}  measure {:>4.2}",
                lane.label(),
                beat,
                current_measure(playback, automation)
            )),
            Line::from(format!(
                "authored duration {:>5.1}s  audio duration {}",
                automation.duration_secs(),
                playback
                    .duration_secs
                    .map(|value| format!("{value:>5.1}s"))
                    .unwrap_or_else(|| "--".to_string())
            )),
            Line::from(""),
        ];
        lines.extend(build_timeline_graph(
            draft,
            lane,
            beat,
            total_beats,
            graph_width,
        ));
        lines.push(Line::from(""));

        if lane.is_color() {
            for (index, keyframe) in draft.color_keyframes(lane).iter().enumerate() {
                let marker = if index == self.keyframe_index {
                    ">"
                } else {
                    " "
                };
                lines.push(Line::from(format!(
                    "{} beat {:>5.2}  rgb [{:.2}, {:.2}, {:.2}]  {:?}",
                    marker,
                    keyframe.beat,
                    keyframe.value[0],
                    keyframe.value[1],
                    keyframe.value[2],
                    keyframe.interpolation
                )));
            }
        } else {
            for (index, keyframe) in draft.float_keyframes(lane).iter().enumerate() {
                let marker = if index == self.keyframe_index {
                    ">"
                } else {
                    " "
                };
                lines.push(Line::from(format!(
                    "{} beat {:>5.2}  value {:>6.3}  {:?}",
                    marker, keyframe.beat, keyframe.value, keyframe.interpolation
                )));
            }
        }

        if lines.len() == 3 {
            lines.push(Line::from("no keyframes on selected lane"));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" timeline ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::TimelineGraph {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                ),
            area,
        );
    }

    fn render_inspector(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let lane = self.selected_lane();
        let playback = session.playback_clock();
        let sampled = resolved_value_preview(draft, lane, playback);
        let mut lines = vec![
            render_inspector_line(
                "bpm",
                format!("{:.2}", draft.automation().bpm),
                self.inspector_field == InspectorField::Bpm,
            ),
            render_inspector_line(
                "measures",
                draft.automation().measures.to_string(),
                self.inspector_field == InspectorField::Measures,
            ),
            Line::from(""),
            Line::from(format!("sampled now: {}", sampled)),
        ];
        if lane.is_color() {
            if let Some(keyframe) = draft.selected_color_keyframe(lane, self.keyframe_index) {
                lines.extend(render_color_inspector(keyframe, self.inspector_field));
            } else {
                lines.push(Line::from("selected keyframe: none"));
            }
        } else if let Some(keyframe) = draft.selected_float_keyframe(lane, self.keyframe_index) {
            lines.extend(render_float_inspector(keyframe, self.inspector_field));
        } else {
            lines.push(Line::from("selected keyframe: none"));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" inspector ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::TimelineInspector {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                ),
            area,
        );
    }

    fn render_export_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let export = draft
            .export_json()
            .unwrap_or_else(|error| error.to_string());
        frame.render_widget(
            Paragraph::new(export)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(TEXT).bg(Color::Rgb(12, 18, 20)))
                .block(Block::bordered().title(" visualizer json ").border_style(
                    Style::default().fg(if self.focus_area == EditorFocusArea::Export {
                        CYAN
                    } else {
                        BORDER
                    }),
                )),
            area,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, draft: &ChromaticBulgeGridEditorDraft) {
        let status = if let Some(input) = &self.numeric_input {
            format!("editing {} = {}", input.target.label(), input.buffer)
        } else {
            self.status_message
                .borrow()
                .clone()
                .unwrap_or_else(|| "Tab focus  Enter edit  N add key  I interp  C copy".to_string())
        };
        let lines = vec![Line::from(vec![
            Span::styled(status, Style::default().fg(AMBER)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "keys: E close  P play/pause  ←/→ beat  [/ ] measure  D delete  S save  dirty={}",
                    draft.is_dirty()
                ),
                Style::default().fg(DIM),
            ),
        ])];
        frame.render_widget(
            Paragraph::new(lines)
                .style(Style::default().bg(Color::Rgb(10, 14, 16)))
                .block(Block::bordered().border_style(Style::default().fg(BORDER))),
            area,
        );
    }

    fn handle_non_numeric_key(
        &mut self,
        key: KeyCode,
        session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match self.tab {
            EditorTab::State => self.handle_state_key(key, record_id),
            EditorTab::Timeline => self.handle_timeline_key(key, session, record_id),
            EditorTab::Export => self.handle_export_key(key, record_id),
        }
    }

    fn handle_state_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match (self.focus_area, key) {
            (EditorFocusArea::StateTarget, KeyCode::Up)
            | (EditorFocusArea::StateTarget, KeyCode::Down) => {
                self.state_target = match self.state_target {
                    StateTarget::Playing => StateTarget::Idle,
                    StateTarget::Idle => StateTarget::Playing,
                };
                true
            }
            (EditorFocusArea::StateParams, KeyCode::Up) => {
                self.state_field_index = self.state_field_index.saturating_sub(1);
                true
            }
            (EditorFocusArea::StateParams, KeyCode::Down) => {
                self.state_field_index =
                    (self.state_field_index + 1).min(ParameterField::ALL.len().saturating_sub(1));
                true
            }
            (EditorFocusArea::StateParams, KeyCode::Enter) => {
                self.begin_state_numeric_edit(record_id);
                true
            }
            _ => true,
        }
    }

    fn handle_timeline_key(
        &mut self,
        key: KeyCode,
        session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match key {
            KeyCode::Up => {
                match self.focus_area {
                    EditorFocusArea::TimelineLanes => {
                        self.lane_index = self.lane_index.saturating_sub(1);
                        self.sync_selection(record_id);
                    }
                    EditorFocusArea::TimelineGraph => {
                        self.keyframe_index = self.keyframe_index.saturating_sub(1);
                    }
                    EditorFocusArea::TimelineInspector => {
                        self.inspector_field = self.inspector_field.prev(self.selected_lane());
                    }
                    _ => {}
                }
                true
            }
            KeyCode::Down => {
                match self.focus_area {
                    EditorFocusArea::TimelineLanes => {
                        self.lane_index =
                            (self.lane_index + 1).min(LaneId::ALL.len().saturating_sub(1));
                        self.sync_selection(record_id);
                    }
                    EditorFocusArea::TimelineGraph => {
                        let max_index = self
                            .drafts
                            .draft_for(record_id)
                            .map(|draft| draft.keyframe_count(self.selected_lane()))
                            .unwrap_or_default()
                            .saturating_sub(1);
                        self.keyframe_index = (self.keyframe_index + 1).min(max_index);
                    }
                    EditorFocusArea::TimelineInspector => {
                        self.inspector_field = self.inspector_field.next(self.selected_lane());
                    }
                    _ => {}
                }
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let playback = session.playback_clock();
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    let beat = current_beat(playback, draft.automation());
                    self.keyframe_index = draft.insert_keyframe(lane, beat);
                }
                true
            }
            KeyCode::Delete | KeyCode::Backspace => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.delete_keyframe(lane, self.keyframe_index);
                    self.sync_selection(record_id);
                    self.set_status("keyframe deleted");
                }
                true
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.delete_keyframe(lane, self.keyframe_index);
                    self.sync_selection(record_id);
                    self.set_status("keyframe deleted");
                }
                true
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.cycle_interpolation(lane, self.keyframe_index);
                }
                self.inspector_field = InspectorField::Interpolation;
                self.set_status("interpolation toggled");
                true
            }
            KeyCode::Enter => {
                if self.inspector_field == InspectorField::Interpolation {
                    let lane = self.selected_lane();
                    if let Some(draft) = self.drafts.draft_mut(record_id) {
                        draft.cycle_interpolation(lane, self.keyframe_index);
                    }
                    self.set_status("interpolation toggled");
                } else {
                    self.begin_timeline_numeric_edit(record_id);
                }
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if let Some(draft) = self.drafts.draft_for(record_id) {
                    copy_export_json(
                        draft.export_json().unwrap_or_default(),
                        Rc::clone(&self.status_message),
                    );
                }
                true
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if let Some(draft) = self.drafts.draft_for(record_id) {
                    save_visualizer_to_record(
                        record_id,
                        draft.working.normalized_for_export(),
                        Rc::clone(&self.status_message),
                    );
                }
                true
            }
            _ => true,
        }
    }

    fn handle_export_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match key {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if let Some(draft) = self.drafts.draft_for(record_id) {
                    copy_export_json(
                        draft.export_json().unwrap_or_default(),
                        Rc::clone(&self.status_message),
                    );
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if let Some(draft) = self.drafts.draft_for(record_id) {
                    save_visualizer_to_record(
                        record_id,
                        draft.working.normalized_for_export(),
                        Rc::clone(&self.status_message),
                    );
                }
            }
            _ => {}
        }
        true
    }

    fn handle_numeric_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        let Some(input) = &mut self.numeric_input else {
            return false;
        };
        match key {
            KeyCode::Esc => {
                self.numeric_input = None;
                true
            }
            KeyCode::Enter => {
                let buffer = input.buffer.clone();
                let target = input.target.clone();
                self.numeric_input = None;
                self.commit_numeric_edit(record_id, target, buffer);
                true
            }
            KeyCode::Backspace => {
                if input.replace_on_type {
                    input.buffer.clear();
                } else {
                    input.buffer.pop();
                }
                true
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == '.' || ch == '-' => {
                if input.replace_on_type {
                    input.buffer.clear();
                    input.replace_on_type = false;
                }
                input.buffer.push(ch);
                true
            }
            _ => true,
        }
    }

    fn begin_state_numeric_edit(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let field = self.selected_state_field();
        let state = draft.state(self.state_target);
        let value = format_parameter_value(state, field);
        self.numeric_input = Some(NumericInputState {
            target: NumericEditTarget::StateField(field),
            buffer: value,
            replace_on_type: true,
        });
        self.set_status(format!("editing {}", field.label()));
    }

    fn begin_timeline_numeric_edit(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let lane = self.selected_lane();
        let target = match self.inspector_field {
            InspectorField::Bpm => {
                self.numeric_input = Some(NumericInputState {
                    target: NumericEditTarget::Bpm,
                    buffer: format!("{:.2}", draft.automation().bpm),
                    replace_on_type: true,
                });
                self.set_status("editing bpm");
                return;
            }
            InspectorField::Measures => {
                self.numeric_input = Some(NumericInputState {
                    target: NumericEditTarget::Measures,
                    buffer: draft.automation().measures.to_string(),
                    replace_on_type: true,
                });
                self.set_status("editing measures");
                return;
            }
            InspectorField::Beat => {
                let buffer = draft
                    .keyframe_beat(lane, self.keyframe_index)
                    .map(|value| format!("{value:.3}"))
                    .unwrap_or_else(|| "0.0".to_string());
                self.numeric_input = Some(NumericInputState {
                    target: NumericEditTarget::KeyframeBeat(lane),
                    buffer,
                    replace_on_type: true,
                });
                self.set_status("editing keyframe beat");
                return;
            }
            InspectorField::Interpolation => {
                self.set_status("press I or Enter to toggle interpolation");
                return;
            }
            InspectorField::Value => NumericEditTarget::KeyframeValue(lane),
            InspectorField::Red => NumericEditTarget::KeyframeColor(lane, 0),
            InspectorField::Green => NumericEditTarget::KeyframeColor(lane, 1),
            InspectorField::Blue => NumericEditTarget::KeyframeColor(lane, 2),
        };
        let buffer = draft
            .keyframe_component(lane, self.keyframe_index, self.inspector_field)
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "0.0".to_string());
        self.numeric_input = Some(NumericInputState {
            target,
            buffer,
            replace_on_type: true,
        });
        self.set_status(format!("editing {}", self.inspector_field.label()));
    }

    fn commit_numeric_edit(&mut self, record_id: &str, target: NumericEditTarget, buffer: String) {
        let Ok(value) = buffer.parse::<f32>() else {
            self.set_status("numeric parse failed");
            return;
        };
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            return;
        };
        match target {
            NumericEditTarget::StateField(field) => {
                draft.set_state_field(self.state_target, field, value)
            }
            NumericEditTarget::Bpm => draft.working.automation_mut().bpm = value.max(1.0),
            NumericEditTarget::Measures => {
                draft.working.automation_mut().measures = value.max(1.0).round() as u32
            }
            NumericEditTarget::KeyframeBeat(lane) => {
                draft.set_keyframe_beat(lane, self.keyframe_index, value.max(0.0))
            }
            NumericEditTarget::KeyframeValue(lane) => {
                draft.set_keyframe_value(lane, self.keyframe_index, value)
            }
            NumericEditTarget::KeyframeColor(lane, channel) => {
                draft.set_keyframe_color(lane, self.keyframe_index, channel, value)
            }
        }
        self.sync_selection(record_id);
    }

    fn scrub_beats(&mut self, session: &mut SessionModel, delta_beats: f32) {
        let Some(draft) = self.drafts.draft_for(session.current_record().id.as_str()) else {
            return;
        };
        let bpm = draft.automation().bpm.max(1.0);
        let playback = session.playback_clock();
        let next_secs = playback.current_time_secs + delta_beats * 60.0 / bpm;
        session.seek_to_secs(next_secs.max(0.0));
    }

    fn scrub_measures(&mut self, session: &mut SessionModel, delta_measures: f32) {
        let Some(draft) = self.drafts.draft_for(session.current_record().id.as_str()) else {
            return;
        };
        let automation = draft.automation();
        let delta_beats = delta_measures * automation.beats_per_measure as f32;
        self.scrub_beats(session, delta_beats);
    }

    fn sync_selection(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            self.keyframe_index = 0;
            return;
        };
        self.keyframe_index = self
            .keyframe_index
            .min(draft.keyframe_count(self.selected_lane()).saturating_sub(1));
        if !self.selected_lane().is_color()
            && !matches!(
                self.inspector_field,
                InspectorField::Beat | InspectorField::Value | InspectorField::Interpolation
            )
        {
            self.inspector_field = InspectorField::Value;
        }
    }

    fn selected_state_field(&self) -> ParameterField {
        ParameterField::ALL[self.state_field_index]
    }

    fn selected_lane(&self) -> LaneId {
        LaneId::ALL[self.lane_index]
    }

    fn set_status(&self, message: impl Into<String>) {
        *self.status_message.borrow_mut() = Some(message.into());
    }
}

impl EditorDraftStore {
    fn ensure(&mut self, record: &RecordDocument) {
        if self.drafts.contains_key(record.id.as_str()) {
            return;
        }
        let config = record
            .visualizer()
            .cloned()
            .unwrap_or_else(default_visualizer_config);
        self.drafts.insert(
            record.id.clone(),
            ChromaticBulgeGridEditorDraft {
                record_id: record.id.clone(),
                original: config.clone(),
                working: config,
            },
        );
    }

    fn draft_for(&self, record_id: &str) -> Option<&ChromaticBulgeGridEditorDraft> {
        self.drafts.get(record_id)
    }

    fn draft_mut(&mut self, record_id: &str) -> Option<&mut ChromaticBulgeGridEditorDraft> {
        self.drafts.get_mut(record_id)
    }
}

impl ChromaticBulgeGridEditorDraft {
    pub fn automation(&self) -> &ChromaticBulgeGridAutomation {
        self.working
            .automation
            .as_ref()
            .expect("editor draft automation should exist")
    }

    pub fn state(&self, target: StateTarget) -> ChromaticBulgeGridShaderState {
        let states = self.working.params.shader_states.unwrap();
        match target {
            StateTarget::Playing => states.playing,
            StateTarget::Idle => states.idle,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.original.normalized_for_export() != self.working.normalized_for_export()
    }

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        to_string_pretty(&self.working.normalized_for_export())
    }

    pub fn keyframe_count(&self, lane: LaneId) -> usize {
        if lane.is_color() {
            self.color_keyframes(lane).len()
        } else {
            self.float_keyframes(lane).len()
        }
    }

    pub fn float_keyframes(&self, lane: LaneId) -> &[FloatKeyframe] {
        float_lane(self.automation(), lane)
    }

    pub fn color_keyframes(&self, lane: LaneId) -> &[ColorKeyframe] {
        color_lane(self.automation(), lane)
    }

    pub fn selected_float_keyframe(&self, lane: LaneId, index: usize) -> Option<&FloatKeyframe> {
        self.float_keyframes(lane).get(index)
    }

    pub fn selected_color_keyframe(&self, lane: LaneId, index: usize) -> Option<&ColorKeyframe> {
        self.color_keyframes(lane).get(index)
    }

    pub fn keyframe_beat(&self, lane: LaneId, index: usize) -> Option<f32> {
        if lane.is_color() {
            self.selected_color_keyframe(lane, index)
                .map(|keyframe| keyframe.beat)
        } else {
            self.selected_float_keyframe(lane, index)
                .map(|keyframe| keyframe.beat)
        }
    }

    pub fn keyframe_component(
        &self,
        lane: LaneId,
        index: usize,
        field: InspectorField,
    ) -> Option<f32> {
        if lane.is_color() {
            let keyframe = self.selected_color_keyframe(lane, index)?;
            match field {
                InspectorField::Bpm | InspectorField::Measures => None,
                InspectorField::Value | InspectorField::Red => Some(keyframe.value[0]),
                InspectorField::Green => Some(keyframe.value[1]),
                InspectorField::Blue => Some(keyframe.value[2]),
                InspectorField::Interpolation => None,
                InspectorField::Beat => Some(keyframe.beat),
            }
        } else {
            self.selected_float_keyframe(lane, index)
                .map(|keyframe| match field {
                    InspectorField::Beat => keyframe.beat,
                    InspectorField::Interpolation => keyframe.value,
                    _ => keyframe.value,
                })
        }
    }

    pub fn set_state_field(&mut self, target: StateTarget, field: ParameterField, value: f32) {
        let states = self.working.params.shader_states.as_mut().unwrap();
        let state = match target {
            StateTarget::Playing => &mut states.playing,
            StateTarget::Idle => &mut states.idle,
        };
        set_parameter_field(state, field, value);
    }

    pub fn insert_keyframe(&mut self, lane: LaneId, beat: f32) -> usize {
        let current_value = resolved_lane_value(
            self,
            lane,
            PlaybackClock {
                current_time_secs: beat * 60.0 / self.automation().bpm.max(1.0),
                duration_secs: None,
                is_playing: true,
            },
        );
        if lane.is_color() {
            let keyframes = color_lane_mut(self.working.automation_mut(), lane);
            if let Some(index) = keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
            {
                return index;
            }
            let value = match current_value {
                LaneValue::Color(value) => value,
                LaneValue::Float(_) => [1.0, 1.0, 1.0],
            };
            keyframes.push(ColorKeyframe {
                beat,
                value,
                interpolation: InterpolationMode::Hold,
            });
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
                .unwrap_or_default()
        } else {
            let keyframes = float_lane_mut(self.working.automation_mut(), lane);
            if let Some(index) = keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
            {
                return index;
            }
            let value = match current_value {
                LaneValue::Float(value) => value,
                LaneValue::Color(_) => 0.0,
            };
            keyframes.push(FloatKeyframe {
                beat,
                value,
                interpolation: InterpolationMode::Hold,
            });
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
                .unwrap_or_default()
        }
    }

    pub fn delete_keyframe(&mut self, lane: LaneId, index: usize) {
        if lane.is_color() {
            let keyframes = color_lane_mut(self.working.automation_mut(), lane);
            if index < keyframes.len() {
                keyframes.remove(index);
            }
        } else {
            let keyframes = float_lane_mut(self.working.automation_mut(), lane);
            if index < keyframes.len() {
                keyframes.remove(index);
            }
        }
    }

    pub fn cycle_interpolation(&mut self, lane: LaneId, index: usize) {
        if lane.is_color() {
            if let Some(keyframe) =
                color_lane_mut(self.working.automation_mut(), lane).get_mut(index)
            {
                keyframe.interpolation = keyframe.interpolation.cycle();
            }
        } else if let Some(keyframe) =
            float_lane_mut(self.working.automation_mut(), lane).get_mut(index)
        {
            keyframe.interpolation = keyframe.interpolation.cycle();
        }
    }

    pub fn set_keyframe_beat(&mut self, lane: LaneId, index: usize, beat: f32) {
        if lane.is_color() {
            let keyframes = color_lane_mut(self.working.automation_mut(), lane);
            if let Some(keyframe) = keyframes.get_mut(index) {
                keyframe.beat = beat;
            }
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
        } else {
            let keyframes = float_lane_mut(self.working.automation_mut(), lane);
            if let Some(keyframe) = keyframes.get_mut(index) {
                keyframe.beat = beat;
            }
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
        }
    }

    pub fn set_keyframe_value(&mut self, lane: LaneId, index: usize, value: f32) {
        if let Some(keyframe) = float_lane_mut(self.working.automation_mut(), lane).get_mut(index) {
            keyframe.value = value;
        }
    }

    pub fn set_keyframe_color(&mut self, lane: LaneId, index: usize, channel: usize, value: f32) {
        if let Some(keyframe) = color_lane_mut(self.working.automation_mut(), lane).get_mut(index) {
            if channel < 3 {
                keyframe.value[channel] = value;
            }
        }
    }
}

impl TrackVisualizerConfig {
    fn automation_mut(&mut self) -> &mut ChromaticBulgeGridAutomation {
        self.automation.get_or_insert_with(default_automation)
    }
}

impl ParameterField {
    pub const ALL: [Self; 27] = [
        Self::MotionRate,
        Self::LatticeDensity,
        Self::CircleRadius,
        Self::CircleFalloffStart,
        Self::CircleFalloffEnd,
        Self::BulgeAmount,
        Self::RimGuard,
        Self::RimExponent,
        Self::RimWarp,
        Self::SpacingMaxPx,
        Self::SpacingMinPx,
        Self::DotSize,
        Self::OuterDotScale,
        Self::EdgeSoftness,
        Self::ChromaticAberration,
        Self::ScrollBase,
        Self::ScrollMotionScale,
        Self::ScrollMotionFloor,
        Self::ScrollMotionCeiling,
        Self::ColdColorR,
        Self::ColdColorG,
        Self::ColdColorB,
        Self::HotColorR,
        Self::HotColorG,
        Self::HotColorB,
        Self::ColorCycleRate,
        Self::InnerAlpha,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::MotionRate => "motion_rate",
            Self::LatticeDensity => "lattice_density",
            Self::CircleRadius => "circle_radius",
            Self::CircleFalloffStart => "circle_falloff_start",
            Self::CircleFalloffEnd => "circle_falloff_end",
            Self::BulgeAmount => "bulge_amount",
            Self::RimGuard => "rim_guard",
            Self::RimExponent => "rim_exponent",
            Self::RimWarp => "rim_warp",
            Self::SpacingMaxPx => "spacing_max_px",
            Self::SpacingMinPx => "spacing_min_px",
            Self::DotSize => "dot_size",
            Self::OuterDotScale => "outer_dot_scale",
            Self::EdgeSoftness => "edge_softness",
            Self::ChromaticAberration => "chromatic_aberration",
            Self::ScrollBase => "scroll_base",
            Self::ScrollMotionScale => "scroll_motion_scale",
            Self::ScrollMotionFloor => "scroll_motion_floor",
            Self::ScrollMotionCeiling => "scroll_motion_ceiling",
            Self::ColdColorR => "cold_color.r",
            Self::ColdColorG => "cold_color.g",
            Self::ColdColorB => "cold_color.b",
            Self::HotColorR => "hot_color.r",
            Self::HotColorG => "hot_color.g",
            Self::HotColorB => "hot_color.b",
            Self::ColorCycleRate => "color_cycle_rate",
            Self::InnerAlpha => "inner_alpha",
        }
    }
}

impl LaneId {
    pub const ALL: [Self; 23] = [
        Self::MotionRate,
        Self::LatticeDensity,
        Self::CircleRadius,
        Self::CircleFalloffStart,
        Self::CircleFalloffEnd,
        Self::BulgeAmount,
        Self::RimGuard,
        Self::RimExponent,
        Self::RimWarp,
        Self::SpacingMaxPx,
        Self::SpacingMinPx,
        Self::DotSize,
        Self::OuterDotScale,
        Self::EdgeSoftness,
        Self::ChromaticAberration,
        Self::ScrollBase,
        Self::ScrollMotionScale,
        Self::ScrollMotionFloor,
        Self::ScrollMotionCeiling,
        Self::ColdColor,
        Self::HotColor,
        Self::ColorCycleRate,
        Self::InnerAlpha,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::MotionRate => "motion_rate",
            Self::LatticeDensity => "lattice_density",
            Self::CircleRadius => "circle_radius",
            Self::CircleFalloffStart => "circle_falloff_start",
            Self::CircleFalloffEnd => "circle_falloff_end",
            Self::BulgeAmount => "bulge_amount",
            Self::RimGuard => "rim_guard",
            Self::RimExponent => "rim_exponent",
            Self::RimWarp => "rim_warp",
            Self::SpacingMaxPx => "spacing_max_px",
            Self::SpacingMinPx => "spacing_min_px",
            Self::DotSize => "dot_size",
            Self::OuterDotScale => "outer_dot_scale",
            Self::EdgeSoftness => "edge_softness",
            Self::ChromaticAberration => "chromatic_aberration",
            Self::ScrollBase => "scroll_base",
            Self::ScrollMotionScale => "scroll_motion_scale",
            Self::ScrollMotionFloor => "scroll_motion_floor",
            Self::ScrollMotionCeiling => "scroll_motion_ceiling",
            Self::ColdColor => "cold_color",
            Self::HotColor => "hot_color",
            Self::ColorCycleRate => "color_cycle_rate",
            Self::InnerAlpha => "inner_alpha",
        }
    }

    fn is_color(self) -> bool {
        matches!(self, Self::ColdColor | Self::HotColor)
    }
}

impl InspectorField {
    fn label(self) -> &'static str {
        match self {
            Self::Bpm => "bpm",
            Self::Measures => "measures",
            Self::Beat => "beat",
            Self::Value => "value",
            Self::Interpolation => "interpolation",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
        }
    }

    fn next(self, lane: LaneId) -> Self {
        match (lane.is_color(), self) {
            (false, Self::Bpm) => Self::Measures,
            (false, Self::Measures) => Self::Beat,
            (false, Self::Beat) => Self::Value,
            (false, Self::Value) => Self::Interpolation,
            (false, _) => Self::Bpm,
            (true, Self::Bpm) => Self::Measures,
            (true, Self::Measures) => Self::Beat,
            (true, Self::Beat) => Self::Interpolation,
            (true, Self::Interpolation) => Self::Red,
            (true, Self::Red) => Self::Green,
            (true, Self::Green) => Self::Blue,
            (true, _) => Self::Bpm,
        }
    }

    fn prev(self, lane: LaneId) -> Self {
        match (lane.is_color(), self) {
            (false, Self::Bpm) => Self::Interpolation,
            (false, Self::Measures) => Self::Bpm,
            (false, Self::Beat) => Self::Measures,
            (false, Self::Value) => Self::Beat,
            (false, Self::Interpolation) => Self::Value,
            (false, _) => Self::Interpolation,
            (true, Self::Bpm) => Self::Blue,
            (true, Self::Measures) => Self::Bpm,
            (true, Self::Beat) => Self::Measures,
            (true, Self::Interpolation) => Self::Beat,
            (true, Self::Red) => Self::Interpolation,
            (true, Self::Green) => Self::Red,
            (true, Self::Blue) => Self::Green,
            (true, Self::Value) => Self::Beat,
        }
    }
}

impl InterpolationMode {
    fn cycle(self) -> Self {
        match self {
            Self::Hold => Self::Linear,
            Self::Linear => Self::Hold,
        }
    }
}

fn editor_flag_enabled() -> bool {
    let Some(window) = window() else {
        return false;
    };
    let Ok(search) = window.location().search() else {
        return false;
    };
    web_sys::UrlSearchParams::new_with_str(search.as_str())
        .ok()
        .and_then(|params| params.get("editor"))
        .as_deref()
        == Some("1")
}

fn default_visualizer_config() -> TrackVisualizerConfig {
    TrackVisualizerConfig {
        mode: TrackVisualizerMode::ChromaticBulgeGrid,
        params: crate::archive::TrackVisualizerParams {
            shader_states: Some(ChromaticBulgeGridShaderStates::default()),
            ..Default::default()
        },
        automation: Some(default_automation()),
    }
}

fn default_automation() -> ChromaticBulgeGridAutomation {
    ChromaticBulgeGridAutomation {
        bpm: 132.0,
        measures: 64,
        beats_per_measure: 4,
        lanes: ChromaticBulgeGridAutomationLanes::default(),
    }
}

fn next_focus_area(tab: EditorTab, current: EditorFocusArea) -> EditorFocusArea {
    match tab {
        EditorTab::State => match current {
            EditorFocusArea::StateTarget => EditorFocusArea::StateParams,
            _ => EditorFocusArea::StateTarget,
        },
        EditorTab::Timeline => match current {
            EditorFocusArea::TimelineLanes => EditorFocusArea::TimelineGraph,
            EditorFocusArea::TimelineGraph => EditorFocusArea::TimelineInspector,
            _ => EditorFocusArea::TimelineLanes,
        },
        EditorTab::Export => EditorFocusArea::Export,
    }
}

fn set_parameter_field(
    state: &mut ChromaticBulgeGridShaderState,
    field: ParameterField,
    value: f32,
) {
    match field {
        ParameterField::MotionRate => state.motion_rate = value,
        ParameterField::LatticeDensity => state.lattice_density = value,
        ParameterField::CircleRadius => state.circle_radius = value,
        ParameterField::CircleFalloffStart => state.circle_falloff_start = value,
        ParameterField::CircleFalloffEnd => state.circle_falloff_end = value,
        ParameterField::BulgeAmount => state.bulge_amount = value,
        ParameterField::RimGuard => state.rim_guard = value,
        ParameterField::RimExponent => state.rim_exponent = value,
        ParameterField::RimWarp => state.rim_warp = value,
        ParameterField::SpacingMaxPx => state.spacing_max_px = value,
        ParameterField::SpacingMinPx => state.spacing_min_px = value,
        ParameterField::DotSize => state.dot_size = value,
        ParameterField::OuterDotScale => state.outer_dot_scale = value,
        ParameterField::EdgeSoftness => state.edge_softness = value,
        ParameterField::ChromaticAberration => state.chromatic_aberration = value,
        ParameterField::ScrollBase => state.scroll_base = value,
        ParameterField::ScrollMotionScale => state.scroll_motion_scale = value,
        ParameterField::ScrollMotionFloor => state.scroll_motion_floor = value,
        ParameterField::ScrollMotionCeiling => state.scroll_motion_ceiling = value,
        ParameterField::ColdColorR => state.cold_color[0] = value,
        ParameterField::ColdColorG => state.cold_color[1] = value,
        ParameterField::ColdColorB => state.cold_color[2] = value,
        ParameterField::HotColorR => state.hot_color[0] = value,
        ParameterField::HotColorG => state.hot_color[1] = value,
        ParameterField::HotColorB => state.hot_color[2] = value,
        ParameterField::ColorCycleRate => state.color_cycle_rate = value,
        ParameterField::InnerAlpha => state.inner_alpha = value,
    }
}

fn format_parameter_value(state: ChromaticBulgeGridShaderState, field: ParameterField) -> String {
    let value = match field {
        ParameterField::MotionRate => state.motion_rate,
        ParameterField::LatticeDensity => state.lattice_density,
        ParameterField::CircleRadius => state.circle_radius,
        ParameterField::CircleFalloffStart => state.circle_falloff_start,
        ParameterField::CircleFalloffEnd => state.circle_falloff_end,
        ParameterField::BulgeAmount => state.bulge_amount,
        ParameterField::RimGuard => state.rim_guard,
        ParameterField::RimExponent => state.rim_exponent,
        ParameterField::RimWarp => state.rim_warp,
        ParameterField::SpacingMaxPx => state.spacing_max_px,
        ParameterField::SpacingMinPx => state.spacing_min_px,
        ParameterField::DotSize => state.dot_size,
        ParameterField::OuterDotScale => state.outer_dot_scale,
        ParameterField::EdgeSoftness => state.edge_softness,
        ParameterField::ChromaticAberration => state.chromatic_aberration,
        ParameterField::ScrollBase => state.scroll_base,
        ParameterField::ScrollMotionScale => state.scroll_motion_scale,
        ParameterField::ScrollMotionFloor => state.scroll_motion_floor,
        ParameterField::ScrollMotionCeiling => state.scroll_motion_ceiling,
        ParameterField::ColdColorR => state.cold_color[0],
        ParameterField::ColdColorG => state.cold_color[1],
        ParameterField::ColdColorB => state.cold_color[2],
        ParameterField::HotColorR => state.hot_color[0],
        ParameterField::HotColorG => state.hot_color[1],
        ParameterField::HotColorB => state.hot_color[2],
        ParameterField::ColorCycleRate => state.color_cycle_rate,
        ParameterField::InnerAlpha => state.inner_alpha,
    };
    format!("{value:.3}")
}

fn current_beat(playback: PlaybackClock, automation: &ChromaticBulgeGridAutomation) -> f32 {
    playback.current_time_secs.max(0.0) * automation.bpm.max(1.0) / 60.0
}

fn current_measure(playback: PlaybackClock, automation: &ChromaticBulgeGridAutomation) -> f32 {
    current_beat(playback, automation) / automation.beats_per_measure.max(1) as f32
}

fn resolved_value_preview(
    draft: &ChromaticBulgeGridEditorDraft,
    lane: LaneId,
    playback: PlaybackClock,
) -> String {
    match resolved_lane_value(draft, lane, playback) {
        LaneValue::Float(value) => format!("{value:.3}"),
        LaneValue::Color(value) => format!("[{:.2}, {:.2}, {:.2}]", value[0], value[1], value[2]),
    }
}

fn resolved_lane_value(
    draft: &ChromaticBulgeGridEditorDraft,
    lane: LaneId,
    playback: PlaybackClock,
) -> LaneValue {
    let ChromaticBulgeGridResolvedState { uniforms, .. } =
        draft.working.resolve_chromatic_bulge_grid(playback);
    match lane {
        LaneId::MotionRate => LaneValue::Float(uniforms.motion_rate),
        LaneId::LatticeDensity => LaneValue::Float(uniforms.lattice_density),
        LaneId::CircleRadius => LaneValue::Float(uniforms.circle_radius),
        LaneId::CircleFalloffStart => LaneValue::Float(uniforms.circle_falloff_start),
        LaneId::CircleFalloffEnd => LaneValue::Float(uniforms.circle_falloff_end),
        LaneId::BulgeAmount => LaneValue::Float(uniforms.bulge_amount),
        LaneId::RimGuard => LaneValue::Float(uniforms.rim_guard),
        LaneId::RimExponent => LaneValue::Float(uniforms.rim_exponent),
        LaneId::RimWarp => LaneValue::Float(uniforms.rim_warp),
        LaneId::SpacingMaxPx => LaneValue::Float(uniforms.spacing_max_px),
        LaneId::SpacingMinPx => LaneValue::Float(uniforms.spacing_min_px),
        LaneId::DotSize => LaneValue::Float(uniforms.dot_size),
        LaneId::OuterDotScale => LaneValue::Float(uniforms.outer_dot_scale),
        LaneId::EdgeSoftness => LaneValue::Float(uniforms.edge_softness),
        LaneId::ChromaticAberration => LaneValue::Float(uniforms.chromatic_aberration),
        LaneId::ScrollBase => LaneValue::Float(uniforms.scroll_base),
        LaneId::ScrollMotionScale => LaneValue::Float(uniforms.scroll_motion_scale),
        LaneId::ScrollMotionFloor => LaneValue::Float(uniforms.scroll_motion_floor),
        LaneId::ScrollMotionCeiling => LaneValue::Float(uniforms.scroll_motion_ceiling),
        LaneId::ColdColor => LaneValue::Color(uniforms.cold_color),
        LaneId::HotColor => LaneValue::Color(uniforms.hot_color),
        LaneId::ColorCycleRate => LaneValue::Float(uniforms.color_cycle_rate),
        LaneId::InnerAlpha => LaneValue::Float(uniforms.inner_alpha),
    }
}

fn build_timeline_graph(
    draft: &ChromaticBulgeGridEditorDraft,
    lane: LaneId,
    playhead_beat: f32,
    total_beats: f32,
    width: usize,
) -> Vec<Line<'static>> {
    let mut top = vec!['-'; width];
    let mut mid = vec![' '; width];
    let mut bottom = vec!['-'; width];
    let lane_key_positions = if lane.is_color() {
        draft
            .color_keyframes(lane)
            .iter()
            .enumerate()
            .map(|(index, keyframe)| (index, keyframe.beat))
            .collect::<Vec<_>>()
    } else {
        draft
            .float_keyframes(lane)
            .iter()
            .enumerate()
            .map(|(index, keyframe)| (index, keyframe.beat))
            .collect::<Vec<_>>()
    };

    let to_index = |beat: f32| -> usize {
        (((beat / total_beats).clamp(0.0, 1.0)) * (width.saturating_sub(1) as f32)).round() as usize
    };

    for beat in 0..=draft.automation().measures {
        let beat_position = beat as f32 * draft.automation().beats_per_measure as f32;
        let index = to_index(beat_position.min(total_beats));
        top[index] = '|';
        bottom[index] = '|';
    }

    for (index, beat) in lane_key_positions {
        let column = to_index(beat);
        mid[column] = if index == draft.keyframe_count(lane).saturating_sub(1) {
            '*'
        } else {
            'o'
        };
    }

    let playhead_index = to_index(playhead_beat);
    top[playhead_index] = '^';
    mid[playhead_index] = '|';
    bottom[playhead_index] = 'v';

    vec![
        Line::from(format!(
            "0{:>width$}",
            format!("{:.0} beats", total_beats),
            width = width - 1
        )),
        Line::from(top.into_iter().collect::<String>()),
        Line::from(mid.into_iter().collect::<String>()),
        Line::from(bottom.into_iter().collect::<String>()),
    ]
}

#[derive(Clone, Copy)]
enum LaneValue {
    Float(f32),
    Color([f32; 3]),
}

fn float_lane(automation: &ChromaticBulgeGridAutomation, lane: LaneId) -> &[FloatKeyframe] {
    match lane {
        LaneId::MotionRate => &automation.lanes.motion_rate,
        LaneId::LatticeDensity => &automation.lanes.lattice_density,
        LaneId::CircleRadius => &automation.lanes.circle_radius,
        LaneId::CircleFalloffStart => &automation.lanes.circle_falloff_start,
        LaneId::CircleFalloffEnd => &automation.lanes.circle_falloff_end,
        LaneId::BulgeAmount => &automation.lanes.bulge_amount,
        LaneId::RimGuard => &automation.lanes.rim_guard,
        LaneId::RimExponent => &automation.lanes.rim_exponent,
        LaneId::RimWarp => &automation.lanes.rim_warp,
        LaneId::SpacingMaxPx => &automation.lanes.spacing_max_px,
        LaneId::SpacingMinPx => &automation.lanes.spacing_min_px,
        LaneId::DotSize => &automation.lanes.dot_size,
        LaneId::OuterDotScale => &automation.lanes.outer_dot_scale,
        LaneId::EdgeSoftness => &automation.lanes.edge_softness,
        LaneId::ChromaticAberration => &automation.lanes.chromatic_aberration,
        LaneId::ScrollBase => &automation.lanes.scroll_base,
        LaneId::ScrollMotionScale => &automation.lanes.scroll_motion_scale,
        LaneId::ScrollMotionFloor => &automation.lanes.scroll_motion_floor,
        LaneId::ScrollMotionCeiling => &automation.lanes.scroll_motion_ceiling,
        LaneId::ColorCycleRate => &automation.lanes.color_cycle_rate,
        LaneId::InnerAlpha => &automation.lanes.inner_alpha,
        LaneId::ColdColor | LaneId::HotColor => &[],
    }
}

fn color_lane(automation: &ChromaticBulgeGridAutomation, lane: LaneId) -> &[ColorKeyframe] {
    match lane {
        LaneId::ColdColor => &automation.lanes.cold_color,
        LaneId::HotColor => &automation.lanes.hot_color,
        _ => &[],
    }
}

fn float_lane_mut(
    automation: &mut ChromaticBulgeGridAutomation,
    lane: LaneId,
) -> &mut Vec<FloatKeyframe> {
    match lane {
        LaneId::MotionRate => &mut automation.lanes.motion_rate,
        LaneId::LatticeDensity => &mut automation.lanes.lattice_density,
        LaneId::CircleRadius => &mut automation.lanes.circle_radius,
        LaneId::CircleFalloffStart => &mut automation.lanes.circle_falloff_start,
        LaneId::CircleFalloffEnd => &mut automation.lanes.circle_falloff_end,
        LaneId::BulgeAmount => &mut automation.lanes.bulge_amount,
        LaneId::RimGuard => &mut automation.lanes.rim_guard,
        LaneId::RimExponent => &mut automation.lanes.rim_exponent,
        LaneId::RimWarp => &mut automation.lanes.rim_warp,
        LaneId::SpacingMaxPx => &mut automation.lanes.spacing_max_px,
        LaneId::SpacingMinPx => &mut automation.lanes.spacing_min_px,
        LaneId::DotSize => &mut automation.lanes.dot_size,
        LaneId::OuterDotScale => &mut automation.lanes.outer_dot_scale,
        LaneId::EdgeSoftness => &mut automation.lanes.edge_softness,
        LaneId::ChromaticAberration => &mut automation.lanes.chromatic_aberration,
        LaneId::ScrollBase => &mut automation.lanes.scroll_base,
        LaneId::ScrollMotionScale => &mut automation.lanes.scroll_motion_scale,
        LaneId::ScrollMotionFloor => &mut automation.lanes.scroll_motion_floor,
        LaneId::ScrollMotionCeiling => &mut automation.lanes.scroll_motion_ceiling,
        LaneId::ColorCycleRate => &mut automation.lanes.color_cycle_rate,
        LaneId::InnerAlpha => &mut automation.lanes.inner_alpha,
        LaneId::ColdColor | LaneId::HotColor => unreachable!("color lane requested as float"),
    }
}

fn color_lane_mut(
    automation: &mut ChromaticBulgeGridAutomation,
    lane: LaneId,
) -> &mut Vec<ColorKeyframe> {
    match lane {
        LaneId::ColdColor => &mut automation.lanes.cold_color,
        LaneId::HotColor => &mut automation.lanes.hot_color,
        _ => unreachable!("float lane requested as color"),
    }
}

fn render_float_inspector(
    keyframe: &FloatKeyframe,
    inspector_field: InspectorField,
) -> Vec<Line<'static>> {
    vec![
        render_inspector_line(
            "beat",
            format!("{:.3}", keyframe.beat),
            inspector_field == InspectorField::Beat,
        ),
        render_inspector_line(
            "value",
            format!("{:.3}", keyframe.value),
            inspector_field == InspectorField::Value,
        ),
        render_inspector_line(
            "interp",
            format!("{:?}", keyframe.interpolation),
            inspector_field == InspectorField::Interpolation,
        ),
    ]
}

fn render_color_inspector(
    keyframe: &ColorKeyframe,
    inspector_field: InspectorField,
) -> Vec<Line<'static>> {
    vec![
        render_inspector_line(
            "beat",
            format!("{:.3}", keyframe.beat),
            inspector_field == InspectorField::Beat,
        ),
        render_inspector_line(
            "interp",
            format!("{:?}", keyframe.interpolation),
            inspector_field == InspectorField::Interpolation,
        ),
        render_inspector_line(
            "red",
            format!("{:.3}", keyframe.value[0]),
            inspector_field == InspectorField::Red,
        ),
        render_inspector_line(
            "green",
            format!("{:.3}", keyframe.value[1]),
            inspector_field == InspectorField::Green,
        ),
        render_inspector_line(
            "blue",
            format!("{:.3}", keyframe.value[2]),
            inspector_field == InspectorField::Blue,
        ),
    ]
}

fn render_inspector_line(label: &str, value: String, selected: bool) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<8}", label),
            Style::default().fg(if selected { WHITE } else { DIM }),
        ),
        Span::styled(value, Style::default().fg(CYAN)),
    ])
}

fn copy_export_json(json: String, status: Rc<RefCell<Option<String>>>) {
    let Some(window) = window() else {
        web_sys::console::log_1(&json.clone().into());
        *status.borrow_mut() = Some("clipboard unavailable; logged json to console".to_string());
        return;
    };
    let clipboard = window.navigator().clipboard();
    let promise = clipboard.write_text(&json);
    *status.borrow_mut() = Some("clipboard write requested".to_string());
    spawn_local(async move {
        if JsFuture::from(promise).await.is_err() {
            web_sys::console::log_1(&json.into());
            *status.borrow_mut() = Some("clipboard failed; logged json to console".to_string());
        } else {
            *status.borrow_mut() = Some("visualizer json copied".to_string());
        }
    });
}

fn save_visualizer_to_record(
    record_id: &str,
    visualizer: TrackVisualizerConfig,
    status: Rc<RefCell<Option<String>>>,
) {
    let payload = serde_json::json!({
        "record_id": record_id,
        "visualizer": visualizer,
    });
    let Ok(body) = serde_json::to_string(&payload) else {
        *status.borrow_mut() = Some("save failed: could not serialize payload".to_string());
        return;
    };

    let Some(window) = window() else {
        *status.borrow_mut() = Some("save failed: no browser window".to_string());
        return;
    };

    let request = RequestInit::new();
    request.set_method("POST");
    request.set_mode(RequestMode::Cors);
    request.set_body(&JsValue::from_str(body.as_str()));

    *status.borrow_mut() = Some("saving visualizer to record json...".to_string());
    let promise = window.fetch_with_str_and_init(SAVE_BRIDGE_URL, &request);
    let record_id = record_id.to_string();
    spawn_local(async move {
        match JsFuture::from(promise).await {
            Ok(response_value) => {
                let Ok(response) = response_value.dyn_into::<Response>() else {
                    *status.borrow_mut() = Some("save failed: invalid bridge response".to_string());
                    return;
                };
                if response.ok() {
                    *status.borrow_mut() = Some(format!("saved visualizer into {record_id}.json"));
                } else {
                    *status.borrow_mut() =
                        Some("save failed: start `cargo run -p tty0-save-bridge`".to_string());
                }
            }
            Err(_) => {
                *status.borrow_mut() =
                    Some("save failed: start `cargo run -p tty0-save-bridge`".to_string());
            }
        }
    });
}
