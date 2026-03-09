use crate::{
    archive::{ArchiveCategory, ArchiveRecord},
    state::StateId,
};
use tachyonfx::Duration;

pub struct SessionModel {
    pub selected_category: usize,
    pub selected_record_by_category: Vec<usize>,
    pub detail_scroll: usize,
    pub decryption_status: &'static str,
    pub terminal_locked: bool,
    pub corruption_tick: u64,
    pub corruption_elapsed_ms: u32,
    pub terminal_return_state: StateId,
}

impl SessionModel {
    pub fn new(category_count: usize) -> Self {
        Self {
            selected_category: 0,
            selected_record_by_category: vec![0; category_count],
            detail_scroll: 0,
            decryption_status: "archive signal attached // decryption stalled at 0%",
            terminal_locked: true,
            corruption_tick: 0,
            corruption_elapsed_ms: 0,
            terminal_return_state: StateId::Archive,
        }
    }

    pub fn current_category<'a>(&self, archive: &'a [ArchiveCategory]) -> &'a ArchiveCategory {
        &archive[self.selected_category]
    }

    pub fn current_record<'a>(&self, archive: &'a [ArchiveCategory]) -> &'a ArchiveRecord {
        let category = self.current_category(archive);
        &category.records[self.current_record_index()]
    }

    pub fn current_record_index(&self) -> usize {
        self.selected_record_by_category[self.selected_category]
    }

    pub fn move_category(&mut self, delta: i32, archive: &[ArchiveCategory]) {
        if archive.is_empty() {
            return;
        }
        let max_index = archive.len().saturating_sub(1) as i32;
        let next = (self.selected_category as i32 + delta).clamp(0, max_index) as usize;
        self.selected_category = next;
        let record_max = archive[next].records.len().saturating_sub(1);
        self.selected_record_by_category[next] =
            self.selected_record_by_category[next].min(record_max);
        self.detail_scroll = 0;
        self.decryption_status = "category alignment updated // archive selection preserved";
    }

    pub fn move_record(&mut self, delta: i32, archive: &[ArchiveCategory]) {
        let max_index = archive[self.selected_category]
            .records
            .len()
            .saturating_sub(1) as i32;
        let current = self.selected_record_by_category[self.selected_category] as i32;
        let next = (current + delta).clamp(0, max_index) as usize;
        self.selected_record_by_category[self.selected_category] = next;
        self.detail_scroll = 0;
        self.decryption_status = "record focus changed // preview buffers refreshed";
    }

    pub fn scroll_detail(&mut self, delta: i32) {
        let next = (self.detail_scroll as i32 + delta).max(0) as usize;
        self.detail_scroll = next;
    }

    pub fn open_terminal_from(&mut self, state: StateId) {
        self.terminal_return_state = state;
        self.decryption_status = "terminal subsystem requested // access remains denied";
    }

    pub fn tick(&mut self, elapsed: Duration) {
        self.corruption_elapsed_ms += elapsed.as_millis();
        if self.corruption_elapsed_ms > 1200 {
            self.corruption_tick = self.corruption_tick.wrapping_add(1);
            self.corruption_elapsed_ms = 0;
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
}
