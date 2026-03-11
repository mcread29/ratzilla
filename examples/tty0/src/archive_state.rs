use std::{cell::RefCell, rc::Rc};

use crate::{
    archive::{AccessLevel, MediaHealth, RecordDocument, WaveformMode},
    overlay_state::OverlayRenderState,
    panel_shader_visualizer::PanelShaderVisualizerLayer,
    session::{LogColorRole, RecordPageTab, SessionModel},
    state::{StateActions, StateId, StateMachineError},
    track_visualizer,
    visualizer_editor::VisualizerEditorOverlay,
};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ratzilla::widgets::GraphicsCanvasLayer;
use tachyonfx::Duration;

const BG: Color = Color::Black;
const TEXT: Color = Color::Rgb(196, 214, 198);
const DIM: Color = Color::Rgb(118, 128, 120);
const BORDER: Color = Color::DarkGray;
const CYAN: Color = Color::Rgb(110, 220, 212);
const AMBER: Color = Color::Rgb(234, 182, 92);
const RED: Color = Color::Rgb(240, 104, 96);
const GREEN: Color = Color::Rgb(130, 208, 132);
const WHITE: Color = Color::Rgb(232, 238, 232);
const MAGENTA: Color = Color::Rgb(188, 144, 228);
const RECORDS_PANEL_WIDTH: u16 = 65;

pub struct ArchiveState {
    session: Rc<RefCell<SessionModel>>,
    visual_layer: GraphicsCanvasLayer,
    panel_shader_visualizer: PanelShaderVisualizerLayer,
    overlay_state: OverlayRenderState,
    visual_runtime: Rc<RefCell<track_visualizer::TrackVisualizerRuntime>>,
    visualizer_editor: VisualizerEditorOverlay,
    pending_transition: Option<StateId>,
}

impl ArchiveState {
    pub fn new(
        session: Rc<RefCell<SessionModel>>,
        visual_layer: GraphicsCanvasLayer,
        panel_shader_visualizer: PanelShaderVisualizerLayer,
        overlay_state: OverlayRenderState,
    ) -> Self {
        Self {
            session,
            visual_layer,
            panel_shader_visualizer,
            overlay_state,
            visual_runtime: Rc::new(RefCell::new(
                track_visualizer::TrackVisualizerRuntime::default(),
            )),
            visualizer_editor: VisualizerEditorOverlay::new(),
            pending_transition: None,
        }
    }

    pub fn create(
        session: Rc<RefCell<SessionModel>>,
        visual_layer: GraphicsCanvasLayer,
        panel_shader_visualizer: PanelShaderVisualizerLayer,
        overlay_state: OverlayRenderState,
    ) -> Box<dyn StateActions> {
        Box::new(Self::new(
            session,
            visual_layer,
            panel_shader_visualizer,
            overlay_state,
        ))
    }
}

impl StateActions for ArchiveState {
    fn on_enter(&mut self) -> Result<(), StateMachineError> {
        self.pending_transition = None;
        self.overlay_state.set_editor_open(false);
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        let mut session = self.session.borrow_mut();
        if self.visualizer_editor.is_open() {
            self.visualizer_editor.handle_key(key, &mut session);
            return Ok(());
        }
        if matches!(key, KeyCode::Char('e') | KeyCode::Char('E')) {
            let record = session.current_record().clone();
            self.visualizer_editor.toggle_for_record(&record);
            return Ok(());
        }
        match key {
            KeyCode::Left => session.move_record_page(-1),
            KeyCode::Right => session.move_record_page(1),
            KeyCode::Up => session.move_record(-1),
            KeyCode::Down => session.move_record(1),
            KeyCode::Char('p') | KeyCode::Char('P') => session.toggle_playback(),
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
        if self.visualizer_editor.is_open() {
            self.overlay_state.set_editor_open(true);
            let session = self.session.borrow();
            self.visualizer_editor.render(frame, area, &session);
        } else {
            self.overlay_state.set_editor_open(false);
            if area.width < 110 || area.height < 34 {
                self.render_compact(frame, area);
            } else {
                self.render_wide(frame, area);
            }
        }
    }

    fn take_transition(&mut self) -> Option<StateId> {
        self.pending_transition.take()
    }
}

impl ArchiveState {
    fn render_wide(&self, frame: &mut Frame, area: Rect) {
        let root = Block::bordered()
            .title(" tty0 recovered analysis workstation // H help ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(BORDER));
        let inner = root.inner(area);
        frame.render_widget(root, area);

        let body = Layout::horizontal([
            Constraint::Length(RECORDS_PANEL_WIDTH),
            Constraint::Fill(3),
            Constraint::Fill(2),
        ])
        .margin(1)
        .split(inner);
        let center = Layout::vertical([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(10),
        ])
        .split(body[1]);
        let right = Layout::vertical([
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Min(9),
        ])
        .split(body[2]);

        self.render_records(frame, body[0]);
        self.render_header(frame, center[0]);
        self.render_tab_strip(frame, center[1]);
        self.render_page(frame, center[2]);
        self.render_metadata(frame, right[0]);
        self.render_playback(frame, right[1]);
        self.render_logs(frame, right[2]);
    }

    fn render_compact(&self, frame: &mut Frame, area: Rect) {
        let root = Block::bordered()
            .title(" tty0 archive // H help ")
            .border_style(Style::default().fg(BORDER));
        let inner = root.inner(area);
        frame.render_widget(root, area);

        let layout = Layout::vertical([
            Constraint::Length(12),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Min(6),
        ])
        .margin(1)
        .split(inner);

        self.render_records(frame, layout[0]);
        self.render_header(frame, layout[1]);
        self.render_tab_strip(frame, layout[2]);
        self.render_page(frame, layout[3]);
        self.render_metadata(frame, layout[4]);
        self.render_playback(frame, layout[5]);
        self.render_logs(frame, layout[6]);
    }

    fn render_records(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let mut state = ListState::default();
        state.select((session.has_records()).then_some(session.current_record_index()));

        let items = if !session.has_records() {
            vec![ListItem::new(vec![
                Line::from(Span::styled(
                    "archive manifest decoded without records",
                    Style::default().fg(AMBER),
                )),
                Line::from(Span::styled(
                    "no evidence objects are mounted in this build",
                    Style::default().fg(DIM),
                )),
            ])]
        } else {
            session
                .archive()
                .records()
                .iter()
                .map(|entry| {
                    let record = session
                        .archive()
                        .record_by_id(&entry.record_id)
                        .expect("flat record entry must resolve");
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(tree_icon(record), accent_style(record)),
                            Span::styled(" ", Style::default().fg(DIM)),
                            Span::styled(
                                record.title.clone(),
                                Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled(record.id.clone(), Style::default().fg(CYAN)),
                            Span::styled("  ", Style::default().fg(DIM)),
                            Span::styled(record.timeline.clone(), Style::default().fg(AMBER)),
                            Span::styled("  ", Style::default().fg(DIM)),
                            Span::styled(
                                format!("[{}]", record.badge_label()),
                                badge_style(record).add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("cat ", Style::default().fg(DIM)),
                            Span::styled(entry.category_label.clone(), Style::default().fg(TEXT)),
                            Span::styled("  •  ", Style::default().fg(AMBER)),
                            Span::styled(
                                record.access_level.label(),
                                access_style(record.access_level),
                            ),
                        ]),
                        Line::from(vec![Span::styled(
                            format!(
                                "{} pages • {} tags • {} links",
                                record.page_count(),
                                record.tags.len(),
                                record.related_record_ids.len()
                            ),
                            Style::default().fg(DIM),
                        )]),
                    ])
                })
                .collect::<Vec<_>>()
        };

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(" records ( ↑ / ↓ ) ")
                    .border_style(Style::default().fg(BORDER)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(18, 35, 33))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        if !session.has_records() {
            frame.render_widget(
                Paragraph::new("No record is selected because the archive is empty.")
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::bordered()
                            .title(" record header ")
                            .border_style(Style::default().fg(BORDER)),
                    ),
                area,
            );
            return;
        }

        let record = session.current_record();
        let meta = session.current_record_meta();
        let text = vec![
            Line::from(vec![
                Span::styled(meta.category_label.clone(), Style::default().fg(CYAN)),
                Span::raw(" / "),
                Span::styled(
                    record.id.clone(),
                    Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" • ", Style::default().fg(AMBER)),
                Span::styled(record.title.clone(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("[{}]", record.badge_label()),
                    badge_style(record).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" • ", Style::default().fg(AMBER)),
                Span::styled(
                    record.access_level.label(),
                    access_style(record.access_level),
                ),
                Span::styled(" • ", Style::default().fg(AMBER)),
                Span::styled(
                    record.media_health().label(),
                    media_style(record.media_health()),
                ),
                Span::styled(" • ", Style::default().fg(AMBER)),
                Span::styled(record.timeline.clone(), Style::default().fg(DIM)),
            ]),
            Line::from(vec![
                Span::styled("source ", Style::default().fg(DIM)),
                Span::styled(record.recovered_source.clone(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("subtitle ", Style::default().fg(DIM)),
                Span::styled(record.subtitle.clone(), Style::default().fg(DIM)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" record header ")
                    .border_style(Style::default().fg(BORDER)),
            ),
            area,
        );
    }

    fn render_tab_strip(&self, frame: &mut Frame, area: Rect) {
        let active_tab = self.session.borrow().active_page;
        let tabs = RecordPageTab::ALL
            .into_iter()
            .flat_map(|tab| {
                [
                    Span::styled(
                        format!(" {} ", tab.label()),
                        if tab == active_tab {
                            Style::default()
                                .fg(Color::Black)
                                .bg(CYAN)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(DIM)
                        },
                    ),
                    Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(Line::from(tabs)).block(
                Block::bordered()
                    .title(" page tabs ( ← / → ) ")
                    .border_style(Style::default().fg(BORDER)),
            ),
            area,
        );
    }

    fn render_page(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        if !session.has_records() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        "No recovered record surface is mounted.",
                        Style::default().fg(AMBER),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "The workstation is active, but this build does not expose evidence objects.",
                        Style::default().fg(DIM),
                    )),
                ])
                .wrap(Wrap { trim: false })
                .block(
                    Block::bordered()
                        .title(" record surface ")
                        .border_style(Style::default().fg(BORDER)),
                ),
                area,
            );
            return;
        }

        let record = session.current_record();
        if session.active_page == RecordPageTab::Media {
            self.render_media_page(frame, area, &session, record);
            return;
        }

        let lines = match session.active_page {
            RecordPageTab::Overview => build_overview_lines(record),
            RecordPageTab::Dossier => build_dossier_lines(record),
            RecordPageTab::Timeline => build_timeline_lines(record),
            RecordPageTab::Notes => build_notes_lines(record),
            RecordPageTab::Media => unreachable!("media page is handled separately"),
        };

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((session.detail_scroll as u16, 0))
                .style(Style::default().bg(BG))
                .block(
                    Block::bordered()
                        .title(format!(" {} ", session.active_page.label()))
                        .border_style(Style::default().fg(BORDER)),
                ),
            area,
        );
    }

    fn render_media_page(
        &self,
        frame: &mut Frame,
        area: Rect,
        session: &SessionModel,
        record: &RecordDocument,
    ) {
        if record.media_health() == MediaHealth::Corrupted {
            frame.render_widget(
                Paragraph::new(build_media_lines(record, session, area.width, area.height))
                    .wrap(Wrap { trim: false })
                    .style(Style::default().bg(BG))
                    .block(
                        Block::bordered()
                            .title(" media ")
                            .border_style(Style::default().fg(BORDER)),
                    ),
                area,
            );
            return;
        }

        let text_height = area.height.saturating_sub(10).clamp(11, 14);
        let sections =
            Layout::vertical([Constraint::Length(text_height), Constraint::Min(8)]).split(area);
        let detail_lines = build_media_detail_lines(record);

        frame.render_widget(
            Paragraph::new(detail_lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(BG).fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" media artifact ")
                        .border_style(Style::default().fg(BORDER)),
                ),
            sections[0],
        );

        let visual_block = Block::bordered()
            .title(record.visualizer().map_or_else(
                || " waveform fallback ".to_string(),
                |config| format!(" visual // {} ", config.mode.label()),
            ))
            .border_style(Style::default().fg(BORDER));
        let visual_inner = visual_block.inner(sections[1]);
        frame.render_widget(visual_block, sections[1]);

        let preview_visualizer = self.visualizer_editor.preview_visualizer(record);
        let preview_playback = self
            .visualizer_editor
            .preview_playback_clock(record, session.playback_clock());
        if let Some(config) = preview_visualizer.as_ref() {
            track_visualizer::render_visualizer(
                frame,
                visual_inner,
                self.visual_layer.clone(),
                self.panel_shader_visualizer.clone(),
                Rc::clone(&self.visual_runtime),
                &record.id,
                config,
                session.analysis_snapshot(),
                preview_playback,
                session.viewer_tick,
            );
        } else {
            self.render_waveform_fallback(frame, visual_inner, record, session);
        }
    }

    fn render_waveform_fallback(
        &self,
        frame: &mut Frame,
        area: Rect,
        record: &RecordDocument,
        session: &SessionModel,
    ) {
        let width = area.width.saturating_sub(4).max(8) as usize;
        let height = area.height.saturating_sub(3).max(4) as usize;
        let lines = match record.media_page.waveform_mode {
            WaveformMode::Spectrum => build_music_lines(width, height, session.viewer_tick),
            _ => build_waveform_lines(width, height, session.viewer_tick),
        }
        .into_iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(MAGENTA))))
        .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(BG)),
            area,
        );
    }

    fn render_metadata(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        if !session.has_records() {
            frame.render_widget(
                Paragraph::new("No selected record metadata is available.")
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::bordered()
                            .title(" metadata ")
                            .border_style(Style::default().fg(BORDER)),
                    ),
                area,
            );
            return;
        }

        let meta = session.current_record_meta();
        let record = session.current_record();
        let lines = vec![
            Line::from(vec![
                Span::styled("category ", Style::default().fg(DIM)),
                Span::styled(meta.category_label.clone(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("path ", Style::default().fg(DIM)),
                Span::styled(meta.category_path.clone(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("shelf ", Style::default().fg(DIM)),
                Span::styled(meta.category_description.clone(), Style::default().fg(DIM)),
            ]),
            Line::from(vec![
                Span::styled("status ", Style::default().fg(DIM)),
                Span::styled(record.status_label.clone(), Style::default().fg(GREEN)),
            ]),
            Line::from(vec![
                Span::styled("access ", Style::default().fg(DIM)),
                Span::styled(
                    record.access_level.label(),
                    access_style(record.access_level),
                ),
            ]),
            Line::from(vec![
                Span::styled("media ", Style::default().fg(DIM)),
                Span::styled(
                    record.media_health().label(),
                    media_style(record.media_health()),
                ),
            ]),
            Line::from(vec![
                Span::styled("integrity ", Style::default().fg(DIM)),
                Span::styled(record.signal_integrity.clone(), Style::default().fg(GREEN)),
            ]),
            Line::from(vec![
                Span::styled("kind ", Style::default().fg(DIM)),
                Span::styled(record.kind.label(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("pages ", Style::default().fg(DIM)),
                Span::styled(record.page_count().to_string(), Style::default().fg(WHITE)),
            ]),
            Line::from(vec![
                Span::styled("vector ", Style::default().fg(DIM)),
                Span::styled(record.collapse_vector.clone(), Style::default().fg(TEXT)),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" metadata ")
                        .border_style(Style::default().fg(BORDER)),
                ),
            area,
        );
    }

    fn render_playback(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        if !session.has_records() {
            frame.render_widget(
                Paragraph::new("audio transport unavailable")
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::bordered()
                            .title(" playback ")
                            .border_style(Style::default().fg(BORDER)),
                    ),
                area,
            );
            return;
        }

        let record = session.current_record();
        let playback = session.playback_view();
        let mut lines = vec![
            Line::from(Span::styled(
                playback.state_label,
                Style::default().fg(if playback.is_playing { GREEN } else { AMBER }),
            )),
            Line::from(format!(
                "audio_source: {}",
                playback.source.as_deref().unwrap_or("none")
            )),
            Line::from(playback.detail_label),
            Line::from(
                playback
                    .progress_label
                    .unwrap_or_else(|| "progress: --:-- / --:--".to_string()),
            ),
            Line::from(playback.progress_ratio.map_or_else(
                || "reactivity: idle baseline".to_string(),
                |ratio| format!("reactivity: {:>3.0}% progress", ratio * 100.0),
            )),
            Line::from(match (playback.elapsed_secs, playback.duration_secs) {
                (Some(elapsed), Some(duration)) => {
                    format!("clock: {:>5.1}s / {:>5.1}s", elapsed, duration)
                }
                _ => "clock: awaiting duration lock".to_string(),
            }),
            Line::from(if record.visualizer().is_some() {
                "visual: reactive procedural scene"
            } else if record.media_health() == MediaHealth::Mounted {
                "visual: waveform fallback"
            } else {
                "visual: unavailable"
            }),
            Line::from(if playback.is_actionable {
                "control: press P to play/pause"
            } else {
                "control: media surface corrupted"
            }),
        ];

        if let Some(config) = record.visualizer() {
            lines.insert(3, Line::from(format!("mode: {}", config.mode.label())));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(TEXT))
                .block(
                    Block::bordered()
                        .title(" playback ")
                        .border_style(Style::default().fg(BORDER)),
                ),
            area,
        );
    }

    fn render_logs(&self, frame: &mut Frame, area: Rect) {
        let session = self.session.borrow();
        let visible = area.height.saturating_sub(2) as usize;
        let max_width = area.width.saturating_sub(2) as usize;
        let skip = session.log_lines.len().saturating_sub(visible);
        let lines = session
            .log_lines
            .iter()
            .skip(skip)
            .map(|line| {
                let text = truncate_log_line(line.text.as_str(), max_width);
                Line::from(Span::styled(text, log_style(line.color_role)))
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(" logs ")
                    .border_style(Style::default().fg(BORDER)),
            ),
            area,
        );
    }
}

fn truncate_log_line(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let text_len = text.chars().count();
    if text_len <= max_width {
        return text.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let keep = max_width - 3;
    let truncated = text.chars().take(keep).collect::<String>();
    format!("{truncated}...")
}

fn build_overview_lines(record: &RecordDocument) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            record.overview.summary.clone(),
            Style::default().fg(TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "thesis ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(record.overview.thesis.clone(), Style::default().fg(TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("status ", Style::default().fg(CYAN)),
            Span::styled(
                record.overview.status_callout.clone(),
                Style::default().fg(AMBER),
            ),
        ]),
        Line::from(vec![
            Span::styled("warning ", Style::default().fg(CYAN)),
            Span::styled(
                record.overview.content_warning.clone(),
                Style::default().fg(DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("hint ", Style::default().fg(CYAN)),
            Span::styled(
                record.overview.viewer_hint.clone(),
                Style::default().fg(TEXT),
            ),
        ]),
    ]
}

fn build_dossier_lines(record: &RecordDocument) -> Vec<Line<'static>> {
    if record.access_level != AccessLevel::Readable {
        return restricted_lines(record, "dossier");
    }

    let mut lines = Vec::new();
    for section in &record.dossier.sections {
        lines.push(Line::from(Span::styled(
            format!("◆ {}", section.title),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )));
        if let Some(emphasis) = &section.emphasis {
            lines.push(Line::from(Span::styled(
                emphasis.clone(),
                Style::default().fg(AMBER),
            )));
        }
        for body_line in section.body.lines() {
            lines.push(Line::from(Span::styled(
                body_line.to_string(),
                Style::default().fg(TEXT),
            )));
        }
        lines.push(Line::from(""));
    }
    lines
}

fn build_timeline_lines(record: &RecordDocument) -> Vec<Line<'static>> {
    if record.access_level != AccessLevel::Readable {
        return restricted_lines(record, "timeline");
    }

    let mut lines = Vec::new();
    for event in &record.timeline_page.events {
        lines.push(Line::from(vec![
            Span::styled(
                format!("● {}", event.label),
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled(event.timestamp_text.clone(), Style::default().fg(AMBER)),
        ]));
        lines.push(Line::from(Span::styled(
            event.body.clone(),
            Style::default().fg(TEXT),
        )));
        lines.push(Line::from(""));
    }
    lines
}

fn build_notes_lines(record: &RecordDocument) -> Vec<Line<'static>> {
    if record.access_level != AccessLevel::Readable {
        return restricted_lines(record, "notes");
    }

    let mut lines = Vec::new();
    for note in &record.notes_page.tty0_annotation {
        lines.push(Line::from(vec![
            Span::styled("tty0@home:$ ", Style::default().fg(CYAN)),
            Span::styled(note.clone(), Style::default().fg(DIM)),
        ]));
        lines.push(Line::from(""));
    }
    if !lines.is_empty() {
        lines.pop();
    }
    lines
}

fn build_media_detail_lines(record: &RecordDocument) -> Vec<Line<'static>> {
    let audio = record.media_page.audio.as_ref().expect("mounted audio");
    let mut lines = vec![
        Line::from(Span::styled(
            "Mounted transport",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("title ", Style::default().fg(DIM)),
            Span::styled(audio.title.clone(), Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("duration ", Style::default().fg(DIM)),
            Span::styled(audio.duration_hint.clone(), Style::default().fg(AMBER)),
        ]),
        Line::from(vec![
            Span::styled("path ", Style::default().fg(DIM)),
            Span::styled(audio.path.clone(), Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("mode ", Style::default().fg(DIM)),
            Span::styled(
                record.visualizer().map_or_else(
                    || record.media_page.waveform_mode.label(),
                    |config| config.mode.label(),
                ),
                Style::default().fg(AMBER),
            ),
        ]),
        Line::from(Span::styled(
            record.media_page.artifact_note.clone(),
            Style::default().fg(TEXT),
        )),
        Line::from(vec![
            Span::styled("excerpt ", Style::default().fg(CYAN)),
            Span::styled(
                record.media_page.transcript_excerpt.clone(),
                Style::default().fg(DIM),
            ),
        ]),
    ];

    if let Some(config) = record.visualizer() {
        lines.push(Line::from(vec![
            Span::styled("params ", Style::default().fg(DIM)),
            Span::styled(
                format!(
                    "rings {} • particles {} • lattice {}",
                    config.params.ring_count,
                    config.params.particle_count,
                    config.params.lattice_density
                ),
                Style::default().fg(TEXT),
            ),
        ]));
    }

    lines
}

fn build_media_lines(
    record: &RecordDocument,
    session: &SessionModel,
    width: u16,
    height: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match record.media_health() {
        MediaHealth::Mounted => {
            lines.extend(build_media_detail_lines(record));
            lines.push(Line::from(""));
            let visual = match record.media_page.waveform_mode {
                WaveformMode::Spectrum => build_music_lines(
                    width.saturating_sub(4).max(8) as usize,
                    height.saturating_sub(12).max(4) as usize,
                    session.viewer_tick,
                ),
                _ => build_waveform_lines(
                    width.saturating_sub(4).max(8) as usize,
                    height.saturating_sub(12).max(4) as usize,
                    session.viewer_tick,
                ),
            };
            for line in visual {
                lines.push(Line::from(Span::styled(line, Style::default().fg(MAGENTA))));
            }
        }
        MediaHealth::Corrupted => {
            lines.push(Line::from(Span::styled(
                "CORRUPTED",
                Style::default().fg(RED).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                record
                    .media_page
                    .corruption_reason
                    .clone()
                    .unwrap_or_else(|| "transport surface missing".to_string()),
                Style::default().fg(TEXT),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("artifact ", Style::default().fg(CYAN)),
                Span::styled(
                    record.media_page.artifact_note.clone(),
                    Style::default().fg(DIM),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("excerpt ", Style::default().fg(CYAN)),
                Span::styled(
                    record.media_page.transcript_excerpt.clone(),
                    Style::default().fg(DIM),
                ),
            ]));
        }
    }

    lines
}

fn restricted_lines(record: &RecordDocument, label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "RESTRICTED",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "{} surface withheld for {}",
                label,
                record.access_level.label()
            ),
            Style::default().fg(TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("record ", Style::default().fg(CYAN)),
            Span::styled(record.id.clone(), Style::default().fg(AMBER)),
        ]),
        Line::from(vec![
            Span::styled("status ", Style::default().fg(CYAN)),
            Span::styled(record.status_label.clone(), Style::default().fg(DIM)),
        ]),
        Line::from(vec![
            Span::styled("surface ", Style::default().fg(CYAN)),
            Span::styled(
                "index remains readable even when content is sealed",
                Style::default().fg(DIM),
            ),
        ]),
    ]
}

fn build_waveform_lines(width: usize, height: usize, tick: u64) -> Vec<String> {
    (0..height)
        .map(|row| {
            (0..width)
                .map(|col| {
                    let phase = ((col as u64 * 3 + tick / 40) % 20) as i32;
                    let center = (height as i32 / 2) + (phase - 10) / 4;
                    if (row as i32 - center).abs() <= 1 {
                        '█'
                    } else if (row as i32 - center).abs() == 2 {
                        '▓'
                    } else {
                        ' '
                    }
                })
                .collect()
        })
        .collect()
}

fn build_music_lines(width: usize, height: usize, tick: u64) -> Vec<String> {
    (0..height)
        .map(|row| {
            (0..width)
                .map(|col| {
                    let band = (((col as u64 / 3) + tick / 90) % height as u64) as usize;
                    if height - row <= band + 1 {
                        '▇'
                    } else {
                        ' '
                    }
                })
                .collect()
        })
        .collect()
}

fn badge_style(record: &RecordDocument) -> Style {
    match record.badge_label() {
        "AUD" => Style::default().fg(CYAN),
        "COR" => Style::default().fg(RED),
        "IDX" => Style::default().fg(DIM),
        "LOCK" => Style::default().fg(RED),
        _ => Style::default().fg(TEXT),
    }
}

fn accent_style(record: &RecordDocument) -> Style {
    match record.media_health() {
        MediaHealth::Mounted => Style::default().fg(CYAN),
        MediaHealth::Corrupted => Style::default().fg(MAGENTA),
    }
}

fn tree_icon(record: &RecordDocument) -> &'static str {
    match record.badge_label() {
        "AUD" => "◉",
        "COR" => "◌",
        "IDX" => "◌",
        "LOCK" => "⊘",
        _ => "•",
    }
}

fn access_style(access_level: AccessLevel) -> Style {
    match access_level {
        AccessLevel::Readable => Style::default().fg(GREEN),
        AccessLevel::IndexOnly => Style::default().fg(AMBER),
        AccessLevel::Sealed => Style::default().fg(RED),
    }
}

fn media_style(media_health: MediaHealth) -> Style {
    match media_health {
        MediaHealth::Mounted => Style::default().fg(CYAN),
        MediaHealth::Corrupted => Style::default().fg(RED),
    }
}

fn log_style(role: LogColorRole) -> Style {
    match role {
        LogColorRole::Info => Style::default().fg(DIM),
        LogColorRole::Sync => Style::default().fg(CYAN),
        LogColorRole::Warn => Style::default().fg(AMBER),
        LogColorRole::Deny => Style::default().fg(RED),
        LogColorRole::Ghost => Style::default().fg(MAGENTA),
    }
}
