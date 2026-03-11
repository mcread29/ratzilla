use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    archive::{
        legacy_automation_to_timeline, ChromaticBulgeGridAutomationLanes,
        ChromaticBulgeGridClip, ChromaticBulgeGridClipAuthoring, ChromaticBulgeGridClipTimeline,
        ChromaticBulgeGridLaneId, ChromaticBulgeGridShaderState,
        ChromaticBulgeGridShaderStates, ClipParamTrack, ClipPlacement, ClipTweenEase,
        ClipTweenStep, ClipTweenValue, PlaybackClock, RecordDocument, TrackVisualizerConfig,
        TrackVisualizerMode,
    },
    session::SessionModel,
    visualizer_sequence::compile_authoring_lanes,
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
    tab: EditorTab,
    focus_area: EditorFocusArea,
    state_field_index: usize,
    lane_index: usize,
    step_index: usize,
    modal: Option<EditorModal>,
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
pub enum EditorTab {
    State,
    Song,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorFocusArea {
    StateParams,
    ClipLibrary,
    Arrangement,
    ClipTracks,
    Export,
}

#[derive(Clone)]
enum EditorModal {
    Help,
    NewClip(NewClipDialog),
    Placement(PlacementDialog),
    StepEditor(StepEditorDialog),
}

#[derive(Clone)]
struct NewClipDialog {
    field_index: usize,
    name: String,
    length_beats: String,
}

#[derive(Clone)]
struct PlacementDialog {
    field_index: usize,
    start_beat: String,
    repeats: String,
    editing_existing: bool,
}

#[derive(Clone)]
struct StepEditorDialog {
    lane: ChromaticBulgeGridLaneId,
    step_index: Option<usize>,
    field_index: usize,
    value_kind: StepValueEditor,
    duration_beats: String,
    ease: ClipTweenEase,
}

#[derive(Clone)]
enum StepValueEditor {
    Float { value: String },
    Color {
        red: String,
        green: String,
        blue: String,
    },
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
    ClipLength,
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

impl VisualizerEditorOverlay {
    pub fn new() -> Self {
        Self {
            enabled: editor_flag_enabled(),
            is_open: false,
            drafts: EditorDraftStore {
                drafts: HashMap::new(),
            },
            tab: EditorTab::Song,
            focus_area: EditorFocusArea::ClipLibrary,
            state_field_index: 0,
            lane_index: 0,
            step_index: 0,
            modal: None,
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
            self.modal = None;
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
        self.tab = EditorTab::Song;
        self.focus_area = EditorFocusArea::ClipLibrary;
        self.modal = None;
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
        if self.modal.is_some() {
            return self.handle_modal_key(key, &record_id);
        }
        if self.text_input.is_some() {
            return self.handle_text_key(key, &record_id);
        }
        if self.numeric_input.is_some() {
            return self.handle_numeric_key(key, &record_id);
        }

        match key {
            KeyCode::Esc => {
                self.is_open = false;
                true
            }
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
                self.modal = Some(EditorModal::Help);
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
                self.tab = EditorTab::Song;
                self.focus_area = EditorFocusArea::ClipLibrary;
                true
            }
            KeyCode::Char('3') => {
                self.tab = EditorTab::Export;
                self.focus_area = EditorFocusArea::Export;
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
                if self.tab == EditorTab::Song
                    && self.focus_area == EditorFocusArea::Arrangement =>
            {
                self.scrub_beats(session, -0.25);
                true
            }
            KeyCode::Right
                if self.tab == EditorTab::Song
                    && self.focus_area == EditorFocusArea::Arrangement =>
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
            .title(" tty0 tween clip authoring ")
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
            EditorTab::Song => self.render_song_tab(frame, layout[2], draft, session),
            EditorTab::Export => self.render_export_tab(frame, layout[2], draft),
        }
        self.render_footer(frame, layout[3], draft);
        self.render_modal(frame, inner);
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
                    "time {:>5.1}s beat {:>5.2} bpm {:.1} bars {}",
                    playback.current_time_secs, beat, timeline.bpm, timeline.measures
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
            ("2 Song", EditorTab::Song),
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
                let selected = index == self.state_field_index;
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<22}", field.label()),
                        Style::default().fg(if selected { WHITE } else { TEXT }),
                    ),
                    Span::styled(
                        format_parameter_value(state, *field),
                        Style::default().fg(CYAN),
                    ),
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
                        .border_style(Style::default().fg(if self.focus_area
                            == EditorFocusArea::StateParams
                        {
                            CYAN
                        } else {
                            BORDER
                        })),
                )
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(Style::default().bg(Color::Rgb(18, 35, 33))),
            area,
            &mut state,
        );
    }

    fn render_song_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
        session: &SessionModel,
    ) {
        let root = Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)]).split(area);
        self.render_clip_library(frame, root[0], draft);

        let right = Layout::vertical([
            Constraint::Length(16),
            Constraint::Fill(1),
            Constraint::Length(6),
        ])
        .split(root[1]);
        self.render_arrangement(frame, right[0], draft, session);
        self.render_clip_tracks(frame, right[1], draft);
        self.render_song_help(frame, right[2]);
    }

    fn render_song_help(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from("Workflow"),
            Line::from("1. Pick a parameter lane in the left sidebar."),
            Line::from("2. `N` creates a clip for just that parameter."),
            Line::from("3. `C` adds a tween step to the selected clip."),
            Line::from("4. `P` places that clip onto the song timeline."),
            Line::from("5. Work lane by lane through the song."),
            Line::from(""),
            Line::from("Need the full command list: press `H`."),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(Block::bordered().title(" quick help ").border_style(
                    Style::default().fg(BORDER),
                )),
            area,
        );
    }

    fn render_clip_library(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let selected_lane = self.selected_lane();
        let items = ChromaticBulgeGridLaneId::ALL
            .iter()
            .map(|lane| {
                let selected = *lane == selected_lane;
                let header_style = if selected {
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(WHITE)
                };
                let clips = draft.clips_for_lane(*lane);
                let selected_clip_name = draft
                    .selected_clip_for_lane(*lane)
                    .map(|clip| clip.name.as_str())
                    .unwrap_or("none");
                ListItem::new(vec![
                    Line::from(vec![Span::styled(
                        lane.label(),
                        header_style,
                    )]),
                    Line::from(format!(
                        "{} clips  {} placements",
                        clips.len(),
                        draft.placement_count_for_lane(*lane)
                    )),
                    Line::from(format!("selected clip  {}", selected_clip_name)),
                ])
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(self.lane_index));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .title(" parameter lanes ")
                        .border_style(Style::default().fg(if self.focus_area
                            == EditorFocusArea::ClipLibrary
                        {
                            CYAN
                        } else {
                            BORDER
                        })),
                )
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(18, 35, 33))
                        .add_modifier(Modifier::BOLD),
                ),
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
        let beat = current_global_beat(session.playback_clock(), timeline);
        let mut lines = vec![Line::from(format!(
            "beat {:>5.2}  bpm {:.1}  bars {}  selected {}",
            beat,
            timeline.bpm,
            timeline.measures,
            draft
                .selected_placement()
                .and_then(|placement| {
                    let clip = timeline.clip_by_id(&placement.clip_id)?;
                    Some(format!(
                        "{} {} {:.2}-{:.2} x{}",
                        lane_label_for_track(placement.track),
                        clip.name,
                        placement.start_beat,
                        placement.end_beat(clip),
                        placement.repeats
                    ))
                })
                .unwrap_or_else(|| "none".to_string())
        ))];

        if timeline.arrangement.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("no placements"));
            lines.push(Line::from("P add placement at playhead"));
        } else {
            lines.extend(build_arrangement_grid(
                draft,
                beat,
                self.selected_lane(),
                area.height.saturating_sub(5) as usize,
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
                        .border_style(Style::default().fg(if self.focus_area
                            == EditorFocusArea::Arrangement
                        {
                            CYAN
                        } else {
                            BORDER
                        })),
                ),
            area,
        );
    }

    fn render_clip_tracks(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let outer = Block::bordered()
            .title(" lane clip editor ")
            .border_style(Style::default().fg(if self.focus_area == EditorFocusArea::ClipTracks {
                CYAN
            } else {
                BORDER
            }));
        let inner = outer.inner(area);
        frame.render_widget(
            outer.style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT)),
            area,
        );

        let lane = self.selected_lane();
        let lane_clips = draft.clips_for_lane(lane);
        let Some(clip) = draft.selected_clip() else {
            frame.render_widget(
                Paragraph::new(format!(
                    "{} has no clips yet\n\nPress `N` to create one.",
                    lane.label()
                )),
                inner,
            );
            return;
        };

        let split = Layout::horizontal([Constraint::Length(26), Constraint::Fill(1)]).split(inner);
        let clip_items = lane_clips
            .iter()
            .map(|candidate| {
                ListItem::new(vec![
                    Line::from(vec![Span::styled(
                        format!("{} {}", format_color_tag(candidate.color), candidate.name),
                        Style::default().fg(if candidate.id == clip.id { CYAN } else { WHITE }),
                    )]),
                    Line::from(format!(
                        "{:.2} beats  {} placements",
                        candidate.length_beats,
                        draft.placement_count_for_clip(&candidate.id)
                    )),
                    Line::from(build_clip_preview(candidate)),
                ])
            })
            .collect::<Vec<_>>();
        let mut lane_state = ListState::default();
        lane_state.select(draft.selected_clip_index_for_lane(lane));
        frame.render_stateful_widget(
            List::new(clip_items)
                .block(Block::bordered().title(" clips on lane "))
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .highlight_style(Style::default().bg(Color::Rgb(18, 35, 33))),
            split[0],
            &mut lane_state,
        );

        let mut lines = vec![
            Line::from(format!(
                "{}  {}  len {:.2} beats  {} placements",
                lane.label(),
                clip.name,
                clip.length_beats,
                draft.placement_count_for_clip(&clip.id)
            )),
            Line::from(
                "N new clip  D duplicate  R rename  C add step  E edit  Y duplicate  U/J reorder  Delete remove  L clip length",
            ),
            Line::from(""),
        ];

        let Some(track) = draft.track_for_selected_clip() else {
            lines.push(Line::from("no authoring track on selected clip"));
            frame.render_widget(
                Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                    .block(Block::bordered().title(" steps ")),
                split[1],
            );
            return;
        };
        if track.steps.is_empty() {
            lines.push(Line::from("no tween steps"));
        } else {
            for (index, step) in track.steps.iter().enumerate() {
                let selected = index == self.step_index;
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {} {:>2}. to {:<18} dur {:>5.3}  {}",
                        if selected { "*" } else { " " },
                        index + 1,
                        format_tween_value(&step.to),
                        step.duration_beats,
                        step.ease.label()
                    ),
                    Style::default().fg(if selected { CYAN } else { TEXT }),
                )]));
            }
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(Block::bordered().title(" steps ")),
            split[1],
        );
    }

    fn render_export_tab(
        &self,
        frame: &mut Frame,
        area: Rect,
        draft: &ChromaticBulgeGridEditorDraft,
    ) {
        let export = draft.export_json().unwrap_or_else(|error| error.to_string());
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
            format!("editing text = {}", input.buffer)
        } else if let Some(input) = &self.numeric_input {
            format!("editing {} = {}", input.target.label(), input.buffer)
        } else {
            self.status_message
                .borrow()
                .clone()
                .unwrap_or_else(|| self.default_status_message())
        };
        let lines = vec![Line::from(vec![
            Span::styled(status, Style::default().fg(AMBER)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled(
                format!("dirty={} focus={}", draft.is_dirty(), self.focus_area.label()),
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

    fn default_status_message(&self) -> String {
        match self.focus_area {
            EditorFocusArea::StateParams => "Enter edit value  +/- nudge  H help".to_string(),
            EditorFocusArea::ClipLibrary => {
                "Up/Down lane  N new clip  D duplicate  R rename  Delete remove  Enter edit  H help"
                    .to_string()
            }
            EditorFocusArea::Arrangement => {
                "Left/Right scrub  P place clip  Enter edit placement  +/- repeats  Delete remove  H help"
                    .to_string()
            }
            EditorFocusArea::ClipTracks => {
                "Up/Down clip  Left/Right step  N new  D dup clip  R rename  C add  E edit  Y dup step  U/J reorder  H help"
                    .to_string()
            }
            EditorFocusArea::Export => "C copy json  S save  H help".to_string(),
        }
    }

    fn render_modal(&self, frame: &mut Frame, area: Rect) {
        let Some(modal) = &self.modal else {
            return;
        };
        let popup = centered_rect(60, 40, area);
        frame.render_widget(Clear, popup);
        let lines = match modal {
            EditorModal::Help => vec![
                Line::from("tty0 tween clip authoring help"),
                Line::from(""),
                Line::from("Global"),
                Line::from("`1/2/3` switch tabs  `Tab` switch focus  `S` save  `Esc` close editor"),
                Line::from("`[` / `]` scrub by measure"),
                Line::from(""),
                Line::from("Parameter Lanes"),
                Line::from("`Up/Down` choose parameter  `N` new clip  `D` duplicate  `R` rename"),
                Line::from("`Delete` remove unused clip"),
                Line::from(""),
                Line::from("Arrangement"),
                Line::from("`Left/Right` scrub  `P` add placement for selected lane clip at playhead"),
                Line::from("`Enter` edit selected placement  `,` / `.` move selection"),
                Line::from("`-` / `=` change repeats  `Delete` remove placement"),
                Line::from(""),
                Line::from("Lane Clip Editor"),
                Line::from("`Up/Down` choose clip  `Left/Right` choose step  `N` new clip on lane"),
                Line::from("`C` add tween step  `E` or `Enter` edit selected step"),
                Line::from("`D` duplicate clip  `Y` duplicate step  `U/J` reorder step"),
                Line::from("`Delete` remove step  `R` rename clip"),
                Line::from("`L` edit clip length"),
                Line::from(""),
                Line::from("Authoring model"),
                Line::from("Each clip belongs to exactly one parameter lane."),
                Line::from("Placements reuse the clip; edits update every placement on that lane."),
                Line::from(""),
                Line::from("Press `Esc` to close help."),
            ],
            EditorModal::NewClip(dialog) => vec![
                Line::from("new clip"),
                Line::from(""),
                Line::from(format!(
                    "{} name: {}",
                    if dialog.field_index == 0 { ">" } else { " " },
                    dialog.name
                )),
                Line::from(format!(
                    "{} length_beats: {}",
                    if dialog.field_index == 1 { ">" } else { " " },
                    dialog.length_beats
                )),
                Line::from(""),
                Line::from("Enter save  Up/Down field  type to edit"),
            ],
            EditorModal::Placement(dialog) => vec![
                Line::from(if dialog.editing_existing {
                    "edit placement"
                } else {
                    "new placement"
                }),
                Line::from(""),
                Line::from(format!(
                    "{} start_beat: {}",
                    if dialog.field_index == 0 { ">" } else { " " },
                    dialog.start_beat
                )),
                Line::from(format!(
                    "{} repeats: {}",
                    if dialog.field_index == 1 { ">" } else { " " },
                    dialog.repeats
                )),
                Line::from(""),
                Line::from("track follows the selected parameter lane"),
                Line::from("Enter save  Up/Down field  type to edit"),
            ],
            EditorModal::StepEditor(dialog) => {
                let mut lines = vec![
                    Line::from(format!(
                        "{} {}",
                        if dialog.step_index.is_some() {
                            "edit step"
                        } else {
                            "new step"
                        },
                        dialog.lane.label()
                    )),
                    Line::from(""),
                ];
                match &dialog.value_kind {
                    StepValueEditor::Float { value } => {
                        lines.push(Line::from(format!(
                            "{} to: {}",
                            if dialog.field_index == 0 { ">" } else { " " },
                            value
                        )));
                        lines.push(Line::from(format!(
                            "{} duration_beats: {}",
                            if dialog.field_index == 1 { ">" } else { " " },
                            dialog.duration_beats
                        )));
                        lines.push(Line::from(format!(
                            "{} ease: {}",
                            if dialog.field_index == 2 { ">" } else { " " },
                            dialog.ease.label()
                        )));
                    }
                    StepValueEditor::Color { red, green, blue } => {
                        lines.push(Line::from(format!(
                            "{} red: {}",
                            if dialog.field_index == 0 { ">" } else { " " },
                            red
                        )));
                        lines.push(Line::from(format!(
                            "{} green: {}",
                            if dialog.field_index == 1 { ">" } else { " " },
                            green
                        )));
                        lines.push(Line::from(format!(
                            "{} blue: {}",
                            if dialog.field_index == 2 { ">" } else { " " },
                            blue
                        )));
                        lines.push(Line::from(format!(
                            "{} duration_beats: {}",
                            if dialog.field_index == 3 { ">" } else { " " },
                            dialog.duration_beats
                        )));
                        lines.push(Line::from(format!(
                            "{} ease: {}",
                            if dialog.field_index == 4 { ">" } else { " " },
                            dialog.ease.label()
                        )));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from("Enter save  Up/Down field  Left/Right ease"));
                lines
            }
        };
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(Color::Rgb(12, 18, 20)).fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" tween editor ")
                        .border_style(Style::default().fg(CYAN)),
                ),
            popup,
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
            EditorTab::Song => self.handle_song_key(key, session, record_id),
            EditorTab::Export => self.handle_export_key(key, record_id),
        }
    }

    fn handle_state_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match key {
            KeyCode::Up => {
                self.state_field_index = self.state_field_index.saturating_sub(1);
            }
            KeyCode::Down => {
                self.state_field_index =
                    (self.state_field_index + 1).min(ParameterField::ALL.len().saturating_sub(1));
            }
            KeyCode::Enter => self.begin_state_numeric_edit(record_id),
            KeyCode::Char('-') => self.nudge_state_field(record_id, false, -1.0),
            KeyCode::Char('=') => self.nudge_state_field(record_id, false, 1.0),
            KeyCode::Char('_') => self.nudge_state_field(record_id, true, -1.0),
            KeyCode::Char('+') => self.nudge_state_field(record_id, true, 1.0),
            _ => {}
        }
        true
    }

    fn handle_song_key(
        &mut self,
        key: KeyCode,
        session: &mut SessionModel,
        record_id: &str,
    ) -> bool {
        match self.focus_area {
            EditorFocusArea::ClipLibrary => self.handle_clip_library_key(key, record_id),
            EditorFocusArea::Arrangement => self.handle_arrangement_key(key, session, record_id),
            EditorFocusArea::ClipTracks => self.handle_clip_tracks_key(key, record_id),
            _ => true,
        }
    }

    fn handle_clip_library_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        match key {
            KeyCode::Up => {
                let lane = ChromaticBulgeGridLaneId::ALL
                    [self.lane_index.saturating_sub(1)];
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    self.lane_index = self.lane_index.saturating_sub(1);
                    draft.ensure_selected_clip_for_lane(lane);
                }
            }
            KeyCode::Down => {
                let next_index =
                    (self.lane_index + 1).min(ChromaticBulgeGridLaneId::ALL.len() - 1);
                let lane = ChromaticBulgeGridLaneId::ALL[next_index];
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    self.lane_index = next_index;
                    draft.ensure_selected_clip_for_lane(lane);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.modal = Some(EditorModal::NewClip(NewClipDialog {
                    field_index: 0,
                    name: String::new(),
                    length_beats: "4.0".to_string(),
                }));
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(name) = draft.duplicate_selected_clip() {
                        self.set_status(format!("duplicated {}", name));
                    } else {
                        self.set_status("no selected clip");
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.begin_clip_rename(record_id),
            KeyCode::Delete | KeyCode::Backspace => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.delete_selected_clip() {
                        Ok(Some(name)) => self.set_status(format!("deleted {}", name)),
                        Ok(None) => self.set_status("no selected clip"),
                        Err(message) => self.set_status(message),
                    }
                }
            }
            KeyCode::Enter => self.focus_area = EditorFocusArea::ClipTracks,
            _ => {}
        }
        self.sync_selection(record_id);
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
            }
            KeyCode::Down | KeyCode::Char('.') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_placement_delta(1);
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => self.open_placement_dialog(session, record_id, false),
            KeyCode::Enter => self.open_placement_dialog(session, record_id, true),
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
            }
            _ => {}
        }
        self.sync_selection(record_id);
        true
    }

    fn handle_clip_tracks_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        let lane = self.selected_lane();
        match key {
            KeyCode::Up => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_clip_delta_for_lane(lane, -1);
                }
            }
            KeyCode::Down => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.select_clip_delta_for_lane(lane, 1);
                }
            }
            KeyCode::Char(',') => {
                self.step_index = self.step_index.saturating_sub(1);
            }
            KeyCode::Char('.') => self.step_index += 1,
            KeyCode::Left => {
                self.step_index = self.step_index.saturating_sub(1);
            }
            KeyCode::Right => {
                self.step_index += 1;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.modal = Some(EditorModal::NewClip(NewClipDialog {
                    field_index: 0,
                    name: String::new(),
                    length_beats: "4.0".to_string(),
                }));
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(name) = draft.duplicate_selected_clip() {
                        self.set_status(format!("duplicated {}", name));
                    } else {
                        self.set_status("no selected clip");
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.begin_clip_rename(record_id),
            KeyCode::Char('c') | KeyCode::Char('C') => self.open_step_dialog(record_id, lane, None),
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
                self.open_step_dialog(record_id, lane, Some(self.step_index))
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    match draft.duplicate_step(lane, self.step_index) {
                        Ok(index) => {
                            self.step_index = index;
                            self.set_status("step duplicated");
                        }
                        Err(message) => self.set_status(message),
                    }
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(index) = draft.move_step(lane, self.step_index, -1) {
                        self.step_index = index;
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Char('J') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if let Some(index) = draft.move_step(lane, self.step_index, 1) {
                        self.step_index = index;
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => self.begin_clip_length_edit(record_id),
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('x') | KeyCode::Char('X') => {
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    if draft.delete_step(lane, self.step_index) {
                        self.set_status("step deleted");
                    } else {
                        self.set_status("no step selected");
                    }
                }
            }
            _ => {}
        }
        self.sync_selection(record_id);
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

    fn handle_modal_key(&mut self, key: KeyCode, record_id: &str) -> bool {
        let Some(mut modal) = self.modal.take() else {
            return false;
        };
        let keep_open = match &mut modal {
            EditorModal::Help => !matches!(key, KeyCode::Esc | KeyCode::Enter),
            EditorModal::NewClip(dialog) => self.handle_new_clip_modal_key(key, record_id, dialog),
            EditorModal::Placement(dialog) => {
                self.handle_placement_modal_key(key, record_id, dialog)
            }
            EditorModal::StepEditor(dialog) => self.handle_step_modal_key(key, record_id, dialog),
        };
        if keep_open && self.modal.is_none() {
            self.modal = Some(modal);
        }
        keep_open
    }

    fn handle_new_clip_modal_key(
        &mut self,
        key: KeyCode,
        record_id: &str,
        dialog: &mut NewClipDialog,
    ) -> bool {
        match key {
            KeyCode::Esc => return false,
            KeyCode::Up => dialog.field_index = dialog.field_index.saturating_sub(1),
            KeyCode::Down => dialog.field_index = (dialog.field_index + 1).min(1),
            KeyCode::Backspace => {
                if dialog.field_index == 0 {
                    dialog.name.pop();
                } else {
                    dialog.length_beats.pop();
                }
            }
            KeyCode::Char(ch) if ch.is_ascii_graphic() || ch == ' ' => {
                if dialog.field_index == 0 {
                    dialog.name.push(ch);
                } else if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                    dialog.length_beats.push(ch);
                }
            }
            KeyCode::Enter => {
                let Ok(length_beats) = dialog.length_beats.parse::<f32>() else {
                    self.set_status("invalid clip length");
                    return true;
                };
                if dialog.name.trim().is_empty() {
                    self.set_status("clip name required");
                    return true;
                }
                let lane = self.selected_lane();
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    draft.create_clip(lane, dialog.name.trim(), length_beats.max(0.25));
                }
                self.focus_area = EditorFocusArea::ClipTracks;
                self.sync_selection(record_id);
                return false;
            }
            _ => {}
        }
        true
    }

    fn handle_placement_modal_key(
        &mut self,
        key: KeyCode,
        record_id: &str,
        dialog: &mut PlacementDialog,
    ) -> bool {
        match key {
            KeyCode::Esc => return false,
            KeyCode::Up => dialog.field_index = dialog.field_index.saturating_sub(1),
            KeyCode::Down => dialog.field_index = (dialog.field_index + 1).min(1),
            KeyCode::Backspace => match dialog.field_index {
                0 => {
                    dialog.start_beat.pop();
                }
                _ => {
                    dialog.repeats.pop();
                }
            },
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == '.' || ch == '-' => {
                match dialog.field_index {
                    0 => dialog.start_beat.push(ch),
                    _ => dialog.repeats.push(ch),
                }
            }
            KeyCode::Enter => {
                let Ok(start_beat) = dialog.start_beat.parse::<f32>() else {
                    self.set_status("invalid start beat");
                    return true;
                };
                let Ok(repeats) = dialog.repeats.parse::<u32>() else {
                    self.set_status("invalid repeats");
                    return true;
                };
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    let result = if dialog.editing_existing {
                        draft.update_selected_placement(start_beat, repeats)
                    } else {
                        draft.add_placement(start_beat, repeats)
                    };
                    match result {
                        Ok(()) => {}
                        Err(message) => {
                            self.set_status(message);
                            return true;
                        }
                    }
                }
                self.sync_selection(record_id);
                return false;
            }
            _ => {}
        }
        true
    }

    fn handle_step_modal_key(
        &mut self,
        key: KeyCode,
        record_id: &str,
        dialog: &mut StepEditorDialog,
    ) -> bool {
        let max_field = match dialog.value_kind {
            StepValueEditor::Float { .. } => 2,
            StepValueEditor::Color { .. } => 4,
        };
        match key {
            KeyCode::Esc => return false,
            KeyCode::Up => dialog.field_index = dialog.field_index.saturating_sub(1),
            KeyCode::Down => dialog.field_index = (dialog.field_index + 1).min(max_field),
            KeyCode::Left if dialog.field_index == max_field => {
                dialog.ease = dialog.ease.prev();
            }
            KeyCode::Right if dialog.field_index == max_field => {
                dialog.ease = dialog.ease.next();
            }
            KeyCode::Backspace => {
                dialog.active_buffer_mut().pop();
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == '.' || ch == '-' => {
                dialog.active_buffer_mut().push(ch);
            }
            KeyCode::Enter => {
                let Some(step) = dialog.to_step() else {
                    self.set_status("invalid tween step");
                    return true;
                };
                if let Some(draft) = self.drafts.draft_mut(record_id) {
                    let result = match dialog.step_index {
                        Some(index) => draft.update_step(dialog.lane, index, step),
                        None => draft.add_step(dialog.lane, step),
                    };
                    match result {
                        Ok(index) => self.step_index = index,
                        Err(message) => {
                            self.set_status(message);
                            return true;
                        }
                    }
                }
                self.sync_selection(record_id);
                return false;
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
        self.numeric_input = Some(NumericInputState {
            target: NumericEditTarget::StateField(field),
            buffer: format_parameter_value(draft.base_state(), field),
            replace_on_type: true,
        });
    }

    fn begin_clip_length_edit(&mut self, record_id: &str) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let Some(clip) = draft.selected_clip() else {
            return;
        };
        self.numeric_input = Some(NumericInputState {
            target: NumericEditTarget::ClipLength,
            buffer: format!("{:.3}", clip.length_beats),
            replace_on_type: true,
        });
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
            NumericEditTarget::ClipLength => {
                if let Err(message) = draft.set_selected_clip_length(value.max(0.25)) {
                    self.set_status(message);
                    return;
                }
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
                }
            }
        }
    }

    fn nudge_state_field(&mut self, record_id: &str, coarse: bool, direction: f32) {
        let field = self.selected_state_field();
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            return;
        };
        let current = draft.parameter_value(field);
        draft.set_base_field(field, current + parameter_step(field, coarse) * direction);
    }

    fn open_placement_dialog(
        &mut self,
        session: &SessionModel,
        record_id: &str,
        editing_existing: bool,
    ) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let (start_beat, repeats) = if editing_existing {
            draft
                .selected_placement()
                .map(|placement| (placement.start_beat, placement.repeats))
                .unwrap_or((0.0, 1))
        } else {
            (current_global_beat(session.playback_clock(), draft.timeline()).max(0.0), 1)
        };
        self.modal = Some(EditorModal::Placement(PlacementDialog {
            field_index: 0,
            start_beat: format!("{start_beat:.3}"),
            repeats: repeats.to_string(),
            editing_existing,
        }));
    }

    fn open_step_dialog(
        &mut self,
        record_id: &str,
        lane: ChromaticBulgeGridLaneId,
        step_index: Option<usize>,
    ) {
        let Some(draft) = self.drafts.draft_for(record_id) else {
            return;
        };
        let Some(dialog) = draft.step_dialog(lane, step_index) else {
            self.set_status("create a clip on this lane before editing steps");
            return;
        };
        self.modal = Some(EditorModal::StepEditor(dialog));
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

    fn sync_selection(&mut self, record_id: &str) {
        let lane = self.selected_lane();
        let Some(draft) = self.drafts.draft_mut(record_id) else {
            return;
        };
        draft.ensure_selection();
        draft.ensure_selected_clip_for_lane(lane);
        self.step_index = self.step_index.min(draft.step_count(lane).saturating_sub(1));
    }

    fn selected_state_field(&self) -> ParameterField {
        ParameterField::ALL[self.state_field_index]
    }

    fn selected_lane(&self) -> ChromaticBulgeGridLaneId {
        ChromaticBulgeGridLaneId::ALL[self.lane_index]
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
                        "save migrated legacy lane automation into reusable timeline clips",
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
            normalize_editor_timeline(timeline)
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

    pub fn ensure_selected_clip_for_lane(&mut self, lane: ChromaticBulgeGridLaneId) {
        let selected_matches = self
            .selected_clip()
            .and_then(editor_clip_lane)
            .is_some_and(|selected_lane| selected_lane == lane);
        if selected_matches {
            return;
        }
        self.clip_editor.selected_clip_id.0 = self
            .timeline()
            .clips
            .iter()
            .find(|clip| editor_clip_lane(clip) == Some(lane))
            .map(|clip| clip.id.clone());
    }

    pub fn clips_for_lane(&self, lane: ChromaticBulgeGridLaneId) -> Vec<&ChromaticBulgeGridClip> {
        self.timeline()
            .clips
            .iter()
            .filter(|clip| editor_clip_lane(clip) == Some(lane))
            .collect()
    }

    pub fn selected_clip_for_lane(
        &self,
        lane: ChromaticBulgeGridLaneId,
    ) -> Option<&ChromaticBulgeGridClip> {
        let selected = self.selected_clip()?;
        if editor_clip_lane(selected) == Some(lane) {
            Some(selected)
        } else {
            None
        }
    }

    pub fn selected_clip_index_for_lane(&self, lane: ChromaticBulgeGridLaneId) -> Option<usize> {
        let clip_id = self.selected_clip_for_lane(lane)?.id.as_str();
        self.clips_for_lane(lane)
            .iter()
            .position(|clip| clip.id == clip_id)
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

    pub fn placement_count_for_lane(&self, lane: ChromaticBulgeGridLaneId) -> usize {
        self.timeline()
            .arrangement
            .iter()
            .filter(|placement| ChromaticBulgeGridLaneId::from_track_index(placement.track) == Some(lane))
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

    pub fn select_clip_delta_for_lane(&mut self, lane: ChromaticBulgeGridLaneId, delta: isize) {
        let clips = self.clips_for_lane(lane);
        let len = clips.len() as isize;
        if len == 0 {
            self.clip_editor.selected_clip_id.0 = None;
            return;
        }
        let current = self.selected_clip_index_for_lane(lane).unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, len - 1) as usize;
        self.clip_editor.selected_clip_id.0 = Some(clips[next].id.clone());
    }

    pub fn create_clip(
        &mut self,
        lane: ChromaticBulgeGridLaneId,
        name: &str,
        length_beats: f32,
    ) {
        let id = self.next_clip_id(name);
        self.timeline_mut().clips.push(ChromaticBulgeGridClip {
            id: id.clone(),
            name: name.to_string(),
            length_beats: length_beats.max(0.25),
            color: default_clip_color(id.len()),
            authoring: Some(ChromaticBulgeGridClipAuthoring {
                tracks: vec![ClipParamTrack {
                    lane,
                    steps: Vec::new(),
                }],
            }),
            lanes: ChromaticBulgeGridAutomationLanes::default(),
        });
        self.clip_editor.selected_clip_id.0 = Some(id);
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
            authoring: clip.authoring,
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
        let selected_lane = self.selected_clip().and_then(editor_clip_lane);
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
        self.clip_editor.selected_clip_id.0 = selected_lane
            .and_then(|lane| {
                self.timeline()
                    .clips
                    .iter()
                    .find(|clip| editor_clip_lane(clip) == Some(lane))
                    .map(|clip| clip.id.clone())
            })
            .or_else(|| self.timeline().clips.first().map(|clip| clip.id.clone()));
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

    pub fn add_placement(&mut self, beat: f32, repeats: u32) -> Result<(), String> {
        let Some(clip) = self.selected_clip().cloned() else {
            return Err("no selected clip".to_string());
        };
        let track = editor_clip_lane(&clip)
            .map(ChromaticBulgeGridLaneId::track_index)
            .unwrap_or(0);
        let placement = ClipPlacement {
            clip_id: clip.id.clone(),
            start_beat: beat.max(0.0),
            track,
            repeats: repeats.max(1),
        };
        self.validate_placement(None, &placement)?;
        self.timeline_mut().arrangement.push(placement);
        self.timeline_mut()
            .arrangement
            .sort_by(|left, right| left.track.cmp(&right.track).then_with(|| left.start_beat.total_cmp(&right.start_beat)));
        self.clip_editor.selected_placement_index.0 = self
            .timeline()
            .arrangement
            .iter()
            .position(|candidate| {
                candidate.clip_id == clip.id
                    && (candidate.start_beat - beat).abs() < 0.0001
                    && candidate.track == track
            });
        Ok(())
    }

    pub fn update_selected_placement(
        &mut self,
        beat: f32,
        repeats: u32,
    ) -> Result<(), String> {
        let Some(index) = self.clip_editor.selected_placement_index.0 else {
            return Err("no selected placement".to_string());
        };
        let mut placement = self.timeline().arrangement[index].clone();
        let Some(clip) = self.timeline().clip_by_id(&placement.clip_id) else {
            return Err("placement references missing clip".to_string());
        };
        placement.track = editor_clip_lane(clip)
            .map(ChromaticBulgeGridLaneId::track_index)
            .unwrap_or(placement.track);
        placement.start_beat = beat.max(0.0);
        placement.repeats = repeats.max(1);
        self.validate_placement(Some(index), &placement)?;
        self.timeline_mut().arrangement[index] = placement;
        self.timeline_mut()
            .arrangement
            .sort_by(|left, right| left.track.cmp(&right.track).then_with(|| left.start_beat.total_cmp(&right.start_beat)));
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

    pub fn track_for_lane(&self, lane: ChromaticBulgeGridLaneId) -> Option<&ClipParamTrack> {
        self.selected_clip()?
            .authoring
            .as_ref()?
            .tracks
            .iter()
            .find(|track| track.lane == lane)
    }

    pub fn track_for_selected_clip(&self) -> Option<&ClipParamTrack> {
        let clip = self.selected_clip()?;
        let lane = editor_clip_lane(clip)?;
        self.track_for_lane(lane)
    }

    pub fn step_count(&self, lane: ChromaticBulgeGridLaneId) -> usize {
        self.track_for_lane(lane).map(|track| track.steps.len()).unwrap_or(0)
    }

    pub fn toggle_track_enabled(&mut self, lane: ChromaticBulgeGridLaneId) {
        let Some(authoring) = self
            .selected_clip_mut()
            .and_then(|clip| clip.authoring.as_mut())
        else {
            return;
        };
        if authoring.tracks.len() == 1 && authoring.tracks[0].lane == lane {
            return;
        }
        authoring.tracks.clear();
        authoring.tracks.push(ClipParamTrack {
            lane,
            steps: Vec::new(),
        });
        self.recompile_selected_clip();
    }

    pub fn add_step(
        &mut self,
        lane: ChromaticBulgeGridLaneId,
        step: ClipTweenStep,
    ) -> Result<usize, String> {
        let track = self
            .track_for_lane_mut(lane)
            .ok_or_else(|| "enable lane before adding steps".to_string())?;
        track.steps.push(step);
        let index = track.steps.len() - 1;
        self.recompile_selected_clip();
        Ok(index)
    }

    pub fn update_step(
        &mut self,
        lane: ChromaticBulgeGridLaneId,
        index: usize,
        step: ClipTweenStep,
    ) -> Result<usize, String> {
        let track = self
            .track_for_lane_mut(lane)
            .ok_or_else(|| "lane disabled".to_string())?;
        let Some(existing) = track.steps.get_mut(index) else {
            return Err("no step selected".to_string());
        };
        *existing = step;
        self.recompile_selected_clip();
        Ok(index)
    }

    pub fn duplicate_step(
        &mut self,
        lane: ChromaticBulgeGridLaneId,
        index: usize,
    ) -> Result<usize, String> {
        let track = self
            .track_for_lane_mut(lane)
            .ok_or_else(|| "lane disabled".to_string())?;
        let Some(step) = track.steps.get(index).cloned() else {
            return Err("no step selected".to_string());
        };
        let next = index + 1;
        track.steps.insert(next, step);
        self.recompile_selected_clip();
        Ok(next)
    }

    pub fn move_step(
        &mut self,
        lane: ChromaticBulgeGridLaneId,
        index: usize,
        delta: isize,
    ) -> Option<usize> {
        let track = self.track_for_lane_mut(lane)?;
        if index >= track.steps.len() {
            return None;
        }
        let next = (index as isize + delta).clamp(0, track.steps.len() as isize - 1) as usize;
        if next == index {
            return Some(index);
        }
        track.steps.swap(index, next);
        self.recompile_selected_clip();
        Some(next)
    }

    pub fn delete_step(&mut self, lane: ChromaticBulgeGridLaneId, index: usize) -> bool {
        let Some(track) = self.track_for_lane_mut(lane) else {
            return false;
        };
        if index >= track.steps.len() {
            return false;
        }
        track.steps.remove(index);
        self.recompile_selected_clip();
        true
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
        let old_length = self.timeline().clips[index].length_beats;
        let old_lanes = self.timeline().clips[index].lanes.clone();
        self.timeline_mut().clips[index].length_beats = value;
        self.recompile_clip_at(index);
        if let Some(error) = self.validate_current_timeline() {
            self.timeline_mut().clips[index].length_beats = old_length;
            self.timeline_mut().clips[index].lanes = old_lanes;
            return Err(error);
        }
        Ok(())
    }

    fn step_dialog(
        &self,
        lane: ChromaticBulgeGridLaneId,
        step_index: Option<usize>,
    ) -> Option<StepEditorDialog> {
        let track = self.track_for_lane(lane)?;
        let step = step_index.and_then(|index| track.steps.get(index));
        if lane.is_color() {
            let value = match step.map(|step| &step.to) {
                Some(ClipTweenValue::Color(value)) => *value,
                Some(ClipTweenValue::Float(_)) => return None,
                None => self.base_state_color_for_lane(lane),
            };
            Some(StepEditorDialog {
                lane,
                step_index,
                field_index: 0,
                value_kind: StepValueEditor::Color {
                    red: format!("{:.3}", value[0]),
                    green: format!("{:.3}", value[1]),
                    blue: format!("{:.3}", value[2]),
                },
                duration_beats: format!("{:.3}", step.map_or(0.25, |step| step.duration_beats)),
                ease: step.map_or(ClipTweenEase::Linear, |step| step.ease),
            })
        } else {
            let value = match step.map(|step| &step.to) {
                Some(ClipTweenValue::Float(value)) => *value,
                Some(ClipTweenValue::Color(_)) => return None,
                None => self.base_float_for_lane(lane),
            };
            Some(StepEditorDialog {
                lane,
                step_index,
                field_index: 0,
                value_kind: StepValueEditor::Float {
                    value: format!("{value:.3}"),
                },
                duration_beats: format!("{:.3}", step.map_or(0.25, |step| step.duration_beats)),
                ease: step.map_or(ClipTweenEase::Linear, |step| step.ease),
            })
        }
    }

    pub fn set_base_field(&mut self, field: ParameterField, value: f32) {
        let states = self.working.params.shader_states.as_mut().unwrap();
        set_parameter_field(&mut states.playing, field, value);
        set_parameter_field(&mut states.idle, field, value);
        let clip_count = self.timeline().clips.len();
        for index in 0..clip_count {
            if self.timeline().clips[index].authoring.is_some() {
                self.recompile_clip_at(index);
            }
        }
    }

    fn base_float_for_lane(&self, lane: ChromaticBulgeGridLaneId) -> f32 {
        let state = self.base_state();
        match lane {
            ChromaticBulgeGridLaneId::MotionRate => state.motion_rate,
            ChromaticBulgeGridLaneId::LatticeDensity => state.lattice_density,
            ChromaticBulgeGridLaneId::CircleRadius => state.circle_radius,
            ChromaticBulgeGridLaneId::CircleFalloffStart => state.circle_falloff_start,
            ChromaticBulgeGridLaneId::CircleFalloffEnd => state.circle_falloff_end,
            ChromaticBulgeGridLaneId::BulgeAmount => state.bulge_amount,
            ChromaticBulgeGridLaneId::RimGuard => state.rim_guard,
            ChromaticBulgeGridLaneId::RimExponent => state.rim_exponent,
            ChromaticBulgeGridLaneId::RimWarp => state.rim_warp,
            ChromaticBulgeGridLaneId::SpacingMaxPx => state.spacing_max_px,
            ChromaticBulgeGridLaneId::SpacingMinPx => state.spacing_min_px,
            ChromaticBulgeGridLaneId::DotSize => state.dot_size,
            ChromaticBulgeGridLaneId::OuterDotScale => state.outer_dot_scale,
            ChromaticBulgeGridLaneId::EdgeSoftness => state.edge_softness,
            ChromaticBulgeGridLaneId::ChromaticAberration => state.chromatic_aberration,
            ChromaticBulgeGridLaneId::ScrollBase => state.scroll_base,
            ChromaticBulgeGridLaneId::ScrollMotionScale => state.scroll_motion_scale,
            ChromaticBulgeGridLaneId::ScrollMotionFloor => state.scroll_motion_floor,
            ChromaticBulgeGridLaneId::ScrollMotionCeiling => state.scroll_motion_ceiling,
            ChromaticBulgeGridLaneId::ColorCycleRate => state.color_cycle_rate,
            ChromaticBulgeGridLaneId::InnerAlpha => state.inner_alpha,
            ChromaticBulgeGridLaneId::ColdColor | ChromaticBulgeGridLaneId::HotColor => 0.0,
        }
    }

    fn base_state_color_for_lane(&self, lane: ChromaticBulgeGridLaneId) -> [f32; 3] {
        let state = self.base_state();
        match lane {
            ChromaticBulgeGridLaneId::ColdColor => state.cold_color,
            ChromaticBulgeGridLaneId::HotColor => state.hot_color,
            _ => [1.0, 1.0, 1.0],
        }
    }

    fn track_for_lane_mut(
        &mut self,
        lane: ChromaticBulgeGridLaneId,
    ) -> Option<&mut ClipParamTrack> {
        self.selected_clip_mut()?
            .authoring
            .as_mut()?
            .tracks
            .iter_mut()
            .find(|track| track.lane == lane)
    }

    fn recompile_selected_clip(&mut self) {
        if let Some(index) = self.selected_clip_index() {
            self.recompile_clip_at(index);
        }
    }

    fn recompile_clip_at(&mut self, index: usize) {
        let Some(authoring) = self.timeline().clips[index].authoring.clone() else {
            return;
        };
        let base = self.base_state();
        let length_beats = self.timeline().clips[index].length_beats;
        self.timeline_mut().clips[index].lanes =
            compile_authoring_lanes(base, length_beats, &authoring);
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
                if placement.track == existing.track {
                    return Err("placement would overlap existing track".to_string());
                }
                if !clip.authored_lanes().is_disjoint(&existing_clip.authored_lanes()) {
                    return Err("placement would overlap another clip on the same parameter".to_string());
                }
            }
        }
        Ok(())
    }

    fn validate_current_timeline(&self) -> Option<String> {
        let timeline = self.timeline();
        for placement in &timeline.arrangement {
            let clip = timeline.clip_by_id(&placement.clip_id)?;
            if placement.end_beat(clip) > timeline.total_beats() + 0.0001 {
                return Some("clip length change would extend placement past timeline end".to_string());
            }
        }
        let mut spans = timeline
            .arrangement
            .iter()
            .filter_map(|placement| {
                let clip = timeline.clip_by_id(&placement.clip_id)?;
                Some((
                    placement.start_beat,
                    placement.end_beat(clip),
                    placement.track,
                    clip.authored_lanes(),
                ))
            })
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.2.cmp(&right.2)));
        for index in 0..spans.len() {
            for next in spans.iter().skip(index + 1) {
                if next.0 >= spans[index].1 - 0.0001 {
                    break;
                }
                if spans[index].2 == next.2 {
                    return Some("clip length change would create overlapping placements".to_string());
                }
                if !spans[index].3.is_disjoint(&next.3) {
                    return Some("clip length change would overlap another clip on the same parameter".to_string());
                }
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

impl StepEditorDialog {
    fn active_buffer_mut(&mut self) -> &mut String {
        match (&mut self.value_kind, self.field_index) {
            (StepValueEditor::Float { value }, 0) => value,
            (StepValueEditor::Float { .. }, 1) => &mut self.duration_beats,
            (StepValueEditor::Float { .. }, _) => &mut self.duration_beats,
            (StepValueEditor::Color { red, .. }, 0) => red,
            (StepValueEditor::Color { green, .. }, 1) => green,
            (StepValueEditor::Color { blue, .. }, 2) => blue,
            (StepValueEditor::Color { .. }, 3) => &mut self.duration_beats,
            (StepValueEditor::Color { .. }, _) => &mut self.duration_beats,
        }
    }

    fn to_step(&self) -> Option<ClipTweenStep> {
        let duration_beats = self.duration_beats.parse::<f32>().ok()?.max(0.0);
        let to = match &self.value_kind {
            StepValueEditor::Float { value } => ClipTweenValue::Float(value.parse::<f32>().ok()?),
            StepValueEditor::Color { red, green, blue } => ClipTweenValue::Color([
                red.parse::<f32>().ok()?,
                green.parse::<f32>().ok()?,
                blue.parse::<f32>().ok()?,
            ]),
        };
        Some(ClipTweenStep {
            to,
            duration_beats,
            ease: self.ease,
        })
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
            Self::ClipLength => "clip.length_beats",
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

impl EditorFocusArea {
    fn label(self) -> &'static str {
        match self {
            Self::StateParams => "state",
            Self::ClipLibrary => "library",
            Self::Arrangement => "arrangement",
            Self::ClipTracks => "tracks",
            Self::Export => "export",
        }
    }
}

impl ClipTweenEase {
    fn label(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Linear => "linear",
            Self::SineIn => "sine_in",
            Self::SineOut => "sine_out",
            Self::SineInOut => "sine_in_out",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Hold => Self::Linear,
            Self::Linear => Self::SineIn,
            Self::SineIn => Self::SineOut,
            Self::SineOut => Self::SineInOut,
            Self::SineInOut => Self::Hold,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Hold => Self::SineInOut,
            Self::Linear => Self::Hold,
            Self::SineIn => Self::Linear,
            Self::SineOut => Self::SineIn,
            Self::SineInOut => Self::SineOut,
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
            authoring: Some(ChromaticBulgeGridClipAuthoring {
                tracks: vec![ClipParamTrack {
                    lane: ChromaticBulgeGridLaneId::MotionRate,
                    steps: Vec::new(),
                }],
            }),
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

fn normalize_editor_timeline(timeline: &ChromaticBulgeGridClipTimeline) -> ChromaticBulgeGridClipTimeline {
    let mut normalized = timeline.clone();
    for clip in &mut normalized.clips {
        if let Some(lane) = editor_clip_lane(clip) {
            if let Some(authoring) = clip.authoring.as_mut() {
                if authoring.tracks.is_empty() {
                    authoring.tracks.push(ClipParamTrack {
                        lane,
                        steps: Vec::new(),
                    });
                } else if authoring.tracks.len() > 1 {
                    authoring.tracks.retain(|track| track.lane == lane);
                }
            }
        }
    }
    let lane_by_clip_id = normalized
        .clips
        .iter()
        .filter_map(|clip| Some((clip.id.clone(), editor_clip_lane(clip)?)))
        .collect::<HashMap<_, _>>();
    for placement in &mut normalized.arrangement {
        if let Some(lane) = lane_by_clip_id.get(&placement.clip_id) {
            placement.track = lane.track_index();
        }
    }
    normalized
}

fn editor_clip_lane(clip: &ChromaticBulgeGridClip) -> Option<ChromaticBulgeGridLaneId> {
    clip.authoring
        .as_ref()
        .and_then(|authoring| authoring.tracks.first().map(|track| track.lane))
        .or_else(|| clip.primary_lane())
        .or_else(|| clip.authored_lanes().into_iter().next())
}

fn next_focus_area(tab: EditorTab, current: EditorFocusArea) -> EditorFocusArea {
    match tab {
        EditorTab::State => EditorFocusArea::StateParams,
        EditorTab::Song => match current {
            EditorFocusArea::ClipLibrary => EditorFocusArea::Arrangement,
            EditorFocusArea::Arrangement => EditorFocusArea::ClipTracks,
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

fn current_global_beat(playback: PlaybackClock, timeline: &ChromaticBulgeGridClipTimeline) -> f32 {
    playback.current_time_secs.max(0.0) * timeline.bpm.max(1.0) / 60.0
}

fn build_arrangement_grid(
    draft: &ChromaticBulgeGridEditorDraft,
    current_beat: f32,
    selected_lane: ChromaticBulgeGridLaneId,
    visible_rows: usize,
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

    let mut lines = vec![
        Line::from(format!(
            "0{:>width$}",
            format!("{:.0} beats", total_beats),
            width = width - 1
        )),
        Line::from(measure_row.into_iter().collect::<String>()),
        Line::from(playhead_row.into_iter().collect::<String>()),
    ];

    let selected_index = draft.clip_editor.selected_placement_index.0;
    let lane_index = selected_lane.track_index() as usize;
    let row_budget = visible_rows.max(4);
    let start_lane = lane_index.saturating_sub(row_budget / 2);
    let end_lane = (start_lane + row_budget).min(ChromaticBulgeGridLaneId::ALL.len());
    for track in start_lane as u8..end_lane as u8 {
        let mut spans = vec![Span::styled(
            format!("{:<20}", lane_label_for_track(track)),
            Style::default().fg(DIM),
        )];
        let mut cursor = 0usize;
        for (index, placement) in timeline
            .arrangement
            .iter()
            .enumerate()
            .filter(|(_, placement)| placement.track == track)
        {
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
        lines.push(Line::from(spans));
    }

    lines
}

fn build_clip_preview(clip: &ChromaticBulgeGridClip) -> String {
    let active_lanes = clip.authored_lanes();
    if active_lanes.is_empty() {
        "no automation".to_string()
    } else {
        active_lanes
            .into_iter()
            .take(3)
            .map(|lane| lane.label())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_tween_value(value: &ClipTweenValue) -> String {
    match value {
        ClipTweenValue::Float(value) => format!("{value:.3}"),
        ClipTweenValue::Color(value) => {
            format!("[{:.2},{:.2},{:.2}]", value[0], value[1], value[2])
        }
    }
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

fn lane_label_for_track(track: u8) -> &'static str {
    ChromaticBulgeGridLaneId::from_track_index(track)
        .map(ChromaticBulgeGridLaneId::label)
        .unwrap_or("unknown_lane")
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
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
    let trimmed = output.trim_matches('_');
    if trimmed.is_empty() {
        "clip".to_string()
    } else {
        trimmed.to_string()
    }
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
        ClipEditorDraft, EditorPreviewMode, ParameterField, SelectedClipId,
        SelectedPlacementIndex,
    };
    use crate::archive::{
        legacy_automation_to_timeline, ChromaticBulgeGridAutomation,
        ChromaticBulgeGridAutomationLanes, ChromaticBulgeGridLaneId, ClipPlacement, ClipTweenEase,
        ClipTweenStep, ClipTweenValue, FloatKeyframe, InterpolationMode, TrackVisualizerConfig,
    };

    #[test]
    fn mark_saved_resets_dirty_state_against_saved_payload() {
        let original = default_visualizer_config();
        let mut draft = seeded_draft();
        draft.set_base_field(ParameterField::MotionRate, 2.5);
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
                selected_clip_id: SelectedClipId(Some("imported_circle_radius".to_string())),
                selected_placement_index: SelectedPlacementIndex(Some(0)),
                preview_mode: EditorPreviewMode::TimelineWhilePaused,
            },
            opened_from_legacy: true,
        };

        assert_eq!(draft.timeline().clips.len(), 1);
        assert_eq!(draft.timeline().arrangement.len(), 1);
        assert!(draft.working.automation.is_none());
    }

    #[test]
    fn adding_step_populates_compiled_lanes() {
        let mut draft = seeded_draft();
        draft.toggle_track_enabled(ChromaticBulgeGridLaneId::CircleRadius);
        draft.clip_editor.selected_clip_id = SelectedClipId(Some("clip_1".to_string()));
        draft
            .add_step(
                ChromaticBulgeGridLaneId::CircleRadius,
                ClipTweenStep {
                    to: ClipTweenValue::Float(0.8),
                    duration_beats: 0.125,
                    ease: ClipTweenEase::SineOut,
                },
            )
            .expect("step");
        assert!(!draft.selected_clip().unwrap().lanes.circle_radius.is_empty());
    }

    #[test]
    fn increasing_repeats_blocks_overlap_if_invalid() {
        let mut draft = seeded_draft();
        draft.timeline_mut().arrangement = vec![
            ClipPlacement {
                clip_id: "clip_1".to_string(),
                start_beat: 0.0,
                track: 0,
                repeats: 1,
            },
            ClipPlacement {
                clip_id: "clip_1".to_string(),
                start_beat: 4.0,
                track: 0,
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
        draft.timeline_mut().arrangement.push(ClipPlacement {
            clip_id: "clip_1".to_string(),
            start_beat: 0.0,
            track: 0,
            repeats: 1,
        });
        let error = draft
            .delete_selected_clip()
            .expect_err("in-use clip should fail");
        assert!(error.contains("placements"));
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
                preview_mode: EditorPreviewMode::TimelineWhilePaused,
            },
            opened_from_legacy: false,
        }
    }
}
