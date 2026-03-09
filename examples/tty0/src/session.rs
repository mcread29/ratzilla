use std::{collections::VecDeque, rc::Rc};

use crate::{
    archive::{ArchiveStore, FlatRecordEntry, MediaHealth, RecordDocument},
    audio::{AudioController, PlaybackView},
    state::StateId,
};
use tachyonfx::Duration;

const LOG_BUFFER_LIMIT: usize = 64;

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
    Metadata,
    Notes,
    Media,
}

impl RecordPageTab {
    pub const ALL: [Self; 6] = [
        Self::Overview,
        Self::Dossier,
        Self::Timeline,
        Self::Metadata,
        Self::Notes,
        Self::Media,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Dossier => "dossier",
            Self::Timeline => "timeline",
            Self::Metadata => "metadata",
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
    pub log_tick: u64,
    pub viewer_tick: u64,
    audio: AudioController,
}

impl SessionModel {
    pub fn new(archive: Rc<ArchiveStore>) -> Self {
        let mut session = Self {
            archive,
            selected_record: 0,
            detail_scroll: 0,
            active_page: RecordPageTab::Overview,
            decryption_status: "archive signal attached // manifest hydrated // decryption stalled at 0%",
            corruption_tick: 0,
            corruption_elapsed_ms: 0,
            terminal_return_state: StateId::Archive,
            log_lines: VecDeque::new(),
            log_elapsed_ms: 0,
            log_tick: 0,
            viewer_tick: 0,
            audio: AudioController::new(),
        };
        session.bootstrap_logs();
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
            RecordPageTab::Metadata => "metadata surface active",
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
        while self.log_elapsed_ms >= self.next_log_interval() {
            self.log_elapsed_ms -= self.next_log_interval();
            let line = self.build_log_line();
            self.push_log(line);
            self.log_tick = self.log_tick.wrapping_add(1);
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

    fn next_log_interval(&self) -> u32 {
        90 + ((self.log_tick % 6) as u32 * 10)
    }

    fn bootstrap_logs(&mut self) {
        let initial_logs = [
            (
                "◎ manifest parsed -> embedded archive hydrated",
                LogColorRole::Sync,
            ),
            (
                "• decryption truthful at 0% -> waiting for reader movement",
                LogColorRole::Info,
            ),
            (
                "△ terminal surface present -> authority withheld",
                LogColorRole::Deny,
            ),
            (
                "┆ content files mounted -> records / pages / media surfaces",
                LogColorRole::Info,
            ),
            (
                "◇ tty0 left the labels readable on purpose",
                LogColorRole::Ghost,
            ),
        ];

        for (text, color_role) in initial_logs {
            self.push_log(LogLine {
                text: text.to_string(),
                color_role,
            });
        }
    }

    fn push_log(&mut self, line: LogLine) {
        self.log_lines.push_back(line);
        while self.log_lines.len() > LOG_BUFFER_LIMIT {
            self.log_lines.pop_front();
        }
    }

    fn build_log_line(&self) -> LogLine {
        let record = self.current_record();
        let record_meta = self.current_record_meta();
        let lane = self.log_tick % 20;

        if lane == 0 {
            return self.ghost_log(record);
        }
        if lane < 6 {
            return self.reactive_log(record_meta, record);
        }
        self.ambient_log(record_meta, record)
    }

    fn ambient_log(&self, record_meta: &FlatRecordEntry, record: &RecordDocument) -> LogLine {
        let variant = (self.log_tick % 7) as usize;
        let text = match variant {
            0 => format!(
                "• relay sync → {} → {}",
                record_meta.category_id, record.signal_integrity
            ),
            1 => format!("┆ checksum drift {} -> {}", record.id, self.corruption_label()),
            2 => format!(
                "░ index cache refresh -> {} [{}]",
                record_meta.record_id,
                record.badge_label()
            ),
            3 => format!("▁▂▃ page carrier -> {} / {}", record.id, self.active_page.label()),
            4 => format!(
                "• media state {} -> {}",
                record.id,
                record.media_health().label()
            ),
            5 => format!("┆ decryption unchanged -> {}", self.decryption_status),
            _ => format!("╎ archive path verified -> {}", record_meta.category_path),
        };

        let color_role = if variant == 1 || variant == 5 {
            LogColorRole::Warn
        } else if variant == 0 || variant == 4 {
            LogColorRole::Sync
        } else {
            LogColorRole::Info
        };

        LogLine { text, color_role }
    }

    fn reactive_log(&self, record_meta: &FlatRecordEntry, record: &RecordDocument) -> LogLine {
        let variant = (self.log_tick % 5) as usize;
        let text = match variant {
            0 => format!(
                "◎ selection -> {} [{}] // {}",
                record.id,
                record.badge_label(),
                record_meta.category_label
            ),
            1 => format!("› page {} -> {}", record.id, self.active_page.label()),
            2 => format!("› access {} -> {}", record.id, record.access_level.label()),
            3 => format!("› source {} -> {}", record.id, record.recovered_source),
            _ => format!("› media {} -> {}", record.id, record.media_health().label()),
        };

        let color_role = if record.media_health() == MediaHealth::Mounted {
            LogColorRole::Sync
        } else {
            LogColorRole::Info
        };

        LogLine { text, color_role }
    }

    fn ghost_log(&self, record: &RecordDocument) -> LogLine {
        let variant = (self.log_tick % 4) as usize;
        let text = match variant {
            0 => "◇ the signal noticed the host before the host named the signal".to_string(),
            1 => "◇ index first -> meaning later".to_string(),
            2 => format!("◇ tty0 withheld the rest of {}", record.id),
            _ => "◇ terminal surface acknowledged -> authority still absent".to_string(),
        };

        LogLine {
            text,
            color_role: LogColorRole::Ghost,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::archive::ArchiveLoader;

    use super::{RecordPageTab, SessionModel};

    #[test]
    fn moving_record_resets_page_and_scroll() {
        let archive = Rc::new(ArchiveLoader::load_embedded().expect("embedded archive"));
        let mut session = SessionModel::new(archive);

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
        let mut session = SessionModel::new(archive);

        session.move_record(1);

        assert_eq!(
            session.current_record().id,
            session.current_record_meta().record_id
        );
    }
}
