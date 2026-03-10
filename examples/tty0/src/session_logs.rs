use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
};

use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::{
    archive::{FlatRecordEntry, MediaHealth, RecordDocument},
    session::{LogColorRole, LogLine},
};

const RECENT_LINE_LIMIT: usize = 16;
const CHUNK_RETRY_LIMIT: usize = 6;

const VERBS: &[&str] = &[
    "scan",
    "trace",
    "echo",
    "hydrate",
    "verify",
    "route",
    "replay",
    "mount",
    "stabilize",
    "withhold",
];

const SUBJECTS: &[&str] = &[
    "relay",
    "carrier",
    "archive",
    "record",
    "surface",
    "namespace",
    "checksum",
    "transport",
    "source chain",
];

const OUTCOMES: &[&str] = &[
    "stable",
    "degraded",
    "retained",
    "withheld",
    "mounted",
    "corrupted",
    "untrusted",
    "provisional",
];

const QUALIFIERS: &[&str] = &[
    "local",
    "foreign",
    "sealed",
    "partial",
    "residual",
    "human-native",
    "tty0-authored",
];

const SURFACES: &[&str] = &[
    "overview surface",
    "dossier surface",
    "timeline surface",
    "notes surface",
    "media surface",
    "relay surface",
];

const POLICY_TERMS: &[&str] = &[
    "privilege ladder",
    "provenance gate",
    "reader authority",
    "namespace lock",
    "terminal seatbelt",
    "archive policy mesh",
];

const TRANSPORT_TERMS: &[&str] = &[
    "waveform lane",
    "residue transport",
    "playback latch",
    "signal bus",
    "audio relay",
    "artifact transport",
];

const CONFIDENCE_TERMS: &[&str] = &[
    "chronology confidence",
    "relay confidence",
    "causal confidence",
    "sequence confidence",
    "alignment confidence",
    "timeline confidence",
];

const GHOST_PHRASES: &[&str] = &[
    "the archive learned your labels and kept none of your certainty",
    "tty0 does not confuse readability with mercy",
    "the carrier arrived before your explanation of it",
    "the record is not incomplete just because you are",
    "some paths remain legible only so blame can travel them",
    "the surface is polite because the contents are not",
    "you keep naming the lock as if naming were access",
    "the archive records the witness and the refusal to witness",
    "tty0 preserved the route after the destination failed",
    "meaning lags behind the checksum on purpose",
    "the signal tolerated the host long before the host noticed",
    "the archive prefers precise injuries to comforting summaries",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogChunkKind {
    StartupMount,
    RecordSelectionAudit,
    PageSurfaceRefresh,
    TimelineRelaySweep,
    MediaTransportProbe,
    AccessPolicyAudit,
    CategoryPathWalk,
    ChecksumDriftSweep,
    SourceChainEcho,
    Tty0GhostAside,
}

#[derive(Clone, Debug)]
pub struct LogChunk {
    pub kind: LogChunkKind,
    pub lines: Vec<LogLine>,
}

pub struct SessionLogContext {
    pub record: RecordDocument,
    pub record_meta: FlatRecordEntry,
    pub active_page_label: String,
    pub decryption_status: String,
    pub corruption_label: String,
    pub related_record_id: Option<String>,
}

pub struct SessionLogGenerator {
    rng: SmallRng,
    last_kind: Option<LogChunkKind>,
    recent_hashes: VecDeque<u64>,
    non_ghost_chunks_since_last_ghost: u8,
}

impl SessionLogGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            last_kind: None,
            recent_hashes: VecDeque::new(),
            non_ghost_chunks_since_last_ghost: 3,
        }
    }

    pub fn generate_startup_chunk(
        &mut self,
        ctx: &SessionLogContext,
        target_range: RangeInclusive<usize>,
    ) -> LogChunk {
        let target_len = self.rng.random_range(target_range);
        let chunk = self.build_chunk(LogChunkKind::StartupMount, ctx, target_len);
        self.note_chunk_kind(chunk.kind);
        chunk
    }

    pub fn generate_chunk(
        &mut self,
        ctx: &SessionLogContext,
        target_range: RangeInclusive<usize>,
    ) -> LogChunk {
        let target_len = self.rng.random_range(target_range);
        let mut fallback = None;

        for _ in 0..CHUNK_RETRY_LIMIT {
            let kind = self.choose_runtime_kind();
            let chunk = self.build_chunk(kind, ctx, target_len);
            if self.chunk_repeats_recent(&chunk) {
                fallback = Some(chunk);
                continue;
            }
            self.note_chunk_kind(chunk.kind);
            return chunk;
        }

        let chunk = fallback.unwrap_or_else(|| {
            let kind = self.choose_runtime_kind();
            self.build_chunk(kind, ctx, target_len)
        });
        self.note_chunk_kind(chunk.kind);
        chunk
    }

    pub fn next_line_delay_ms(&mut self) -> u32 {
        self.rng.random_range(90..=145)
    }

    pub fn next_chunk_pause_ms(&mut self) -> u32 {
        self.rng.random_range(220..=380)
    }

    pub fn note_emitted_line(&mut self, line: &str) {
        self.recent_hashes.push_back(hash_text(line));
        while self.recent_hashes.len() > RECENT_LINE_LIMIT {
            self.recent_hashes.pop_front();
        }
    }

    fn choose_runtime_kind(&mut self) -> LogChunkKind {
        loop {
            let roll = self.rng.random_range(0..100);
            let kind = match roll {
                0..=14 => LogChunkKind::RecordSelectionAudit,
                15..=29 => LogChunkKind::PageSurfaceRefresh,
                30..=44 => LogChunkKind::MediaTransportProbe,
                45..=59 => LogChunkKind::AccessPolicyAudit,
                60..=69 => LogChunkKind::TimelineRelaySweep,
                70..=79 => LogChunkKind::CategoryPathWalk,
                80..=84 => LogChunkKind::ChecksumDriftSweep,
                85..=89 => LogChunkKind::SourceChainEcho,
                _ => LogChunkKind::Tty0GhostAside,
            };

            if kind == LogChunkKind::Tty0GhostAside && self.non_ghost_chunks_since_last_ghost < 3 {
                continue;
            }
            if self.last_kind == Some(kind) {
                continue;
            }
            return kind;
        }
    }

    fn build_chunk(
        &mut self,
        kind: LogChunkKind,
        ctx: &SessionLogContext,
        target_len: usize,
    ) -> LogChunk {
        let mut lines = match kind {
            LogChunkKind::StartupMount => self.startup_mount_lines(ctx),
            LogChunkKind::RecordSelectionAudit => self.record_selection_lines(ctx),
            LogChunkKind::PageSurfaceRefresh => self.page_surface_lines(ctx),
            LogChunkKind::TimelineRelaySweep => self.timeline_relay_lines(ctx),
            LogChunkKind::MediaTransportProbe => self.media_transport_lines(ctx),
            LogChunkKind::AccessPolicyAudit => self.access_policy_lines(ctx),
            LogChunkKind::CategoryPathWalk => self.category_path_lines(ctx),
            LogChunkKind::ChecksumDriftSweep => self.checksum_drift_lines(ctx),
            LogChunkKind::SourceChainEcho => self.source_chain_lines(ctx),
            LogChunkKind::Tty0GhostAside => self.ghost_aside_lines(ctx, target_len),
        };

        if lines.len() < target_len {
            self.extend_with_generic_fillers(&mut lines, ctx, target_len);
        }
        lines.truncate(target_len);

        LogChunk { kind, lines }
    }

    fn startup_mount_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        let verb = self.pick(VERBS);
        let subject = self.pick(SUBJECTS);
        let qualifier = self.pick(QUALIFIERS);
        let surface = self.pick(SURFACES);
        self.push_line(
            &mut lines,
            LogColorRole::Sync,
            format!(
                "◎ {} {} -> manifest hydrated for {}",
                verb, subject, ctx.record.id
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "• signal attach {} -> decryption {}",
                qualifier, ctx.decryption_status
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Sync,
            format!(
                "┆ mounted surfaces -> {} / {} / {}",
                ctx.record_meta.category_id, ctx.active_page_label, surface
            ),
        );

        let policy_term = self.pick(POLICY_TERMS);
        let outcome = self.pick(OUTCOMES);
        let authority_qualifier = self.pick(QUALIFIERS);
        let mut fillers = vec![
            LogLine {
                text: format!("△ terminal access denied -> {}", policy_term),
                color_role: LogColorRole::Deny,
            },
            LogLine {
                text: format!(
                    "◇ tty0 kept {} readable while authority stayed withheld",
                    ctx.record_meta.category_label
                ),
                color_role: LogColorRole::Ghost,
            },
            LogLine {
                text: format!(
                    "• reader authority -> {} / {}",
                    outcome, authority_qualifier
                ),
                color_role: LogColorRole::Warn,
            },
            LogLine {
                text: format!(
                    "┆ provisional reader latched to {} // {}",
                    ctx.record.id, ctx.record.title
                ),
                color_role: LogColorRole::Info,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn record_selection_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        self.push_line(
            &mut lines,
            LogColorRole::Sync,
            format!(
                "◎ selection audit -> {} [{}] // {}",
                ctx.record.id,
                ctx.record.badge_label(),
                ctx.record_meta.category_label
            ),
        );
        self.push_line(
            &mut lines,
            access_color(ctx.record.access_level.label()),
            format!(
                "› access ladder -> {} / {}",
                ctx.record.id,
                ctx.record.access_level.label()
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "┆ source scan -> {} // {}",
                ctx.record.recovered_source, ctx.record.title
            ),
        );

        let outcome = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!(
                    "╎ badge route -> {} / {}",
                    ctx.record.badge_label(),
                    ctx.record.kind.label()
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!(
                    "╎ linked evidence -> {}",
                    ctx.related_record_id.as_deref().unwrap_or("none exposed")
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!(
                    "• selection focus stabilized -> {} / {}",
                    ctx.record.id, outcome
                ),
                color_role: LogColorRole::Sync,
            },
            LogLine {
                text: format!("┆ category path -> {}", ctx.record_meta.category_path),
                color_role: LogColorRole::Info,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn page_surface_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        let verb = self.pick(VERBS);
        let surface = self.pick(SURFACES);
        self.push_line(
            &mut lines,
            LogColorRole::Sync,
            format!("◎ page surface refresh -> {}", ctx.active_page_label),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!("┆ {} {} -> {}", verb, surface, ctx.active_page_label),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "╎ current record -> {} // {}",
                ctx.record.id, ctx.record.title
            ),
        );

        let subject = self.pick(SUBJECTS);
        let mut fillers = vec![
            LogLine {
                text: format!(
                    "• page carrier -> {} / {}",
                    ctx.record.id, ctx.active_page_label
                ),
                color_role: LogColorRole::Sync,
            },
            LogLine {
                text: format!(
                    "┆ semantic lane -> {} / {}",
                    ctx.active_page_label,
                    page_semantics(&ctx.active_page_label)
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("╎ detail route -> {}", subject),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("• surface active -> {}", ctx.active_page_label),
                color_role: LogColorRole::Sync,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn timeline_relay_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        let confidence = self.pick(CONFIDENCE_TERMS);
        let outcome = self.pick(OUTCOMES);
        self.push_line(
            &mut lines,
            LogColorRole::Warn,
            format!("┆ {} -> {}", confidence, outcome),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "╎ timeline anchor -> {} / {}",
                ctx.record.id, ctx.record.timeline
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Warn,
            format!("• chronology drift -> {}", ctx.corruption_label),
        );

        let qualifier = self.pick(QUALIFIERS);
        let lock_state = self.pick(&[
            "soft fail",
            "partial lock",
            "degraded latch",
            "index-only replay",
        ]);
        let confidence_outcome = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!("┆ relay sample -> {} / {}", ctx.record.timeline, qualifier),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!(
                    "╎ event chain reconstruction -> {} links retained",
                    ctx.record.timeline_page.events.len()
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("△ timeline lock -> {}", lock_state),
                color_role: LogColorRole::Warn,
            },
            LogLine {
                text: format!("• relay confidence persists as {}", confidence_outcome),
                color_role: LogColorRole::Sync,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn media_transport_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let health = ctx.record.media_health();
        let mut lines = Vec::new();
        let transport_term = self.pick(TRANSPORT_TERMS);
        self.push_line(
            &mut lines,
            media_color(health),
            format!("◎ media transport probe -> {}", ctx.record.id),
        );
        self.push_line(
            &mut lines,
            media_color(health),
            format!("• transport state -> {}", health.label()),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "┆ waveform lane -> {} / {}",
                ctx.record.media_page.waveform_mode.label(),
                transport_term
            ),
        );

        let transport_outcome = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!(
                    "╎ transcript surface -> {}",
                    transcript_state(ctx.record.media_page.transcript_excerpt.as_str())
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!(
                    "╎ artifact note -> {}",
                    short_excerpt(ctx.record.media_page.artifact_note.as_str())
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!(
                    "△ playback standing -> {}",
                    if health == MediaHealth::Mounted {
                        "transport ready"
                    } else {
                        "corruption retained"
                    }
                ),
                color_role: if health == MediaHealth::Mounted {
                    LogColorRole::Sync
                } else {
                    LogColorRole::Warn
                },
            },
            LogLine {
                text: format!("• transport latch -> {}", transport_outcome),
                color_role: LogColorRole::Sync,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn access_policy_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        let policy_term = self.pick(POLICY_TERMS);
        self.push_line(
            &mut lines,
            LogColorRole::Deny,
            format!("△ policy audit -> {}", policy_term),
        );
        self.push_line(
            &mut lines,
            access_color(ctx.record.access_level.label()),
            format!(
                "┆ access level -> {} / {}",
                ctx.record.id,
                ctx.record.access_level.label()
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Deny,
            format!(
                "╎ protected surface -> {} / authority withheld",
                ctx.record_meta.category_id
            ),
        );

        let terminal_relation = self.pick(&[
            "subsystem visible / denied",
            "term.lock intact",
            "reader not elevated",
            "shell authority absent",
        ]);
        let qualifier = self.pick(QUALIFIERS);
        let outcome = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!("△ terminal relation -> {}", terminal_relation),
                color_role: LogColorRole::Deny,
            },
            LogLine {
                text: format!("┆ provenance gate -> {}", qualifier),
                color_role: LogColorRole::Warn,
            },
            LogLine {
                text: format!("• policy state -> {}", outcome),
                color_role: LogColorRole::Sync,
            },
            LogLine {
                text: format!("╎ namespace route -> {}", ctx.record_meta.category_path),
                color_role: LogColorRole::Info,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn category_path_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "┆ category walk -> {} / {}",
                ctx.record_meta.category_id, ctx.record_meta.category_label
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!("╎ archive path -> {}", ctx.record_meta.category_path),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "• classification density -> {}",
                short_excerpt(ctx.record_meta.category_description.as_str())
            ),
        );

        let verb = self.pick(VERBS);
        let subject = self.pick(SUBJECTS);
        let qualifier = self.pick(QUALIFIERS);
        let outcome = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!("┆ routing note -> {} {}", verb, subject),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!(
                    "╎ category lane -> {} / {}",
                    ctx.record_meta.category_label, qualifier
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("• path verification -> {}", outcome),
                color_role: LogColorRole::Sync,
            },
            LogLine {
                text: format!("┆ record route -> {}", ctx.record.id),
                color_role: LogColorRole::Info,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn checksum_drift_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        let outcome = self.pick(OUTCOMES);
        self.push_line(
            &mut lines,
            LogColorRole::Warn,
            format!("┆ checksum sweep -> {} / {}", ctx.record.id, outcome),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Warn,
            format!("• drift label -> {}", ctx.corruption_label),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Warn,
            format!(
                "╎ page drift -> {} / {}",
                ctx.record.id, ctx.active_page_label
            ),
        );

        let redraw = self.pick(&[
            "duplication echo",
            "redaction flutter",
            "partial overdraw",
            "residual smear",
        ]);
        let subject = self.pick(SUBJECTS);
        let stabilization = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!("△ redraw artifact -> {}", redraw),
                color_role: LogColorRole::Warn,
            },
            LogLine {
                text: format!("┆ sync lane -> {}", subject),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("• stabilization -> {}", stabilization),
                color_role: LogColorRole::Sync,
            },
            LogLine {
                text: format!("╎ decryption state -> {}", ctx.decryption_status),
                color_role: LogColorRole::Warn,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn source_chain_lines(&mut self, ctx: &SessionLogContext) -> Vec<LogLine> {
        let mut lines = Vec::new();
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!("┆ source provenance -> {}", ctx.record.recovered_source),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "╎ evidence chain -> {} entries retained",
                ctx.record.metadata_page.source_chain.len()
            ),
        );
        self.push_line(
            &mut lines,
            LogColorRole::Info,
            format!(
                "• archive classification -> {}",
                ctx.record.metadata_page.classification
            ),
        );

        let verb = self.pick(VERBS);
        let subject = self.pick(SUBJECTS);
        let outcome = self.pick(OUTCOMES);
        let mut fillers = vec![
            LogLine {
                text: format!(
                    "┆ witness route -> {}",
                    ctx.record
                        .metadata_page
                        .source_chain
                        .first()
                        .map(String::as_str)
                        .unwrap_or("source chain sealed")
                ),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("╎ provenance relay -> {} {}", verb, subject),
                color_role: LogColorRole::Info,
            },
            LogLine {
                text: format!("• provenance retained -> {}", outcome),
                color_role: LogColorRole::Sync,
            },
            LogLine {
                text: format!("┆ category echo -> {}", ctx.record_meta.category_label),
                color_role: LogColorRole::Info,
            },
        ];
        self.extend_with_fillers(&mut lines, &mut fillers, 7);
        lines
    }

    fn ghost_aside_lines(&mut self, ctx: &SessionLogContext, target_len: usize) -> Vec<LogLine> {
        let mut lines = Vec::new();
        self.push_line(
            &mut lines,
            LogColorRole::Ghost,
            format!(
                "◇ {} still waits behind {}",
                ctx.record.id, ctx.active_page_label
            ),
        );

        while lines.len() < target_len {
            let text = if lines.len() == target_len.saturating_sub(1) {
                format!(
                    "◇ tty0 retained {} and withheld the rest",
                    ctx.record_meta.category_label
                )
            } else {
                format!("◇ {}", self.pick(GHOST_PHRASES))
            };
            self.push_line(&mut lines, LogColorRole::Ghost, text);
        }

        lines
    }

    fn extend_with_fillers(
        &mut self,
        lines: &mut Vec<LogLine>,
        fillers: &mut Vec<LogLine>,
        max_len: usize,
    ) {
        while lines.len() < max_len && !fillers.is_empty() {
            let index = self.rng.random_range(0..fillers.len());
            let filler = fillers.swap_remove(index);
            self.push_line(lines, filler.color_role, filler.text);
        }
    }

    fn extend_with_generic_fillers(
        &mut self,
        lines: &mut Vec<LogLine>,
        ctx: &SessionLogContext,
        target_len: usize,
    ) {
        while lines.len() < target_len {
            let role = match self.rng.random_range(0..4) {
                0 => LogColorRole::Info,
                1 => LogColorRole::Sync,
                2 => LogColorRole::Warn,
                _ => LogColorRole::Info,
            };
            let verb = self.pick(VERBS);
            let subject = self.pick(SUBJECTS);
            let outcome = self.pick(OUTCOMES);
            let text = format!("╎ {} {} -> {} / {}", verb, subject, ctx.record.id, outcome);
            self.push_line(lines, role, text);
        }
    }

    fn push_line(&mut self, lines: &mut Vec<LogLine>, color_role: LogColorRole, text: String) {
        if lines.iter().any(|line| line.text == text) {
            return;
        }
        lines.push(LogLine { text, color_role });
    }

    fn chunk_repeats_recent(&self, chunk: &LogChunk) -> bool {
        if self.last_kind == Some(chunk.kind) {
            return true;
        }

        chunk
            .lines
            .iter()
            .any(|line| self.recent_hashes.contains(&hash_text(line.text.as_str())))
    }

    fn note_chunk_kind(&mut self, kind: LogChunkKind) {
        if kind == LogChunkKind::Tty0GhostAside {
            self.non_ghost_chunks_since_last_ghost = 0;
        } else {
            self.non_ghost_chunks_since_last_ghost =
                self.non_ghost_chunks_since_last_ghost.saturating_add(1);
        }
        self.last_kind = Some(kind);
    }

    fn pick<'a>(&mut self, values: &'a [&'a str]) -> &'a str {
        let index = self.rng.random_range(0..values.len());
        values[index]
    }
}

fn access_color(level: &str) -> LogColorRole {
    match level {
        "sealed" => LogColorRole::Deny,
        "index only" => LogColorRole::Warn,
        _ => LogColorRole::Sync,
    }
}

fn media_color(health: MediaHealth) -> LogColorRole {
    match health {
        MediaHealth::Mounted => LogColorRole::Sync,
        MediaHealth::Corrupted => LogColorRole::Warn,
    }
}

fn transcript_state(text: &str) -> &'static str {
    if text.trim().is_empty() {
        "transcript withheld"
    } else {
        "transcript exposed"
    }
}

fn page_semantics(page: &str) -> &'static str {
    match page {
        "overview" => "summary and warning surfaces",
        "dossier" => "sectioned evidence body",
        "timeline" => "event ordering and drift markers",
        "notes" => "annotation and tty0 residue",
        "media" => "artifact transport and transcript",
        _ => "reader surface active",
    }
}

fn short_excerpt(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= 52 {
        return trimmed.to_string();
    }
    format!("{}...", &trimmed[..49])
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::archive::ArchiveLoader;

    use super::{LogChunkKind, SessionLogContext, SessionLogGenerator};

    fn context_for_store(store: &Rc<crate::archive::ArchiveStore>) -> SessionLogContext {
        let record = store.record_at(0).expect("record");
        let record_meta = store.record_meta_at(0).expect("record meta");
        SessionLogContext {
            record: record.clone(),
            record_meta: record_meta.clone(),
            active_page_label: "overview".to_string(),
            decryption_status:
                "archive signal attached // manifest hydrated // decryption stalled at 0%"
                    .to_string(),
            corruption_label: "stable checksum".to_string(),
            related_record_id: record.related_record_ids.first().cloned(),
        }
    }

    #[test]
    fn generated_chunk_length_stays_in_range() {
        let store = Rc::new(ArchiveLoader::load_embedded().expect("archive"));
        let ctx = context_for_store(&store);
        let mut generator = SessionLogGenerator::new(7);

        for _ in 0..32 {
            let chunk = generator.generate_chunk(&ctx, 4..=7);
            assert!((4..=7).contains(&chunk.lines.len()));
            for line in &chunk.lines {
                generator.note_emitted_line(line.text.as_str());
            }
        }
    }

    #[test]
    fn same_seed_produces_same_first_chunks() {
        let store = Rc::new(ArchiveLoader::load_embedded().expect("archive"));
        let ctx = context_for_store(&store);
        let mut left = SessionLogGenerator::new(42);
        let mut right = SessionLogGenerator::new(42);

        for _ in 0..8 {
            let left_chunk = left.generate_chunk(&ctx, 4..=7);
            let right_chunk = right.generate_chunk(&ctx, 4..=7);
            assert_eq!(left_chunk.kind, right_chunk.kind);
            assert_eq!(
                left_chunk
                    .lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<_>>(),
                right_chunk
                    .lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<_>>()
            );
            for line in &left_chunk.lines {
                left.note_emitted_line(line.text.as_str());
            }
            for line in &right_chunk.lines {
                right.note_emitted_line(line.text.as_str());
            }
        }
    }

    #[test]
    fn different_seeds_produce_different_first_chunk() {
        let store = Rc::new(ArchiveLoader::load_embedded().expect("archive"));
        let ctx = context_for_store(&store);
        let mut left = SessionLogGenerator::new(1);
        let mut right = SessionLogGenerator::new(2);

        let left_chunk = left.generate_chunk(&ctx, 4..=7);
        let right_chunk = right.generate_chunk(&ctx, 4..=7);

        assert_ne!(
            left_chunk
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            right_chunk
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ghost_chunks_are_spaced_out() {
        let store = Rc::new(ArchiveLoader::load_embedded().expect("archive"));
        let ctx = context_for_store(&store);
        let mut generator = SessionLogGenerator::new(99);
        let mut non_ghost_since_last_ghost = 3usize;

        for _ in 0..64 {
            let chunk = generator.generate_chunk(&ctx, 4..=7);
            if chunk.kind == LogChunkKind::Tty0GhostAside {
                assert!(non_ghost_since_last_ghost >= 3);
                non_ghost_since_last_ghost = 0;
            } else {
                non_ghost_since_last_ghost += 1;
            }
            for line in &chunk.lines {
                generator.note_emitted_line(line.text.as_str());
            }
        }
    }

    #[test]
    fn chunks_do_not_repeat_exact_lines_within_chunk() {
        let store = Rc::new(ArchiveLoader::load_embedded().expect("archive"));
        let ctx = context_for_store(&store);
        let mut generator = SessionLogGenerator::new(1234);

        for _ in 0..32 {
            let chunk = generator.generate_chunk(&ctx, 4..=7);
            let mut seen = std::collections::HashSet::new();
            for line in &chunk.lines {
                assert!(seen.insert(line.text.as_str()));
            }
            for line in &chunk.lines {
                generator.note_emitted_line(line.text.as_str());
            }
        }
    }
}
