use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    archive::{
        legacy_automation_to_timeline, ChromaticBulgeGridAutomationLanes, ChromaticBulgeGridClip,
        ChromaticBulgeGridClipTimeline, ChromaticBulgeGridShaderState,
        ChromaticBulgeGridShaderStates, ClipPlacement, ColorKeyframe, FloatKeyframe,
        InterpolationMode, PlaybackClock, RecordDocument, TrackVisualizerConfig,
        TrackVisualizerMode,
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
    state_field_index: usize,
    lane_index: usize,
    keyframe_index: usize,
    inspector_field: InspectorField,
    numeric_input: Option<NumericInputState>,
    text_input: Option<TextInputState>,
    status_message: Rc<RefCell<Option<String>>>,
    save_feedback: Rc<RefCell<Option<SaveFeedback>>>,
    migration_notice_shown: bool,
}

pub struct EditorDraftStore {
    drafts: HashMap<String, ChromaticBulgeGridEditorDraft>,
}

#[derive(Clone)]
pub struct ChromaticBulgeGridEditorDraft {
    pub record_id: String,
    pub original: TrackVisualizerConfig,
    pub working: TrackVisualizerConfig,
    pub clip_editor: ClipEditorDraft,
    pub opened_from_legacy: bool,
}

#[derive(Clone)]
pub struct ClipEditorDraft {
    pub selected_clip_id: SelectedClipId,
    pub selected_placement_index: SelectedPlacementIndex,
    pub clip_cursor_beat: f32,
    pub preview_mode: EditorPreviewMode,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct SelectedClipId(pub Option<String>);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectedPlacementIndex(pub Option<usize>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPreviewMode {
    TimelineWhilePaused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorFocusArea {
    StateParams,
    ClipLibrary,
    Arrangement,
    ClipLanes,
    ClipGraph,
    ClipInspector,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTab {
    State,
    Clips,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorField {
    Bpm,
    Measures,
    BeatsPerMeasure,
    LengthBeats,
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
pub struct TextInputState {
    pub target: TextEditTarget,
    pub buffer: String,
}

#[derive(Clone, Debug)]
pub enum NumericEditTarget {
    StateField(ParameterField),
    TimelineBpm,
    TimelineMeasures,
    TimelineBeatsPerMeasure,
    ClipLength,
    KeyframeBeat(LaneId),
    KeyframeValue(LaneId),
    KeyframeColor(LaneId, usize),
}

#[derive(Clone, Debug)]
pub enum TextEditTarget {
    ClipName(String),
}

#[derive(Clone)]
enum SaveFeedback {
    Saved {
        record_id: String,
        visualizer: TrackVisualizerConfig,
        migrated_legacy: bool,
    },
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
            state_field_index: 0,
            lane_index: 0,
            keyframe_index: 0,
            inspector_field: InspectorField::LengthBeats,
            numeric_input: None,
            text_input: None,
            status_message: Rc::new(RefCell::new(None)),
            save_feedback: Rc::new(RefCell::new(None)),
            migration_notice_shown: false,
        }
    }

    pub fn preview_visualizer(&self, record: &RecordDocument) -> Option<TrackVisualizerConfig> {
        self.drafts
            .draft_for(record.id.as_str())
            .map(|draft| draft.working.clone())
            .or_else(|| record.visualizer().cloned())
    }

    pub fn preview_playback_clock(
        &self,
        record: &RecordDocument,
        mut playback: PlaybackClock,
    ) -> PlaybackClock {
        if self.is_open {
            if let Some(draft) = self.drafts.draft_for(record.id.as_str()) {
                playback.timeline_preview =
                    draft.clip_editor.preview_mode.enables_timeline_preview();
            }
        }
        playback
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
            self.text_input = None;
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
        self.tab = EditorTab::Clips;
        self.focus_area = EditorFocusArea::ClipLibrary;
        self.numeric_input = None;
        self.text_input = None;
        self.sync_selection(record.id.as_str());
    }

    pub fn handle_key(&mut self, key: KeyCode, session: &mut SessionModel) -> bool {
        if !self.is_open {
            return false;
        }
        self.flush_save_feedback();

        let record_id = session.current_record().id.clone();
        if self.text_input.is_some() {
            return self.handle_text_key(key, &record_id);
        }
        if self.numeric_input.is_some() {
            return self.handle_numeric_key(key, &record_id);
        }

        match key {
            KeyCode::Esc | KeyCode::Char('e') | KeyCode::Char('E') => {
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
                self.tab = EditorTab::Clips;
                self.focus_area = EditorFocusArea::ClipLibrary;
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
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.save_current(&record_id);
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
            KeyCode::Left
                if self.tab == EditorTab::Clips
                    && self.focus_area != EditorFocusArea::ClipGraph =>
            {
                self.scrub_beats(session, -0.25);
                true
            }
            KeyCode::Right
                if self.tab == EditorTab::Clips
                    && self.focus_area != EditorFocusArea::ClipGraph =>
            {
                self.scrub_beats(session, 0.25);
                true
            }
            _ => self.handle_non_modal_key(key, session, &record_id),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, session: &SessionModel) {
        if !self.is_open {
            return;
        }
        self.flush_save_feedback();

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(6, 10, 12))),
            area,
        );
        let overlay = Block::bordered()
            .title(" tty0 shader clip sequencer ")
            .border_type(BorderType::Double)
            .style(Style::default().bg(Color::Rgb(10, 14, 16)))
            .border_style(Style::default().fg(CYAN));
        let inner = overlay.inner(area);
        frame.render_widget(overlay, area);

        if area.width < MIN_EDITOR_WIDTH || area.height < MIN_EDITOR_HEIGHT {
            frame.render_widget(
                Paragraph::new("editor requires a wider viewport in dev mode")
                    .style(Style::default().fg(AMBER).bg(BG)),
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
            EditorTab::Clips => self.render_clips_tab(frame, layout[2], draft, session),
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
        let timeline = draft.timeline();
        let authored = timeline.duration_secs();
        let mismatch = playback
            .duration_secs
            .map(|duration| (duration - authored).abs())
            .unwrap_or_default();
        let warning = if mismatch > 1.5 {
            format!("  drift warning {:.1}s", mismatch)
        } else {
            String::new()
        };
        let beat = current_global_beat(playback, timeline);
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
                    beat,
                    beat / timeline.beats_per_measure.max(1) as f32,
                    timeline.bpm,
                    timeline.measures,
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
            ("2 Clips", EditorTab::Clips),
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
        let state = draft.base_state();
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
                        .title(" base shader values ")
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
            area,
            &mut state,
        );
    }

    fn render_clips_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let layout = Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)]).split(area);
        self.render_clip_library(frame, layout[0], draft);

        let right =
            Layout::vertical([Constraint::Percentage(40), Constraint::Fill(1)]).split(layout[1]);
        self.render_arrangement(frame, right[0], draft, session);
        self.render_clip_editor(frame, right[1], draft, session);
    }

    fn render_clip_library(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let timeline = draft.timeline();
        let items = timeline
            .clips
            .iter()
            .map(|clip| {
                let selected =
                    draft.clip_editor.selected_clip_id.0.as_deref() == Some(clip.id.as_str());
                let header_style = if selected {
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(WHITE)
                };
                let preview = build_clip_preview(clip);
                ListItem::new(vec![
                    Line::from(vec![Span::styled(
                        format!("{} {}", format_color_tag(clip.color), clip.name),
                        header_style,
                    )]),
                    Line::from(format!(
                        "{:.2} beats / {:.2} measures",
                        clip.length_beats,
                        clip.length_beats / timeline.beats_per_measure.max(1) as f32
                    )),
                    Line::from(format!(
                        "{} placements  {} lanes",
                        draft.placement_count_for_clip(&clip.id),
                        clip.lanes.automated_lane_count()
                    )),
                    Line::from(format!("preview {}", preview)),
                ])
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(draft.selected_clip_index());
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .title(" clip library ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::ClipLibrary {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                )
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(18, 35, 33))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(""),
            area,
            &mut state,
        );
    }

    fn render_arrangement(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let timeline = draft.timeline();
        let playback = session.playback_clock();
        let beat = current_global_beat(playback, timeline);
        let mut lines = vec![arrangement_status_line(draft, beat)];

        if timeline.arrangement.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("no placements on the arrangement lane"));
            lines.push(Line::from("A add placement of selected clip at playhead"));
        } else {
            lines.extend(build_arrangement_grid(
                draft,
                beat,
                area.width.saturating_sub(4) as usize,
            ));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" arrangement ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::Arrangement {
                                CYAN
                            } else {
                                BORDER
                            },
                        )),
                ),
            area,
        );
    }

    fn render_clip_editor(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let layout = Layout::horizontal([
            Constraint::Length(24),
            Constraint::Fill(1),
            Constraint::Length(36),
        ])
        .split(area);
        self.render_lane_list(frame, layout[0], draft);
        self.render_clip_graph(frame, layout[1], draft, session);
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
                ListItem::new(format!(
                    "{:<18} {:>2} keys",
                    lane.label(),
                    draft.keyframe_count(*lane)
                ))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(self.lane_index));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .title(" clip lanes ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::ClipLanes {
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

    fn render_clip_graph(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let lane = self.selected_lane();
        let Some(clip) = draft.selected_clip() else {
            frame.render_widget(
                Paragraph::new("no clip selected")
                    .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                    .block(Block::bordered().title(" clip graph ").border_style(
                        Style::default().fg(if self.focus_area == EditorFocusArea::ClipGraph {
                            CYAN
                        } else {
                            BORDER
                        }),
                    )),
                area,
            );
            return;
        };

        let cursor = draft
            .clip_editor
            .clip_cursor_beat
            .clamp(0.0, clip.length_beats);
        let ghost = draft.live_local_playback_beat(session.playback_clock());
        let graph_width = area.width.saturating_sub(6).max(16) as usize;
        let mut lines = vec![
            Line::from(format!(
                "clip: {}  lane: {}  cursor {:>5.2}/{:>5.2}",
                clip.name,
                lane.label(),
                cursor,
                clip.length_beats
            )),
            Line::from(format!(
                "base {}  sampled {}",
                format_lane_value(base_lane_value(draft.base_state(), lane)),
                draft.sampled_value_at_cursor(lane)
            )),
            Line::from(""),
        ];
        lines.extend(build_local_clip_graph(
            draft,
            lane,
            clip.length_beats,
            cursor,
            ghost,
            graph_width,
            self.keyframe_index,
        ));
        lines.push(Line::from(""));
        lines.extend(render_selected_lane_keyframes(
            draft,
            lane,
            self.keyframe_index,
        ));

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" clip graph ")
                        .border_style(Style::default().fg(
                            if self.focus_area == EditorFocusArea::ClipGraph {
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
        let timeline = draft.timeline();
        let mut lines = vec![
            render_inspector_line(
                "bpm",
                format!("{:.2}", timeline.bpm),
                self.inspector_field == InspectorField::Bpm,
            ),
            render_inspector_line(
                "measures",
                timeline.measures.to_string(),
                self.inspector_field == InspectorField::Measures,
            ),
            render_inspector_line(
                "beats/bar",
                timeline.beats_per_measure.to_string(),
                self.inspector_field == InspectorField::BeatsPerMeasure,
            ),
        ];

        if let Some(clip) = draft.selected_clip() {
            lines.push(render_inspector_line(
                "clip_len",
                format!("{:.2}", clip.length_beats),
                self.inspector_field == InspectorField::LengthBeats,
            ));
        } else {
            lines.push(Line::from("clip_len none"));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "lane base: {}",
            format_lane_value(base_lane_value(draft.base_state(), lane))
        )));
        lines.push(Line::from(format!(
            "sampled @ cursor: {}",
            draft.sampled_value_at_cursor(lane)
        )));
        if let Some(local) = draft.live_local_playback_beat(session.playback_clock()) {
            lines.push(Line::from(format!("live local beat: {:.2}", local)));
        }

        lines.push(Line::from(""));
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
                .block(Block::bordered().title(" clip inspector ").border_style(
                    Style::default().fg(if self.focus_area == EditorFocusArea::ClipInspector {
                        CYAN
                    } else {
                        BORDER
                    }),
                )),
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
        let status = if let Some(input) = &self.text_input {
            format!("renaming clip = {}", input.buffer)
        } else if let Some(input) = &self.numeric_input {
            format!("editing {} = {}", input.target.label(), input.buffer)
        } else {
            self.status_message.borrow().clone().unwrap_or_else(|| {
                "Tab focus  P play/pause  S save  E close  arrows scrub / edit based on focus"
                    .to_string()
            })
        };
        let lines = vec![Line::from(vec![
            Span::styled(status, Style::default().fg(AMBER)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "dirty={}  focus={}",
                    draft.is_dirty(),
                    self.focus_area.label()
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

    fn handle_non_modal_key(
        &mut self,
        key: KeyCode,
        session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match self.tab {
            EditorTab::State => self.handle_state_key(key, record_id),
            EditorTab::Clips => self.handle_clips_key(key, session, record_id),
            EditorTab::Export => self.handle_export_key(key, record_id),
        }
    }

    fn handle_state_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match (self.focus_area, key) {
            (EditorFocusArea::StateParams, KeyCode::Up) => {
                self.state_field_index = self.state_field_index.saturating_sub(1);
            }
            (EditorFocusArea::StateParams, KeyCode::Down) => {
                self.state_field_index =
                    (self.state_field_index + 1).min(ParameterField::ALL.len().saturating_sub(1));
            }
            (EditorFocusArea::StateParams, KeyCode::Enter) => {
                self.begin_state_numeric_edit(record_id)
            }
            (EditorFocusArea::StateParams, KeyCode::Char('-')) => {
                self.nudge_state_field(record_id, false, -1.0)
            }
            (EditorFocusArea::StateParams, KeyCode::Char('=')) => {
                self.nudge_state_field(record_id, false, 1.0)
            }
            (EditorFocusArea::StateParams, KeyCode::Char('_')) => {
                self.nudge_state_field(record_id, true, -1.0)
            }
            (EditorFocusArea::StateParams, KeyCode::Char('+')) => {
                self.nudge_state_field(record_id, true, 1.0)
            }
            _ => {}
        }
        true
    }

    fn handle_clips_key(
        &mut self,
        key: KeyCode,
        session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match self.focus_area {
            EditorFocusArea::ClipLibrary => self.handle_clip_library_key(key, record_id),
            EditorFocusArea::Arrangement => self.handle_arrangement_key(key, session, record_id),
            EditorFocusArea::ClipLanes => self.handle_clip_lanes_key(key, record_id),
            EditorFocusArea::ClipGraph => self.handle_clip_graph_key(key, record_id),
            EditorFocusArea::ClipInspector => {
                self.handle_clip_inspector_key(key, session, record_id)
            }
            _ => true,
        }
    }

    fn handle_clip_library_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match key {
            KeyCode::Up => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_clip_delta(-1);
                }
                self.sync_selection(record_id);
            }
            KeyCode::Down => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_clip_delta(1);
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    let name = draft.create_empty_clip();
                    self.set_status(format!("created {}", name));
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(name) = draft.duplicate_selected_clip() {
                        self.set_status(format!("duplicated {}", name));
                    } else {
                        self.set_status("no selected clip to duplicate");
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.begin_clip_rename(record_id),
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.delete_selected_clip() {
                        Ok(Some(name)) => self.set_status(format!("deleted {}", name)),
                        Ok(None) => self.set_status("no selected clip"),
                        Err(message) => self.set_status(message),
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Enter => self.focus_area = EditorFocusArea::ClipGraph,
            _ => {}
        }
        true
    }

    fn handle_arrangement_key(
        &mut self,
        key: KeyCode,
        session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char(',') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_placement_delta(-1);
                }
                self.sync_selection(record_id);
            }
            KeyCode::Down | KeyCode::Char('.') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_placement_delta(1);
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                let beat = self.current_global_beat_for_record(session, record_id);
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.add_placement_at_playhead(beat) {
                        Ok(value) => self.set_status(format!("added placement at {:.2}", value)),
                        Err(message) => self.set_status(message),
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                let beat = self.current_global_beat_for_record(session, record_id);
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.move_selected_placement_to(beat) {
                        Ok(()) => self.set_status(format!("moved placement to {:.2}", beat)),
                        Err(message) => self.set_status(message),
                    }
                }
            }
            KeyCode::Char('-') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.adjust_selected_placement_repeats(-1) {
                        Ok(repeats) => self.set_status(format!("repeats {}", repeats)),
                        Err(message) => self.set_status(message),
                    }
                }
            }
            KeyCode::Char('=') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.adjust_selected_placement_repeats(1) {
                        Ok(repeats) => self.set_status(format!("repeats {}", repeats)),
                        Err(message) => self.set_status(message),
                    }
                }
            }
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if draft.delete_selected_placement() {
                        self.set_status("placement deleted");
                    } else {
                        self.set_status("no placement selected");
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Enter => {
                let beat = self.current_global_beat_for_record(session, record_id);
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if draft.select_placement_at_beat(beat) {
                        self.set_status("selected placement under playhead");
                    } else {
                        self.set_status("no placement under playhead");
                    }
                }
                self.sync_selection(record_id);
            }
            _ => {}
        }
        true
    }

    fn handle_clip_lanes_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match key {
            KeyCode::Up => {
                self.lane_index = self.lane_index.saturating_sub(1);
                self.sync_selection(record_id);
            }
            KeyCode::Down => {
                self.lane_index = (self.lane_index + 1).min(LaneId::ALL.len().saturating_sub(1));
                self.sync_selection(record_id);
            }
            KeyCode::Enter => self.focus_area = EditorFocusArea::ClipGraph,
            _ => {}
        }
        true
    }

    fn handle_clip_graph_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match key {
            KeyCode::Left => self.move_clip_cursor(record_id, -0.25),
            KeyCode::Right => self.move_clip_cursor(record_id, 0.25),
            KeyCode::Home => self.set_clip_cursor(record_id, 0.0),
            KeyCode::End => {
                if let Some(draft) = self.drafts.draft_for(record_id) {
                    if let Some(clip) = draft.selected_clip() {
                        self.set_clip_cursor(record_id, clip.length_beats);
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(index) = draft.insert_keyframe_at_cursor(lane) {
                        self.keyframe_index = index;
                        self.set_status("inserted sampled keyframe");
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(index) = draft.clone_keyframe_to_cursor(lane, self.keyframe_index) {
                        self.keyframe_index = index;
                        self.set_status("cloned keyframe to cursor");
                    } else {
                        self.set_status("no selected keyframe to clone");
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.move_selected_keyframe_to_cursor(lane, self.keyframe_index) {
                        Ok(Some(index)) => {
                            self.keyframe_index = index;
                            self.set_status("moved keyframe to cursor");
                        }
                        Ok(None) => self.set_status("no selected keyframe"),
                        Err(message) => self.set_status(message),
                    }
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char(',') => self.step_selected_keyframe(record_id, -1),
            KeyCode::Char('.') => self.step_selected_keyframe(record_id, 1),
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('d') | KeyCode::Char('D') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.delete_keyframe(lane, self.keyframe_index);
                    self.set_status("keyframe deleted");
                }
                self.sync_selection(record_id);
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.cycle_interpolation(lane, self.keyframe_index);
                }
                self.inspector_field = InspectorField::Interpolation;
                self.set_status("interpolation toggled");
            }
            KeyCode::Enter => self.begin_clip_numeric_edit(record_id),
            _ => {}
        }
        true
    }

    fn handle_clip_inspector_key(
        &mut self,
        key: KeyCode,
        _session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match key {
            KeyCode::Up => self.inspector_field = self.inspector_field.prev(self.selected_lane()),
            KeyCode::Down => self.inspector_field = self.inspector_field.next(self.selected_lane()),
            KeyCode::Char('-') => self.nudge_clip_field(record_id, false, -1.0),
            KeyCode::Char('=') => self.nudge_clip_field(record_id, false, 1.0),
            KeyCode::Char('_') => self.nudge_clip_field(record_id, true, -1.0),
            KeyCode::Char('+') => self.nudge_clip_field(record_id, true, 1.0),
            KeyCode::Enter => self.begin_clip_numeric_edit(record_id),
            _ => {}
        }
        true
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
            KeyCode::Char('s') | KeyCode::Char('S') => self.save_current(record_id),
            _ => {}
        }
        true
    }

    fn handle_numeric_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        let Some(input) = &mut self.numeric_input else {
            return false;
        };
        match key {
            KeyCode::Esc => self.numeric_input = None,
            KeyCode::Enter => {
                let buffer = input.buffer.clone();
                let target = input.target.clone();
                self.numeric_input = None;
                self.commit_numeric_edit(record_id, target, buffer);
            }
            KeyCode::Backspace => {
                if input.replace_on_type {
                    input.buffer.clear();
                    input.replace_on_type = false;
                } else {
                    input.buffer.pop();
                }
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == '.' || ch == '-' => {
                if input.replace_on_type {
                    input.buffer.clear();
                    input.replace_on_type = false;
                }
                input.buffer.push(ch);
            }
            _ => {}
        }
        true
    }

    fn handle_text_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        let Some(input) = &mut self.text_input else {
            return false;
        };
        match key {
            KeyCode::Esc => self.text_input = None,
            KeyCode::Enter => {
                let buffer = input.buffer.clone();
                let target = input.target.clone();
                self.text_input = None;
                self.commit_text_edit(record_id, target, buffer);
            }
            KeyCode::Backspace => {
                input.buffer.pop();
            }
            KeyCode::Char(ch) if ch.is_ascii_graphic() || ch == ' ' => {
                input.buffer.push(ch);
            }
            _ => {}
        }
        true
    }

    fn begin_state_numeric_edit(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let field = self.selected_state_field();
        let state = draft.base_state();
        let value = format_parameter_value(state, field);
        self.numeric_input = Some(NumericInputState {
            target: NumericEditTarget::StateField(field),
            buffer: value,
            replace_on_type: true,
        });
        self.set_status(format!("editing {}", field.label()));
    }

    fn begin_clip_numeric_edit(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let target = match self.inspector_field {
            InspectorField::Bpm => NumericEditTarget::TimelineBpm,
            InspectorField::Measures => NumericEditTarget::TimelineMeasures,
            InspectorField::BeatsPerMeasure => NumericEditTarget::TimelineBeatsPerMeasure,
            InspectorField::LengthBeats => NumericEditTarget::ClipLength,
            InspectorField::Beat => NumericEditTarget::KeyframeBeat(self.selected_lane()),
            InspectorField::Value => NumericEditTarget::KeyframeValue(self.selected_lane()),
            InspectorField::Interpolation => {
                self.set_status("press I to toggle interpolation");
                return;
            }
            InspectorField::Red => NumericEditTarget::KeyframeColor(self.selected_lane(), 0),
            InspectorField::Green => NumericEditTarget::KeyframeColor(self.selected_lane(), 1),
            InspectorField::Blue => NumericEditTarget::KeyframeColor(self.selected_lane(), 2),
        };
        let buffer = match target {
            NumericEditTarget::TimelineBpm => format!("{:.2}", draft.timeline().bpm),
            NumericEditTarget::TimelineMeasures => draft.timeline().measures.to_string(),
            NumericEditTarget::TimelineBeatsPerMeasure => {
                draft.timeline().beats_per_measure.to_string()
            }
            NumericEditTarget::ClipLength => draft
                .selected_clip()
                .map(|clip| format!("{:.3}", clip.length_beats))
                .unwrap_or_else(|| "1.0".to_string()),
            NumericEditTarget::KeyframeBeat(lane)
            | NumericEditTarget::KeyframeValue(lane)
            | NumericEditTarget::KeyframeColor(lane, _) => draft
                .keyframe_component(lane, self.keyframe_index, self.inspector_field)
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "0.0".to_string()),
            NumericEditTarget::StateField(_) => String::new(),
        };
        self.numeric_input = Some(NumericInputState {
            target,
            buffer,
            replace_on_type: true,
        });
        self.set_status(format!("editing {}", self.inspector_field.label()));
    }

    fn begin_clip_rename(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let Some(clip) = draft.selected_clip() else {
            self.set_status("no selected clip");
            return;
        };
        self.text_input = Some(TextInputState {
            target: TextEditTarget::ClipName(clip.id.clone()),
            buffer: clip.name.clone(),
        });
        self.set_status("renaming clip");
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
            NumericEditTarget::StateField(field) => draft.set_base_field(field, value),
            NumericEditTarget::TimelineBpm => draft.timeline_mut().bpm = value.max(1.0),
            NumericEditTarget::TimelineMeasures => {
                draft.timeline_mut().measures = value.max(1.0).round() as u32
            }
            NumericEditTarget::TimelineBeatsPerMeasure => {
                draft.timeline_mut().beats_per_measure = value.max(1.0).round() as u32
            }
            NumericEditTarget::ClipLength => {
                match draft.set_selected_clip_length(value.max(0.25)) {
                    Ok(()) => {}
                    Err(message) => {
                        self.set_status(message);
                        return;
                    }
                }
            }
            NumericEditTarget::KeyframeBeat(lane) => {
                match draft.move_keyframe_to_beat(lane, self.keyframe_index, value.max(0.0)) {
                    Ok(Some(index)) => self.keyframe_index = index,
                    Ok(None) => {}
                    Err(message) => {
                        self.set_status(message);
                        return;
                    }
                }
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

    fn commit_text_edit(&mut self, record_id: &str, target: TextEditTarget, buffer: String) {
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            return;
        };
        match target {
            TextEditTarget::ClipName(clip_id) => {
                if buffer.trim().is_empty() {
                    self.set_status("clip name cannot be empty");
                    return;
                }
                if draft.rename_clip(&clip_id, buffer.trim()) {
                    self.set_status("clip renamed");
                } else {
                    self.set_status("clip rename failed");
                }
            }
        }
    }

    fn nudge_state_field(&mut self, record_id: &str, coarse: bool, direction: f32) {
        let field = self.selected_state_field();
        let step = parameter_step(field, coarse) * direction;
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            return;
        };
        let current = draft.parameter_value(field);
        draft.set_base_field(field, current + step);
        self.set_status(format!("base {} by {:.3}", field.label(), step));
    }

    fn nudge_clip_field(&mut self, record_id: &str, coarse: bool, direction: f32) {
        let lane = self.selected_lane();
        let field = self.inspector_field;
        let step = inspector_step(field, lane, coarse) * direction;
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            return;
        };
        match field {
            InspectorField::Bpm => {
                draft.timeline_mut().bpm = (draft.timeline().bpm + step).max(1.0);
            }
            InspectorField::Measures => {
                draft.timeline_mut().measures =
                    ((draft.timeline().measures as f32) + step).max(1.0).round() as u32;
            }
            InspectorField::BeatsPerMeasure => {
                draft.timeline_mut().beats_per_measure =
                    ((draft.timeline().beats_per_measure as f32) + step)
                        .max(1.0)
                        .round() as u32;
            }
            InspectorField::LengthBeats => {
                if let Err(message) = draft.set_selected_clip_length(
                    draft.selected_clip().map_or(1.0, |clip| clip.length_beats) + step,
                ) {
                    self.set_status(message);
                    return;
                }
            }
            InspectorField::Beat => {
                if let Some(current) = draft.keyframe_beat(lane, self.keyframe_index) {
                    match draft.move_keyframe_to_beat(lane, self.keyframe_index, current + step) {
                        Ok(Some(index)) => self.keyframe_index = index,
                        Ok(None) => {}
                        Err(message) => {
                            self.set_status(message);
                            return;
                        }
                    }
                }
            }
            InspectorField::Value => {
                if let Some(current) =
                    draft.keyframe_component(lane, self.keyframe_index, InspectorField::Value)
                {
                    draft.set_keyframe_value(lane, self.keyframe_index, current + step);
                }
            }
            InspectorField::Red | InspectorField::Green | InspectorField::Blue => {
                let channel = match field {
                    InspectorField::Red => 0,
                    InspectorField::Green => 1,
                    InspectorField::Blue => 2,
                    _ => unreachable!(),
                };
                if let Some(current) = draft.keyframe_component(lane, self.keyframe_index, field) {
                    draft.set_keyframe_color(lane, self.keyframe_index, channel, current + step);
                }
            }
            InspectorField::Interpolation => {
                self.set_status("press I to toggle interpolation");
                return;
            }
        }
        self.sync_selection(record_id);
    }

    fn step_selected_keyframe(&mut self, record_id: &str, delta: isize) {
        let max_index = self
            .drafts
            .draft_for(record_id)
            .map(|draft| draft.keyframe_count(self.selected_lane()))
            .unwrap_or_default()
            .saturating_sub(1) as isize;
        self.keyframe_index = (self.keyframe_index as isize + delta).clamp(0, max_index) as usize;
        self.set_status(format!("selected keyframe {}", self.keyframe_index + 1));
    }

    fn move_clip_cursor(&mut self, record_id: &str, delta: f32) {
        if let Some(draft) = self.drafts.draft_mut(record_id) {
            if let Some(clip) = draft.selected_clip() {
                draft.clip_editor.clip_cursor_beat =
                    (draft.clip_editor.clip_cursor_beat + delta).clamp(0.0, clip.length_beats);
            }
        }
    }

    fn set_clip_cursor(&mut self, record_id: &str, beat: f32) {
        if let Some(draft) = self.drafts.draft_mut(record_id) {
            if let Some(clip) = draft.selected_clip() {
                draft.clip_editor.clip_cursor_beat = beat.clamp(0.0, clip.length_beats);
            }
        }
    }

    fn save_current(&mut self, record_id: &str) {
        if let Some(draft) = self.drafts.draft_for(record_id) {
            save_visualizer_to_record(
                record_id,
                draft.working.normalized_for_export(),
                draft.opened_from_legacy,
                Rc::clone(&self.status_message),
                Rc::clone(&self.save_feedback),
            );
        }
    }

    fn scrub_beats(&mut self, session: &mut SessionModel, delta_beats: f32) {
        let Some(draft) = self.drafts.draft_for(session.current_record().id.as_str()) else {
            return;
        };
        let bpm = draft.timeline().bpm.max(1.0);
        let playback = session.playback_clock();
        let next_secs = playback.current_time_secs + delta_beats * 60.0 / bpm;
        session.seek_to_secs(next_secs.max(0.0));
    }

    fn scrub_measures(&mut self, session: &mut SessionModel, delta_measures: f32) {
        let Some(draft) = self.drafts.draft_for(session.current_record().id.as_str()) else {
            return;
        };
        let delta_beats = delta_measures * draft.timeline().beats_per_measure as f32;
        self.scrub_beats(session, delta_beats);
    }

    fn current_global_beat_for_record(&self, session: &SessionModel, record_id: &str) -> f32 {
        self.drafts
            .draft_for(record_id)
            .map(|draft| current_global_beat(session.playback_clock(), draft.timeline()))
            .unwrap_or_default()
    }

    fn sync_selection(&mut self, record_id: &str) {
        let lane = self.selected_lane();
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            self.keyframe_index = 0;
            return;
        };
        draft.ensure_selection();
        if let Some(clip) = draft.selected_clip() {
            draft.clip_editor.clip_cursor_beat = draft
                .clip_editor
                .clip_cursor_beat
                .clamp(0.0, clip.length_beats);
        } else {
            draft.clip_editor.clip_cursor_beat = 0.0;
        }
        self.keyframe_index = self
            .keyframe_index
            .min(draft.keyframe_count(lane).saturating_sub(1));
        if !lane.is_color()
            && matches!(
                self.inspector_field,
                InspectorField::Red | InspectorField::Green | InspectorField::Blue
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

    fn flush_save_feedback(&mut self) {
        let feedback = self.save_feedback.borrow_mut().take();
        let Some(feedback) = feedback else {
            return;
        };
        match feedback {
            SaveFeedback::Saved {
                record_id,
                visualizer,
                migrated_legacy,
            } => {
                if let Some(draft) = self.drafts.draft_mut(record_id.as_str()) {
                    draft.mark_saved(visualizer);
                }
                if migrated_legacy && !self.migration_notice_shown {
                    self.migration_notice_shown = true;
                    self.set_status(
                        "save will migrate legacy lane automation to clip timeline format",
                    );
                }
            }
        }
    }
}

impl EditorDraftStore {
    fn ensure(&mut self, record: &RecordDocument) {
        if self.drafts.contains_key(record.id.as_str()) {
            return;
        }

        let original = record
            .visualizer()
            .cloned()
            .unwrap_or_else(default_visualizer_config)
            .with_synced_base_states();
        let opened_from_legacy = original.timeline.is_none() && original.automation.is_some();
        let working_timeline = if let Some(timeline) = &original.timeline {
            timeline.clone()
        } else if let Some(automation) = &original.automation {
            legacy_automation_to_timeline(automation)
        } else {
            default_timeline()
        };
        let selected_clip_id = working_timeline.clips.first().map(|clip| clip.id.clone());
        let working = TrackVisualizerConfig {
            mode: original.mode,
            params: original.params.clone(),
            automation: None,
            timeline: Some(working_timeline),
        }
        .with_synced_base_states();

        self.drafts.insert(
            record.id.clone(),
            ChromaticBulgeGridEditorDraft {
                record_id: record.id.clone(),
                original,
                working,
                clip_editor: ClipEditorDraft {
                    selected_clip_id: SelectedClipId(selected_clip_id),
                    selected_placement_index: SelectedPlacementIndex(None),
                    clip_cursor_beat: 0.0,
                    preview_mode: EditorPreviewMode::TimelineWhilePaused,
                },
                opened_from_legacy,
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
    pub fn timeline(&self) -> &ChromaticBulgeGridClipTimeline {
        self.working
            .timeline
            .as_ref()
            .expect("editor draft timeline should exist")
    }

    pub fn timeline_mut(&mut self) -> &mut ChromaticBulgeGridClipTimeline {
        self.working
            .timeline
            .as_mut()
            .expect("editor draft timeline should exist")
    }

    pub fn base_state(&self) -> ChromaticBulgeGridShaderState {
        self.working.params.shader_states.unwrap().playing
    }

    pub fn parameter_value(&self, field: ParameterField) -> f32 {
        parameter_value(self.base_state(), field)
    }

    pub fn is_dirty(&self) -> bool {
        self.original.normalized_for_export() != self.working.normalized_for_export()
    }

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        to_string_pretty(&self.working.normalized_for_export())
    }

    pub fn mark_saved(&mut self, visualizer: TrackVisualizerConfig) {
        self.original = visualizer.with_synced_base_states().normalized_for_export();
        self.opened_from_legacy = false;
    }

    pub fn ensure_selection(&mut self) {
        let selected_clip = self.clip_editor.selected_clip_id.0.clone();
        let fallback = self.timeline().clips.first().map(|clip| clip.id.clone());
        if let Some(selected) = selected_clip {
            if self.timeline().clip_by_id(&selected).is_none() {
                self.clip_editor.selected_clip_id.0 = fallback;
            }
        } else {
            self.clip_editor.selected_clip_id.0 = fallback;
        }
        if let Some(index) = self.clip_editor.selected_placement_index.0 {
            if index >= self.timeline().arrangement.len() {
                self.clip_editor.selected_placement_index.0 =
                    self.timeline().arrangement.len().checked_sub(1);
            }
        }
    }

    pub fn selected_clip(&self) -> Option<&ChromaticBulgeGridClip> {
        self.clip_editor
            .selected_clip_id
            .0
            .as_deref()
            .and_then(|clip_id| self.timeline().clip_by_id(clip_id))
    }

    pub fn selected_clip_mut(&mut self) -> Option<&mut ChromaticBulgeGridClip> {
        let clip_id = self.clip_editor.selected_clip_id.0.clone()?;
        self.timeline_mut()
            .clips
            .iter_mut()
            .find(|clip| clip.id == clip_id)
    }

    pub fn selected_clip_index(&self) -> Option<usize> {
        let clip_id = self.clip_editor.selected_clip_id.0.as_deref()?;
        self.timeline()
            .clips
            .iter()
            .position(|clip| clip.id == clip_id)
    }

    pub fn selected_placement(&self) -> Option<&ClipPlacement> {
        self.clip_editor
            .selected_placement_index
            .0
            .and_then(|index| self.timeline().arrangement.get(index))
    }

    pub fn placement_count_for_clip(&self, clip_id: &str) -> usize {
        self.timeline()
            .arrangement
            .iter()
            .filter(|placement| placement.clip_id == clip_id)
            .count()
    }

    pub fn select_clip_delta(&mut self, delta: isize) {
        let len = self.timeline().clips.len() as isize;
        if len == 0 {
            self.clip_editor.selected_clip_id.0 = None;
            return;
        }
        let current = self.selected_clip_index().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, len - 1) as usize;
        if let Some(clip) = self.timeline().clips.get(next) {
            self.clip_editor.selected_clip_id.0 = Some(clip.id.clone());
        }
    }

    pub fn create_empty_clip(&mut self) -> String {
        let index = self.timeline().clips.len() + 1;
        let id = self.next_clip_id("clip");
        let name = format!("Clip {}", index);
        self.timeline_mut().clips.push(ChromaticBulgeGridClip {
            id: id.clone(),
            name: name.clone(),
            length_beats: 4.0,
            color: default_clip_color(id.len()),
            lanes: ChromaticBulgeGridAutomationLanes::default(),
        });
        self.clip_editor.selected_clip_id.0 = Some(id);
        name
    }

    pub fn duplicate_selected_clip(&mut self) -> Option<String> {
        let clip = self.selected_clip()?.clone();
        let id = self.next_clip_id(&clip.id);
        let name = format!("{} Copy", clip.name);
        self.timeline_mut().clips.push(ChromaticBulgeGridClip {
            id: id.clone(),
            name: name.clone(),
            length_beats: clip.length_beats,
            color: clip.color,
            lanes: clip.lanes,
        });
        self.clip_editor.selected_clip_id.0 = Some(id);
        Some(name)
    }

    pub fn rename_clip(&mut self, clip_id: &str, name: &str) -> bool {
        let Some(clip) = self
            .timeline_mut()
            .clips
            .iter_mut()
            .find(|clip| clip.id == clip_id)
        else {
            return false;
        };
        clip.name = name.to_string();
        true
    }

    pub fn delete_selected_clip(&mut self) -> Result<Option<String>, String> {
        let Some(clip_id) = self.clip_editor.selected_clip_id.0.clone() else {
            return Ok(None);
        };
        if self
            .timeline()
            .arrangement
            .iter()
            .any(|placement| placement.clip_id == clip_id)
        {
            return Err("cannot delete clip with placements".to_string());
        }
        let Some(index) = self
            .timeline()
            .clips
            .iter()
            .position(|clip| clip.id == clip_id)
        else {
            return Ok(None);
        };
        let removed = self.timeline_mut().clips.remove(index);
        self.clip_editor.selected_clip_id.0 =
            self.timeline().clips.first().map(|clip| clip.id.clone());
        Ok(Some(removed.name))
    }

    pub fn select_placement_delta(&mut self, delta: isize) {
        let len = self.timeline().arrangement.len() as isize;
        if len == 0 {
            self.clip_editor.selected_placement_index.0 = None;
            return;
        }
        let current = self.clip_editor.selected_placement_index.0.unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, len - 1) as usize;
        self.clip_editor.selected_placement_index.0 = Some(next);
        if let Some(placement) = self.timeline().arrangement.get(next) {
            self.clip_editor.selected_clip_id.0 = Some(placement.clip_id.clone());
        }
    }

    pub fn select_placement_at_beat(&mut self, beat: f32) -> bool {
        let Some(index) = self.placement_index_at_beat(beat) else {
            return false;
        };
        self.clip_editor.selected_placement_index.0 = Some(index);
        if let Some(placement) = self.timeline().arrangement.get(index) {
            self.clip_editor.selected_clip_id.0 = Some(placement.clip_id.clone());
        }
        true
    }

    pub fn add_placement_at_playhead(&mut self, beat: f32) -> Result<f32, String> {
        let Some(clip) = self.selected_clip().cloned() else {
            return Err("no selected clip".to_string());
        };
        let placement = ClipPlacement {
            clip_id: clip.id.clone(),
            start_beat: beat.max(0.0),
            repeats: 1,
        };
        self.validate_placement(None, &placement)?;
        self.timeline_mut().arrangement.push(placement);
        self.timeline_mut()
            .arrangement
            .sort_by(|left, right| left.start_beat.total_cmp(&right.start_beat));
        let index = self
            .timeline()
            .arrangement
            .iter()
            .position(|candidate| {
                candidate.clip_id == clip.id
                    && (candidate.start_beat - beat).abs() < 0.0001
                    && candidate.repeats == 1
            })
            .unwrap_or(0);
        self.clip_editor.selected_placement_index.0 = Some(index);
        Ok(beat)
    }

    pub fn move_selected_placement_to(&mut self, beat: f32) -> Result<(), String> {
        let Some(index) = self.clip_editor.selected_placement_index.0 else {
            return Err("no selected placement".to_string());
        };
        let mut placement = self.timeline().arrangement[index].clone();
        placement.start_beat = beat.max(0.0);
        self.validate_placement(Some(index), &placement)?;
        self.timeline_mut().arrangement[index] = placement;
        self.timeline_mut()
            .arrangement
            .sort_by(|left, right| left.start_beat.total_cmp(&right.start_beat));
        self.clip_editor.selected_placement_index.0 = self.placement_index_at_beat(beat);
        Ok(())
    }

    pub fn adjust_selected_placement_repeats(&mut self, delta: i32) -> Result<u32, String> {
        let Some(index) = self.clip_editor.selected_placement_index.0 else {
            return Err("no selected placement".to_string());
        };
        let mut placement = self.timeline().arrangement[index].clone();
        placement.repeats = (placement.repeats as i32 + delta).max(1) as u32;
        self.validate_placement(Some(index), &placement)?;
        self.timeline_mut().arrangement[index] = placement.clone();
        Ok(placement.repeats)
    }

    pub fn delete_selected_placement(&mut self) -> bool {
        let Some(index) = self.clip_editor.selected_placement_index.0 else {
            return false;
        };
        if index >= self.timeline().arrangement.len() {
            return false;
        }
        self.timeline_mut().arrangement.remove(index);
        self.clip_editor.selected_placement_index.0 = if self.timeline().arrangement.is_empty() {
            None
        } else {
            Some(index.min(self.timeline().arrangement.len() - 1))
        };
        true
    }

    pub fn live_local_playback_beat(&self, playback: PlaybackClock) -> Option<f32> {
        let placement = self.selected_placement()?;
        if self.clip_editor.selected_clip_id.0.as_deref() != Some(placement.clip_id.as_str()) {
            return None;
        }
        let clip = self.timeline().clip_by_id(&placement.clip_id)?;
        let beat = current_global_beat(playback, self.timeline());
        let end = placement.end_beat(clip);
        if beat < placement.start_beat || beat >= end {
            return None;
        }
        Some(((beat - placement.start_beat) % clip.length_beats).clamp(0.0, clip.length_beats))
    }

    pub fn sampled_value_at_cursor(&self, lane: LaneId) -> String {
        let Some(clip) = self.selected_clip() else {
            return "--".to_string();
        };
        let state = clip.apply_to_state(self.base_state(), self.clip_editor.clip_cursor_beat);
        format_lane_value(base_lane_value(state, lane))
    }

    pub fn keyframe_count(&self, lane: LaneId) -> usize {
        if lane.is_color() {
            self.color_keyframes(lane).len()
        } else {
            self.float_keyframes(lane).len()
        }
    }

    pub fn float_keyframes(&self, lane: LaneId) -> &[FloatKeyframe] {
        self.selected_clip()
            .map(|clip| float_lane(&clip.lanes, lane))
            .unwrap_or(&[])
    }

    pub fn color_keyframes(&self, lane: LaneId) -> &[ColorKeyframe] {
        self.selected_clip()
            .map(|clip| color_lane(&clip.lanes, lane))
            .unwrap_or(&[])
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
                InspectorField::Beat => Some(keyframe.beat),
                InspectorField::Red | InspectorField::Value => Some(keyframe.value[0]),
                InspectorField::Green => Some(keyframe.value[1]),
                InspectorField::Blue => Some(keyframe.value[2]),
                _ => None,
            }
        } else {
            let keyframe = self.selected_float_keyframe(lane, index)?;
            match field {
                InspectorField::Beat => Some(keyframe.beat),
                InspectorField::Value => Some(keyframe.value),
                _ => None,
            }
        }
    }

    pub fn set_base_field(&mut self, field: ParameterField, value: f32) {
        let states = self.working.params.shader_states.as_mut().unwrap();
        set_parameter_field(&mut states.playing, field, value);
        set_parameter_field(&mut states.idle, field, value);
    }

    pub fn insert_keyframe_at_cursor(&mut self, lane: LaneId) -> Option<usize> {
        let beat = self.clip_editor.clip_cursor_beat;
        let base = self.base_state();
        let clip = self.selected_clip()?.clone();
        if lane.is_color() {
            let value = match sampled_clip_lane_value(&clip, base, lane, beat) {
                LaneValue::Color(value) => value,
                LaneValue::Float(_) => [1.0, 1.0, 1.0],
            };
            let keyframes = color_lane_mut(&mut self.selected_clip_mut()?.lanes, lane);
            if let Some(index) = keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
            {
                return Some(index);
            }
            keyframes.push(ColorKeyframe {
                beat,
                value,
                interpolation: InterpolationMode::Hold,
            });
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
        } else {
            let value = match sampled_clip_lane_value(&clip, base, lane, beat) {
                LaneValue::Float(value) => value,
                LaneValue::Color(_) => 0.0,
            };
            let keyframes = float_lane_mut(&mut self.selected_clip_mut()?.lanes, lane);
            if let Some(index) = keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
            {
                return Some(index);
            }
            keyframes.push(FloatKeyframe {
                beat,
                value,
                interpolation: InterpolationMode::Hold,
            });
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
        }
    }

    pub fn clone_keyframe_to_cursor(&mut self, lane: LaneId, index: usize) -> Option<usize> {
        let beat = self.clip_editor.clip_cursor_beat;
        if lane.is_color() {
            let keyframes = color_lane_mut(&mut self.selected_clip_mut()?.lanes, lane);
            if let Some(existing) = keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
            {
                return Some(existing);
            }
            let source = keyframes.get(index)?.clone();
            keyframes.push(ColorKeyframe {
                beat,
                value: source.value,
                interpolation: source.interpolation,
            });
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
        } else {
            let keyframes = float_lane_mut(&mut self.selected_clip_mut()?.lanes, lane);
            if let Some(existing) = keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
            {
                return Some(existing);
            }
            let source = keyframes.get(index)?.clone();
            keyframes.push(FloatKeyframe {
                beat,
                value: source.value,
                interpolation: source.interpolation,
            });
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001)
        }
    }

    pub fn move_selected_keyframe_to_cursor(
        &mut self,
        lane: LaneId,
        index: usize,
    ) -> Result<Option<usize>, String> {
        self.move_keyframe_to_beat(lane, index, self.clip_editor.clip_cursor_beat)
    }

    pub fn move_keyframe_to_beat(
        &mut self,
        lane: LaneId,
        index: usize,
        beat: f32,
    ) -> Result<Option<usize>, String> {
        let Some(clip_length) = self.selected_clip().map(|clip| clip.length_beats) else {
            return Ok(None);
        };
        let beat = beat.clamp(0.0, clip_length);
        if lane.is_color() {
            let keyframes = color_lane_mut(&mut self.selected_clip_mut().unwrap().lanes, lane);
            if keyframes.iter().enumerate().any(|(candidate, keyframe)| {
                candidate != index && (keyframe.beat - beat).abs() < 0.0001
            }) {
                return Err(format!("duplicate keyframe at beat {:.2}", beat));
            }
            let Some(keyframe) = keyframes.get_mut(index) else {
                return Ok(None);
            };
            keyframe.beat = beat;
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            Ok(keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001))
        } else {
            let keyframes = float_lane_mut(&mut self.selected_clip_mut().unwrap().lanes, lane);
            if keyframes.iter().enumerate().any(|(candidate, keyframe)| {
                candidate != index && (keyframe.beat - beat).abs() < 0.0001
            }) {
                return Err(format!("duplicate keyframe at beat {:.2}", beat));
            }
            let Some(keyframe) = keyframes.get_mut(index) else {
                return Ok(None);
            };
            keyframe.beat = beat;
            keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            Ok(keyframes
                .iter()
                .position(|keyframe| (keyframe.beat - beat).abs() < 0.0001))
        }
    }

    pub fn delete_keyframe(&mut self, lane: LaneId, index: usize) {
        if let Some(clip) = self.selected_clip_mut() {
            if lane.is_color() {
                let keyframes = color_lane_mut(&mut clip.lanes, lane);
                if index < keyframes.len() {
                    keyframes.remove(index);
                }
            } else {
                let keyframes = float_lane_mut(&mut clip.lanes, lane);
                if index < keyframes.len() {
                    keyframes.remove(index);
                }
            }
        }
    }

    pub fn cycle_interpolation(&mut self, lane: LaneId, index: usize) {
        if let Some(clip) = self.selected_clip_mut() {
            if lane.is_color() {
                if let Some(keyframe) = color_lane_mut(&mut clip.lanes, lane).get_mut(index) {
                    keyframe.interpolation = keyframe.interpolation.cycle();
                }
            } else if let Some(keyframe) = float_lane_mut(&mut clip.lanes, lane).get_mut(index) {
                keyframe.interpolation = keyframe.interpolation.cycle();
            }
        }
    }

    pub fn set_keyframe_value(&mut self, lane: LaneId, index: usize, value: f32) {
        if let Some(clip) = self.selected_clip_mut() {
            if let Some(keyframe) = float_lane_mut(&mut clip.lanes, lane).get_mut(index) {
                keyframe.value = value;
            }
        }
    }

    pub fn set_keyframe_color(&mut self, lane: LaneId, index: usize, channel: usize, value: f32) {
        if let Some(clip) = self.selected_clip_mut() {
            if let Some(keyframe) = color_lane_mut(&mut clip.lanes, lane).get_mut(index) {
                if channel < 3 {
                    keyframe.value[channel] = value;
                }
            }
        }
    }

    pub fn set_selected_clip_length(&mut self, value: f32) -> Result<(), String> {
        let value = value.max(0.25);
        let Some(clip_id) = self.clip_editor.selected_clip_id.0.clone() else {
            return Err("no selected clip".to_string());
        };
        let Some(index) = self
            .timeline()
            .clips
            .iter()
            .position(|clip| clip.id == clip_id)
        else {
            return Err("no selected clip".to_string());
        };
        let old = self.timeline().clips[index].length_beats;
        self.timeline_mut().clips[index].length_beats = value;
        if let Some(error) = self.validate_current_timeline() {
            self.timeline_mut().clips[index].length_beats = old;
            return Err(error);
        }
        Ok(())
    }

    fn placement_index_at_beat(&self, beat: f32) -> Option<usize> {
        self.timeline()
            .arrangement
            .iter()
            .enumerate()
            .find_map(|(index, placement)| {
                let clip = self.timeline().clip_by_id(&placement.clip_id)?;
                let end = placement.end_beat(clip);
                (beat >= placement.start_beat && beat < end).then_some(index)
            })
    }

    fn validate_placement(
        &self,
        exclude_index: Option<usize>,
        placement: &ClipPlacement,
    ) -> Result<(), String> {
        let Some(clip) = self.timeline().clip_by_id(&placement.clip_id) else {
            return Err("placement references missing clip".to_string());
        };
        let end = placement.end_beat(clip);
        if end > self.timeline().total_beats() + 0.0001 {
            return Err("placement extends beyond total timeline beats".to_string());
        }
        for (index, existing) in self.timeline().arrangement.iter().enumerate() {
            if Some(index) == exclude_index {
                continue;
            }
            let Some(existing_clip) = self.timeline().clip_by_id(&existing.clip_id) else {
                continue;
            };
            let existing_end = existing.end_beat(existing_clip);
            if placement.start_beat < existing_end - 0.0001 && existing.start_beat < end - 0.0001 {
                return Err("placement would overlap existing arrangement".to_string());
            }
        }
        Ok(())
    }

    fn validate_current_timeline(&self) -> Option<String> {
        let timeline = self.timeline();
        for placement in &timeline.arrangement {
            let clip = timeline.clip_by_id(&placement.clip_id)?;
            let end = placement.end_beat(clip);
            if end > timeline.total_beats() + 0.0001 {
                return Some(
                    "clip length change would extend placement past timeline end".to_string(),
                );
            }
        }
        let mut spans = timeline
            .arrangement
            .iter()
            .filter_map(|placement| {
                let clip = timeline.clip_by_id(&placement.clip_id)?;
                Some((placement.start_beat, placement.end_beat(clip)))
            })
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        for window in spans.windows(2) {
            if window[0].1 > window[1].0 + 0.0001 {
                return Some("clip length change would create overlapping placements".to_string());
            }
        }
        None
    }

    fn next_clip_id(&self, seed: &str) -> String {
        let base = sanitize_clip_id(seed);
        let mut counter = 1usize;
        loop {
            let candidate = format!("{base}_{counter}");
            if self.timeline().clip_by_id(&candidate).is_none() {
                return candidate;
            }
            counter += 1;
        }
    }
}

impl TrackVisualizerConfig {
    fn with_synced_base_states(mut self) -> Self {
        if let Some(states) = self.params.shader_states.as_mut() {
            states.idle = states.playing;
        }
        self
    }
}

impl EditorPreviewMode {
    fn enables_timeline_preview(self) -> bool {
        matches!(self, Self::TimelineWhilePaused)
    }
}

impl NumericEditTarget {
    fn label(&self) -> &'static str {
        match self {
            Self::StateField(field) => field.label(),
            Self::TimelineBpm => "bpm",
            Self::TimelineMeasures => "measures",
            Self::TimelineBeatsPerMeasure => "beats_per_measure",
            Self::ClipLength => "clip.length_beats",
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
            Self::BeatsPerMeasure => "beats_per_measure",
            Self::LengthBeats => "length_beats",
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
            (_, Self::Bpm) => Self::Measures,
            (_, Self::Measures) => Self::BeatsPerMeasure,
            (_, Self::BeatsPerMeasure) => Self::LengthBeats,
            (_, Self::LengthBeats) => Self::Beat,
            (false, Self::Beat) => Self::Value,
            (false, Self::Value) => Self::Interpolation,
            (false, _) => Self::Bpm,
            (true, Self::Beat) => Self::Interpolation,
            (true, Self::Interpolation) => Self::Red,
            (true, Self::Red) => Self::Green,
            (true, Self::Green) => Self::Blue,
            (true, _) => Self::Bpm,
        }
    }

    fn prev(self, lane: LaneId) -> Self {
        match (lane.is_color(), self) {
            (_, Self::Bpm) => {
                if lane.is_color() {
                    Self::Blue
                } else {
                    Self::Interpolation
                }
            }
            (_, Self::Measures) => Self::Bpm,
            (_, Self::BeatsPerMeasure) => Self::Measures,
            (_, Self::LengthBeats) => Self::BeatsPerMeasure,
            (_, Self::Beat) => Self::LengthBeats,
            (false, Self::Value) => Self::Beat,
            (false, Self::Interpolation) => Self::Value,
            (false, _) => Self::Interpolation,
            (true, Self::Interpolation) => Self::Beat,
            (true, Self::Red) => Self::Interpolation,
            (true, Self::Green) => Self::Red,
            (true, Self::Blue) => Self::Green,
            (true, Self::Value) => Self::Beat,
        }
    }
}

impl EditorFocusArea {
    fn label(self) -> &'static str {
        match self {
            Self::StateParams => "state",
            Self::ClipLibrary => "library",
            Self::Arrangement => "arrangement",
            Self::ClipLanes => "lanes",
            Self::ClipGraph => "graph",
            Self::ClipInspector => "inspector",
            Self::Export => "export",
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
        automation: None,
        timeline: Some(default_timeline()),
    }
}

fn default_timeline() -> ChromaticBulgeGridClipTimeline {
    ChromaticBulgeGridClipTimeline {
        bpm: 132.0,
        measures: 64,
        beats_per_measure: 4,
        clips: vec![ChromaticBulgeGridClip {
            id: "clip_1".to_string(),
            name: "Clip 1".to_string(),
            length_beats: 4.0,
            color: default_clip_color(1),
            lanes: ChromaticBulgeGridAutomationLanes::default(),
        }],
        arrangement: Vec::new(),
    }
}

fn default_clip_color(seed: usize) -> [f32; 3] {
    const PALETTE: [[f32; 3]; 6] = [
        [0.43, 0.86, 0.83],
        [0.90, 0.57, 0.34],
        [0.55, 0.73, 0.96],
        [0.86, 0.69, 0.35],
        [0.69, 0.85, 0.49],
        [0.91, 0.50, 0.59],
    ];
    PALETTE[seed % PALETTE.len()]
}

fn next_focus_area(tab: EditorTab, current: EditorFocusArea) -> EditorFocusArea {
    match tab {
        EditorTab::State => EditorFocusArea::StateParams,
        EditorTab::Clips => match current {
            EditorFocusArea::ClipLibrary => EditorFocusArea::Arrangement,
            EditorFocusArea::Arrangement => EditorFocusArea::ClipLanes,
            EditorFocusArea::ClipLanes => EditorFocusArea::ClipGraph,
            EditorFocusArea::ClipGraph => EditorFocusArea::ClipInspector,
            _ => EditorFocusArea::ClipLibrary,
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

fn parameter_value(state: ChromaticBulgeGridShaderState, field: ParameterField) -> f32 {
    match field {
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
    }
}

fn format_parameter_value(state: ChromaticBulgeGridShaderState, field: ParameterField) -> String {
    format!("{:.3}", parameter_value(state, field))
}

fn parameter_step(field: ParameterField, coarse: bool) -> f32 {
    let step = match field {
        ParameterField::SpacingMaxPx | ParameterField::SpacingMinPx => 2.0,
        ParameterField::ColdColorR
        | ParameterField::ColdColorG
        | ParameterField::ColdColorB
        | ParameterField::HotColorR
        | ParameterField::HotColorG
        | ParameterField::HotColorB => 0.02,
        ParameterField::LatticeDensity
        | ParameterField::ScrollBase
        | ParameterField::ScrollMotionScale
        | ParameterField::ScrollMotionFloor
        | ParameterField::ScrollMotionCeiling => 0.05,
        _ => 0.01,
    };
    if coarse {
        step * 5.0
    } else {
        step
    }
}

fn inspector_step(field: InspectorField, lane: LaneId, coarse: bool) -> f32 {
    let step = match field {
        InspectorField::Bpm => 0.5,
        InspectorField::Measures | InspectorField::BeatsPerMeasure => 1.0,
        InspectorField::LengthBeats | InspectorField::Beat => 0.25,
        InspectorField::Value => {
            if lane.is_color() {
                0.02
            } else {
                0.01
            }
        }
        InspectorField::Red | InspectorField::Green | InspectorField::Blue => 0.02,
        InspectorField::Interpolation => 0.0,
    };
    if coarse {
        step * 4.0
    } else {
        step
    }
}

fn current_global_beat(playback: PlaybackClock, timeline: &ChromaticBulgeGridClipTimeline) -> f32 {
    playback.current_time_secs.max(0.0) * timeline.bpm.max(1.0) / 60.0
}

fn arrangement_status_line(
    draft: &ChromaticBulgeGridEditorDraft,
    current_beat: f32,
) -> Line<'static> {
    let timeline = draft.timeline();
    let selection = draft
        .selected_placement()
        .and_then(|placement| {
            let clip = timeline.clip_by_id(&placement.clip_id)?;
            Some(format!(
                "{} {:.2}-{:.2}",
                clip.name,
                placement.start_beat,
                placement.end_beat(clip)
            ))
        })
        .unwrap_or_else(|| "none".to_string());
    Line::from(format!(
        "bpm {:.1}  measures {}  beats/bar {}  beat {:>5.2}  selected {}",
        timeline.bpm, timeline.measures, timeline.beats_per_measure, current_beat, selection
    ))
}

fn build_arrangement_grid(
    draft: &ChromaticBulgeGridEditorDraft,
    current_beat: f32,
    width: usize,
) -> Vec<Line<'static>> {
    let width = width.max(16);
    let timeline = draft.timeline();
    let total_beats = timeline.total_beats().max(1.0);
    let mut measure_row = vec![' '; width];
    let mut playhead_row = vec![' '; width];

    let to_index = |beat: f32| {
        (((beat / total_beats).clamp(0.0, 1.0)) * (width.saturating_sub(1) as f32)).round() as usize
    };

    for measure in 0..=timeline.measures {
        let beat = measure as f32 * timeline.beats_per_measure as f32;
        let index = to_index(beat.min(total_beats));
        measure_row[index] = '|';
    }
    playhead_row[to_index(current_beat.min(total_beats))] = '^';

    let selected_index = draft.clip_editor.selected_placement_index.0;
    let mut spans = Vec::<Span<'static>>::new();
    let mut cursor = 0usize;
    for (index, placement) in timeline.arrangement.iter().enumerate() {
        let Some(clip) = timeline.clip_by_id(&placement.clip_id) else {
            continue;
        };
        let start = to_index(placement.start_beat);
        let end = to_index(placement.end_beat(clip));
        if start > cursor {
            spans.push(Span::raw(" ".repeat(start - cursor)));
        }
        let cells = end.saturating_sub(start).max(1);
        let label = if cells >= clip.name.len() + 2 {
            clip.name.clone()
        } else {
            clip_initials(&clip.name)
        };
        spans.push(Span::styled(
            fill_label(&label, cells),
            placement_style(clip.color, Some(index) == selected_index),
        ));
        cursor = start + cells;
    }
    if cursor < width {
        spans.push(Span::raw(" ".repeat(width - cursor)));
    }

    vec![
        Line::from(format!(
            "0{:>width$}",
            format!("{:.0} beats", total_beats),
            width = width - 1
        )),
        Line::from(measure_row.into_iter().collect::<String>()),
        Line::from(playhead_row.into_iter().collect::<String>()),
        Line::from(spans),
    ]
}

fn build_clip_preview(clip: &ChromaticBulgeGridClip) -> String {
    if let Some(lane) = headline_lane(clip) {
        if lane.is_color() {
            let color = match sampled_clip_lane_value(
                clip,
                ChromaticBulgeGridShaderState::default(),
                lane,
                0.0,
            ) {
                LaneValue::Color(value) => value,
                LaneValue::Float(_) => [1.0, 1.0, 1.0],
            };
            return format!("RGB {:.2}/{:.2}/{:.2}", color[0], color[1], color[2]);
        }
        let width = 16usize;
        let mut output = String::with_capacity(width);
        for index in 0..width {
            let beat = if width <= 1 {
                0.0
            } else {
                clip.length_beats * index as f32 / (width - 1) as f32
            };
            let value = match sampled_clip_lane_value(
                clip,
                ChromaticBulgeGridShaderState::default(),
                lane,
                beat,
            ) {
                LaneValue::Float(value) => value,
                LaneValue::Color(_) => 0.0,
            };
            output.push(spark_char((value / 2.0).clamp(0.0, 1.0)));
        }
        output
    } else {
        "................".to_string()
    }
}

fn build_local_clip_graph(
    draft: &ChromaticBulgeGridEditorDraft,
    lane: LaneId,
    clip_length: f32,
    cursor: f32,
    ghost: Option<f32>,
    width: usize,
    selected_keyframe_index: usize,
) -> Vec<Line<'static>> {
    let width = width.max(16);
    let to_index = |beat: f32| {
        (((beat / clip_length.max(0.0001)).clamp(0.0, 1.0)) * (width.saturating_sub(1) as f32))
            .round() as usize
    };
    let mut top = vec![' '; width];
    let mut mid = vec![' '; width];
    let mut bottom = vec![' '; width];

    let quarters = (clip_length * 4.0).ceil() as usize;
    for quarter in 0..=quarters {
        let beat = quarter as f32 / 4.0;
        let column = to_index(beat.min(clip_length));
        top[column] = if quarter % 4 == 0 { '|' } else { ':' };
        bottom[column] = if quarter % 4 == 0 { '|' } else { ':' };
    }

    if lane.is_color() {
        for (index, keyframe) in draft.color_keyframes(lane).iter().enumerate() {
            let column = to_index(keyframe.beat.min(clip_length));
            mid[column] = if index == selected_keyframe_index {
                '*'
            } else {
                'o'
            };
        }
    } else {
        for (index, keyframe) in draft.float_keyframes(lane).iter().enumerate() {
            let column = to_index(keyframe.beat.min(clip_length));
            mid[column] = if index == selected_keyframe_index {
                '*'
            } else {
                'o'
            };
        }
    }

    let cursor_index = to_index(cursor);
    top[cursor_index] = '^';
    bottom[cursor_index] = 'v';
    if let Some(ghost) = ghost {
        let ghost_index = to_index(ghost);
        if ghost_index != cursor_index {
            top[ghost_index] = '\'';
            bottom[ghost_index] = '.';
        }
    }

    vec![
        Line::from(format!(
            "0{:>width$}",
            format!("{:.2} beats", clip_length),
            width = width - 1
        )),
        Line::from(top.into_iter().collect::<String>()),
        Line::from(mid.into_iter().collect::<String>()),
        Line::from(bottom.into_iter().collect::<String>()),
    ]
}

fn render_selected_lane_keyframes(
    draft: &ChromaticBulgeGridEditorDraft,
    lane: LaneId,
    keyframe_index: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if lane.is_color() {
        for (index, keyframe) in draft.color_keyframes(lane).iter().enumerate() {
            let marker = if index == keyframe_index { ">" } else { " " };
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
            let marker = if index == keyframe_index { ">" } else { " " };
            lines.push(Line::from(format!(
                "{} beat {:>5.2}  value {:>6.3}  {:?}",
                marker, keyframe.beat, keyframe.value, keyframe.interpolation
            )));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("no keyframes on selected lane"));
    }
    lines
}

#[derive(Clone, Copy)]
enum LaneValue {
    Float(f32),
    Color([f32; 3]),
}

fn sampled_clip_lane_value(
    clip: &ChromaticBulgeGridClip,
    base: ChromaticBulgeGridShaderState,
    lane: LaneId,
    beat: f32,
) -> LaneValue {
    let state = clip.apply_to_state(base, beat);
    base_lane_value(state, lane)
}

fn base_lane_value(uniforms: ChromaticBulgeGridShaderState, lane: LaneId) -> LaneValue {
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

fn format_lane_value(value: LaneValue) -> String {
    match value {
        LaneValue::Float(value) => format!("{value:.3}"),
        LaneValue::Color(value) => format!("[{:.2}, {:.2}, {:.2}]", value[0], value[1], value[2]),
    }
}

fn headline_lane(clip: &ChromaticBulgeGridClip) -> Option<LaneId> {
    for lane in [LaneId::CircleRadius, LaneId::ScrollBase, LaneId::MotionRate] {
        if clip_lane_has_keys(clip, lane) {
            return Some(lane);
        }
    }
    LaneId::ALL
        .iter()
        .copied()
        .find(|lane| clip_lane_has_keys(clip, *lane))
}

fn clip_lane_has_keys(clip: &ChromaticBulgeGridClip, lane: LaneId) -> bool {
    if lane.is_color() {
        !color_lane(&clip.lanes, lane).is_empty()
    } else {
        !float_lane(&clip.lanes, lane).is_empty()
    }
}

fn float_lane(lanes: &ChromaticBulgeGridAutomationLanes, lane: LaneId) -> &[FloatKeyframe] {
    match lane {
        LaneId::MotionRate => &lanes.motion_rate,
        LaneId::LatticeDensity => &lanes.lattice_density,
        LaneId::CircleRadius => &lanes.circle_radius,
        LaneId::CircleFalloffStart => &lanes.circle_falloff_start,
        LaneId::CircleFalloffEnd => &lanes.circle_falloff_end,
        LaneId::BulgeAmount => &lanes.bulge_amount,
        LaneId::RimGuard => &lanes.rim_guard,
        LaneId::RimExponent => &lanes.rim_exponent,
        LaneId::RimWarp => &lanes.rim_warp,
        LaneId::SpacingMaxPx => &lanes.spacing_max_px,
        LaneId::SpacingMinPx => &lanes.spacing_min_px,
        LaneId::DotSize => &lanes.dot_size,
        LaneId::OuterDotScale => &lanes.outer_dot_scale,
        LaneId::EdgeSoftness => &lanes.edge_softness,
        LaneId::ChromaticAberration => &lanes.chromatic_aberration,
        LaneId::ScrollBase => &lanes.scroll_base,
        LaneId::ScrollMotionScale => &lanes.scroll_motion_scale,
        LaneId::ScrollMotionFloor => &lanes.scroll_motion_floor,
        LaneId::ScrollMotionCeiling => &lanes.scroll_motion_ceiling,
        LaneId::ColorCycleRate => &lanes.color_cycle_rate,
        LaneId::InnerAlpha => &lanes.inner_alpha,
        LaneId::ColdColor | LaneId::HotColor => &[],
    }
}

fn color_lane(lanes: &ChromaticBulgeGridAutomationLanes, lane: LaneId) -> &[ColorKeyframe] {
    match lane {
        LaneId::ColdColor => &lanes.cold_color,
        LaneId::HotColor => &lanes.hot_color,
        _ => &[],
    }
}

fn float_lane_mut(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    lane: LaneId,
) -> &mut Vec<FloatKeyframe> {
    match lane {
        LaneId::MotionRate => &mut lanes.motion_rate,
        LaneId::LatticeDensity => &mut lanes.lattice_density,
        LaneId::CircleRadius => &mut lanes.circle_radius,
        LaneId::CircleFalloffStart => &mut lanes.circle_falloff_start,
        LaneId::CircleFalloffEnd => &mut lanes.circle_falloff_end,
        LaneId::BulgeAmount => &mut lanes.bulge_amount,
        LaneId::RimGuard => &mut lanes.rim_guard,
        LaneId::RimExponent => &mut lanes.rim_exponent,
        LaneId::RimWarp => &mut lanes.rim_warp,
        LaneId::SpacingMaxPx => &mut lanes.spacing_max_px,
        LaneId::SpacingMinPx => &mut lanes.spacing_min_px,
        LaneId::DotSize => &mut lanes.dot_size,
        LaneId::OuterDotScale => &mut lanes.outer_dot_scale,
        LaneId::EdgeSoftness => &mut lanes.edge_softness,
        LaneId::ChromaticAberration => &mut lanes.chromatic_aberration,
        LaneId::ScrollBase => &mut lanes.scroll_base,
        LaneId::ScrollMotionScale => &mut lanes.scroll_motion_scale,
        LaneId::ScrollMotionFloor => &mut lanes.scroll_motion_floor,
        LaneId::ScrollMotionCeiling => &mut lanes.scroll_motion_ceiling,
        LaneId::ColorCycleRate => &mut lanes.color_cycle_rate,
        LaneId::InnerAlpha => &mut lanes.inner_alpha,
        LaneId::ColdColor | LaneId::HotColor => unreachable!("color lane requested as float"),
    }
}

fn color_lane_mut(
    lanes: &mut ChromaticBulgeGridAutomationLanes,
    lane: LaneId,
) -> &mut Vec<ColorKeyframe> {
    match lane {
        LaneId::ColdColor => &mut lanes.cold_color,
        LaneId::HotColor => &mut lanes.hot_color,
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
            format!("{:<10}", label),
            Style::default().fg(if selected { WHITE } else { DIM }),
        ),
        Span::styled(value, Style::default().fg(CYAN)),
    ])
}

fn format_color_tag(color: [f32; 3]) -> String {
    format!("[{:.2},{:.2},{:.2}]", color[0], color[1], color[2])
}

fn placement_style(color: [f32; 3], selected: bool) -> Style {
    let fg = Color::Rgb(
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
    );
    let mut style = Style::default().fg(BG).bg(fg);
    if selected {
        style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
    }
    style
}

fn fill_label(label: &str, width: usize) -> String {
    let mut output = String::with_capacity(width);
    let bytes = if label.is_empty() {
        b"?"
    } else {
        label.as_bytes()
    };
    for index in 0..width {
        output.push(bytes[index % bytes.len()] as char);
    }
    output
}

fn clip_initials(name: &str) -> String {
    let initials = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(3)
        .collect::<String>();
    if initials.is_empty() {
        "CLP".to_string()
    } else {
        initials.to_ascii_uppercase()
    }
}

fn sanitize_clip_id(seed: &str) -> String {
    let mut output = seed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while output.contains("__") {
        output = output.replace("__", "_");
    }
    output.trim_matches('_').to_string()
}

fn spark_char(value: f32) -> char {
    const CHARS: &[u8] = b" .:-=+*#%@";
    let index = (value.clamp(0.0, 1.0) * (CHARS.len().saturating_sub(1) as f32)).round() as usize;
    CHARS[index] as char
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
    migrated_legacy: bool,
    status: Rc<RefCell<Option<String>>>,
    save_feedback: Rc<RefCell<Option<SaveFeedback>>>,
) {
    let payload = serde_json::json!({
        "record_id": record_id,
        "visualizer": visualizer.clone(),
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
                    *save_feedback.borrow_mut() = Some(SaveFeedback::Saved {
                        record_id: record_id.clone(),
                        visualizer,
                        migrated_legacy,
                    });
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

#[cfg(test)]
mod tests {
    use super::{
        default_timeline, default_visualizer_config, ChromaticBulgeGridEditorDraft,
        ClipEditorDraft, EditorPreviewMode, LaneId, ParameterField, SelectedClipId,
        SelectedPlacementIndex,
    };
    use crate::archive::{
        legacy_automation_to_timeline, ChromaticBulgeGridAutomation,
        ChromaticBulgeGridAutomationLanes, FloatKeyframe, InterpolationMode, TrackVisualizerConfig,
    };

    #[test]
    fn mark_saved_resets_dirty_state_against_saved_payload() {
        let original = default_visualizer_config();
        let mut draft = seeded_draft();
        draft.set_base_field(ParameterField::MotionRate, 2.5);
        draft.insert_keyframe_at_cursor(LaneId::MotionRate);
        assert!(draft.is_dirty());

        draft.mark_saved(draft.working.normalized_for_export());
        assert!(!draft.is_dirty());
        assert_eq!(draft.original.mode, original.mode);
    }

    #[test]
    fn base_field_edits_keep_playing_and_idle_in_sync() {
        let mut draft = seeded_draft();
        draft.set_base_field(ParameterField::ScrollBase, 1.75);
        let states = draft.working.params.shader_states.expect("shader states");
        assert_eq!(states.playing.scroll_base, 1.75);
        assert_eq!(states.idle.scroll_base, 1.75);
    }

    #[test]
    fn legacy_automation_open_maps_to_imported_clip_and_placement() {
        let automation = ChromaticBulgeGridAutomation {
            bpm: 120.0,
            measures: 8,
            beats_per_measure: 4,
            lanes: ChromaticBulgeGridAutomationLanes {
                circle_radius: vec![FloatKeyframe {
                    beat: 0.0,
                    value: 0.3,
                    interpolation: InterpolationMode::Hold,
                }],
                ..Default::default()
            },
        };
        let original = TrackVisualizerConfig {
            automation: Some(automation.clone()),
            ..default_visualizer_config()
        };
        let draft = ChromaticBulgeGridEditorDraft {
            record_id: "record".to_string(),
            original,
            working: TrackVisualizerConfig {
                timeline: Some(legacy_automation_to_timeline(&automation)),
                automation: None,
                ..default_visualizer_config()
            },
            clip_editor: ClipEditorDraft {
                selected_clip_id: SelectedClipId(Some("imported_timeline".to_string())),
                selected_placement_index: SelectedPlacementIndex(Some(0)),
                clip_cursor_beat: 0.0,
                preview_mode: EditorPreviewMode::TimelineWhilePaused,
            },
            opened_from_legacy: true,
        };

        assert_eq!(draft.timeline().clips.len(), 1);
        assert_eq!(draft.timeline().arrangement.len(), 1);
        assert!(draft.working.automation.is_none());
    }

    #[test]
    fn adding_placement_uses_current_playhead_beat() {
        let mut draft = seeded_draft();
        draft.add_placement_at_playhead(12.0).expect("placement");
        assert_eq!(draft.timeline().arrangement[0].start_beat, 12.0);
    }

    #[test]
    fn increasing_repeats_blocks_overlap_if_invalid() {
        let mut draft = seeded_draft();
        draft.timeline_mut().arrangement = vec![
            super::ClipPlacement {
                clip_id: "clip_1".to_string(),
                start_beat: 0.0,
                repeats: 1,
            },
            super::ClipPlacement {
                clip_id: "clip_1".to_string(),
                start_beat: 4.0,
                repeats: 1,
            },
        ];
        draft.clip_editor.selected_placement_index = SelectedPlacementIndex(Some(0));

        let error = draft
            .adjust_selected_placement_repeats(1)
            .expect_err("overlap should fail");
        assert!(error.contains("overlap"));
    }

    #[test]
    fn deleting_in_use_clip_is_rejected() {
        let mut draft = seeded_draft();
        draft.timeline_mut().arrangement.push(super::ClipPlacement {
            clip_id: "clip_1".to_string(),
            start_beat: 0.0,
            repeats: 1,
        });
        let error = draft
            .delete_selected_clip()
            .expect_err("in-use clip should fail");
        assert!(error.contains("placements"));
    }

    #[test]
    fn clip_rename_preserves_clip_id() {
        let mut draft = seeded_draft();
        let clip_id = draft.selected_clip().unwrap().id.clone();
        draft.rename_clip(&clip_id, "Renamed");
        assert_eq!(draft.selected_clip().unwrap().id, clip_id);
        assert_eq!(draft.selected_clip().unwrap().name, "Renamed");
    }

    #[test]
    fn export_json_is_canonical_timeline_without_automation() {
        let draft = seeded_draft();
        let json = draft.export_json().expect("json");
        assert!(json.contains("\"timeline\""));
        assert!(!json.contains("\"automation\""));
    }

    fn seeded_draft() -> ChromaticBulgeGridEditorDraft {
        let original = default_visualizer_config();
        ChromaticBulgeGridEditorDraft {
            record_id: "record".to_string(),
            original: original.clone(),
            working: TrackVisualizerConfig {
                timeline: Some(default_timeline()),
                automation: None,
                ..original
            },
            clip_editor: ClipEditorDraft {
                selected_clip_id: SelectedClipId(Some("clip_1".to_string())),
                selected_placement_index: SelectedPlacementIndex(None),
                clip_cursor_beat: 0.0,
                preview_mode: EditorPreviewMode::TimelineWhilePaused,
            },
            opened_from_legacy: false,
        }
    }
}
