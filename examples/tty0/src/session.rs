use std::{collections::VecDeque, rc::Rc};

use crate::{
    archive::{ArchiveStore, FlatRecordEntry, PlaybackClock, RecordDocument},
    audio::{AudioController, PlaybackView},
    session_logs::{SessionLogContext, SessionLogGenerator},
    state::StateId,
    track_visualizer::AudioAnalysisSnapshot,
};
use tachyonfx::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

const LOG_BUFFER_LIMIT: usize = 96;
const LOG_CHUNK_RANGE: std::ops::RangeInclusive<usize> = 4..=7;

#[derive(Clone, Debug)]
pub struct LogLine {
    pub text: String,
    pub color_role: LogColorRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogColorRole {
    Info,
    Sync,
    Warn,
    Deny,
    Ghost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordPageTab {
    Overview,
    Dossier,
    Timeline,
    Notes,
    Media,
}

impl RecordPageTab {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Dossier,
        Self::Timeline,
        Self::Notes,
        Self::Media,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Dossier => "dossier",
            Self::Timeline => "timeline",
            Self::Notes => "notes",
            Self::Media => "media",
        }
    }

    pub fn shift(self, delta: i32) -> Self {
        let current = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(Self::ALL.len() as i32) as usize;
        Self::ALL[next]
    }
}

pub struct SessionModel {
    archive: Rc<ArchiveStore>,
    pub selected_record: usize,
    pub detail_scroll: usize,
    pub active_page: RecordPageTab,
    pub decryption_status: &'static str,
    pub corruption_tick: u64,
    pub corruption_elapsed_ms: u32,
    pub terminal_return_state: StateId,
    pub log_lines: VecDeque<LogLine>,
    pub log_elapsed_ms: u32,
    pub next_log_delay_ms: u32,
    pub viewer_tick: u64,
    log_generator: SessionLogGenerator,
    pending_log_lines: VecDeque<LogLine>,
    audio: AudioController,
}

impl SessionModel {
    pub fn new(archive: Rc<ArchiveStore>) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self::new_with_log_seed(archive, seed)
    }

    fn new_with_log_seed(archive: Rc<ArchiveStore>, seed: u64) -> Self {
        let mut session = Self {
            archive,
            selected_record: 0,
            detail_scroll: 0,
            active_page: RecordPageTab::Overview,
            decryption_status:
                "archive signal attached // manifest hydrated // decryption stalled at 0%",
            corruption_tick: 0,
            corruption_elapsed_ms: 0,
            terminal_return_state: StateId::Archive,
            log_lines: VecDeque::new(),
            log_elapsed_ms: 0,
            next_log_delay_ms: 0,
            viewer_tick: 0,
            log_generator: SessionLogGenerator::new(seed),
            pending_log_lines: VecDeque::new(),
            audio: AudioController::new(),
        };
        session.prime_logs();
        session.next_log_delay_ms = session.log_generator.next_chunk_pause_ms();
        session
    }

    pub fn archive(&self) -> &ArchiveStore {
        self.archive.as_ref()
    }

    pub fn has_records(&self) -> bool {
        self.archive.record_count() > 0
    }

    pub fn current_record(&self) -> &RecordDocument {
        self.archive
            .record_at(self.current_record_index())
            .expect("tty0 archive must expose a selected record")
    }

    pub fn current_record_index(&self) -> usize {
        self.selected_record
    }

    pub fn current_record_meta(&self) -> &FlatRecordEntry {
        self.archive
            .record_meta_at(self.current_record_index())
            .expect("tty0 archive must expose selected record metadata")
    }

    pub fn move_record(&mut self, delta: i32) {
        if !self.has_records() {
            return;
        }
        let max_index = self.archive.record_count().saturating_sub(1) as i32;
        let next = (self.current_record_index() as i32 + delta).clamp(0, max_index) as usize;
        self.selected_record = next;
        self.detail_scroll = 0;
        self.active_page = RecordPageTab::Overview;
        self.audio.stop();
        self.decryption_status = "record focus changed // evidence surfaces refreshed";
    }

    pub fn move_record_page(&mut self, delta: i32) {
        self.active_page = self.active_page.shift(delta);
        self.detail_scroll = 0;
        self.decryption_status = match self.active_page {
            RecordPageTab::Overview => "overview surface active",
            RecordPageTab::Dossier => "dossier surface active",
            RecordPageTab::Timeline => "timeline surface active",
            RecordPageTab::Notes => "notes surface active",
            RecordPageTab::Media => "media surface active",
        };
    }

    pub fn open_terminal_from(&mut self, state: StateId) {
        self.terminal_return_state = state;
        self.decryption_status = "terminal subsystem requested // authority withheld";
    }

    pub fn toggle_playback(&mut self) {
        let record = self.current_record().clone();
        match self.audio.toggle(&record) {
            Ok(true) => {
                self.decryption_status = "audio transport active // evidence playback running";
            }
            Ok(false) => {
                self.decryption_status = "audio transport paused // evidence playback standing by";
            }
            Err(_) => {
                self.decryption_status = "selected media surface is corrupted";
            }
        }
    }

    pub fn playback_view(&self) -> PlaybackView {
        self.audio.view_for(self.current_record())
    }

    pub fn playback_clock(&self) -> PlaybackClock {
        self.audio.playback_clock_for(self.current_record())
    }

    pub fn analysis_snapshot(&self) -> Option<AudioAnalysisSnapshot> {
        self.audio.analysis_snapshot_for(self.current_record())
    }

    pub fn set_playback(&mut self, should_play: bool) {
        let record = self.current_record().clone();
        let result = if should_play {
            self.audio.play(&record)
        } else {
            self.audio.pause()
        };

        self.decryption_status = match (should_play, result) {
            (true, Ok(())) => "audio transport active // evidence playback running",
            (false, Ok(())) => "audio transport paused // evidence playback standing by",
            (_, Err(_)) => "selected media surface is corrupted",
        };
    }

    pub fn seek_to_secs(&mut self, secs: f32) {
        let record = self.current_record().clone();
        self.audio.seek_to_secs(&record, secs);
    }

    pub fn tick(&mut self, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis();
        self.viewer_tick = self.viewer_tick.wrapping_add(elapsed_ms as u64);
        self.audio.sync();

        self.corruption_elapsed_ms += elapsed_ms;
        while self.corruption_elapsed_ms >= 1200 {
            self.corruption_tick = self.corruption_tick.wrapping_add(1);
            self.corruption_elapsed_ms -= 1200;
        }

        self.log_elapsed_ms += elapsed_ms;
        while self.log_elapsed_ms >= self.next_log_delay_ms {
            self.log_elapsed_ms -= self.next_log_delay_ms;

            if self.pending_log_lines.is_empty() {
                self.queue_next_chunk();
                if self.pending_log_lines.is_empty() {
                    break;
                }
                self.next_log_delay_ms = 0;
                continue;
            }

            let Some(line) = self.pending_log_lines.pop_front() else {
                self.next_log_delay_ms = self.log_generator.next_chunk_pause_ms();
                break;
            };

            self.push_log(line);
            self.next_log_delay_ms = if self.pending_log_lines.is_empty() {
                self.log_generator.next_chunk_pause_ms()
            } else {
                self.log_generator.next_line_delay_ms()
            };
        }
    }

    pub fn corruption_label(&self) -> &'static str {
        match self.corruption_tick % 4 {
            0 => "stable checksum",
            1 => "minor drift",
            2 => "duplication artifact",
            _ => "redaction flutter",
        }
    }

    fn build_log_context(&self) -> SessionLogContext {
        let record = self.current_record();
        let record_meta = self.current_record_meta();
        SessionLogContext {
            record: record.clone(),
            record_meta: record_meta.clone(),
            active_page_label: self.active_page.label().to_string(),
            decryption_status: self.decryption_status.to_string(),
            corruption_label: self.corruption_label().to_string(),
            related_record_id: record.related_record_ids.first().cloned(),
        }
    }

    fn prime_logs(&mut self) {
        if !self.has_records() {
            return;
        }

        let chunk = {
            let ctx = self.build_log_context();
            self.log_generator
                .generate_startup_chunk(&ctx, LOG_CHUNK_RANGE)
        };

        for line in chunk.lines {
            self.push_log(line);
        }
    }

    fn queue_next_chunk(&mut self) {
        if !self.has_records() {
            self.next_log_delay_ms = self.log_generator.next_chunk_pause_ms();
            return;
        }

        let chunk = {
            let ctx = self.build_log_context();
            self.log_generator.generate_chunk(&ctx, LOG_CHUNK_RANGE)
        };
        self.pending_log_lines = chunk.lines.into();
    }

    fn push_log(&mut self, line: LogLine) {
        self.log_generator.note_emitted_line(line.text.as_str());
        self.log_lines.push_back(line);
        while self.log_lines.len() > LOG_BUFFER_LIMIT {
            self.log_lines.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::archive::ArchiveLoader;

    use super::{RecordPageTab, SessionModel, LOG_BUFFER_LIMIT};
    use tachyonfx::Duration;

    #[test]
    fn moving_record_resets_page_and_scroll() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new_with_log_seed(archive, 11);

        session.active_page = RecordPageTab::Media;
        session.detail_scroll = 7;
        session.move_record(1);

        assert_eq!(session.current_record_index(), 1);
        assert_eq!(session.active_page, RecordPageTab::Overview);
        assert_eq!(session.detail_scroll, 0);
    }

    #[test]
    fn moving_record_updates_flat_metadata() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new_with_log_seed(archive, 12);

        session.move_record(1);

        assert_eq!(
            session.current_record().id,
            session.current_record_meta().record_id
        );
    }

    #[test]
    fn startup_seeding_populates_generated_logs_immediately() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let session = SessionModel::new_with_log_seed(archive, 13);

        assert!((4..=7).contains(&session.log_lines.len()));
        assert!(session.log_lines.iter().any(|line| {
            line.text.contains("manifest hydrated")
                || line.text.contains("signal attach")
                || line.text.contains("provisional reader")
        }));
    }

    #[test]
    fn runtime_streaming_emits_one_line_at_a_time() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new_with_log_seed(archive, 14);
        let initial_len = session.log_lines.len();
        let pause = session.next_log_delay_ms;

        session.tick(Duration::from_millis(pause));
        assert_eq!(session.log_lines.len(), initial_len + 1);
        assert!(!session.pending_log_lines.is_empty());

        let intra_chunk_delay = session.next_log_delay_ms;
        session.tick(Duration::from_millis(intra_chunk_delay));
        assert_eq!(session.log_lines.len(), initial_len + 2);
    }

    #[test]
    fn moving_record_changes_future_chunk_content() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new_with_log_seed(archive, 15);

        session.move_record(1);
        let pause = session.next_log_delay_ms;
        session.tick(Duration::from_millis(pause));

        let record_id = session.current_record().id.clone();
        assert!(session
            .log_lines
            .iter()
            .rev()
            .take(7)
            .any(|line| line.text.contains(record_id.as_str())));
    }

    #[test]
    fn moving_page_changes_future_chunk_content() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new_with_log_seed(archive, 16);

        session.move_record_page(1);
        let pause = session.next_log_delay_ms;
        session.tick(Duration::from_millis(pause));

        assert!(session
            .log_lines
            .iter()
            .rev()
            .take(7)
            .any(|line| line.text.contains(session.active_page.label())));
    }

    #[test]
    fn log_buffer_limit_is_enforced() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new_with_log_seed(archive, 17);

        for _ in 0..256 {
            session.tick(Duration::from_millis(400));
        }

        assert_eq!(session.log_lines.len(), LOG_BUFFER_LIMIT);
    }
}
