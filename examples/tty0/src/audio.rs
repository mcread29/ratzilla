use crate::archive::{MediaHealth, RecordDocument};
use web_sys::HtmlAudioElement;

pub struct AudioController {
    element: Option<HtmlAudioElement>,
    active_record_id: Option<String>,
    active_source: Option<String>,
    last_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PlaybackView {
    pub source: Option<String>,
    pub state_label: String,
    pub detail_label: String,
    pub progress_label: Option<String>,
    pub is_actionable: bool,
    pub is_playing: bool,
}

impl AudioController {
    pub fn new() -> Self {
        Self {
            element: None,
            active_record_id: None,
            active_source: None,
            last_error: None,
        }
    }

    pub fn stop(&mut self) {
        if let Some(audio) = &self.element {
            let _ = audio.pause();
            let _ = audio.set_current_time(0.0);
        }
        self.active_record_id = None;
        self.active_source = None;
    }

    pub fn sync(&mut self) {
        if let Some(audio) = &self.element {
            if audio.ended() {
                let _ = audio.set_current_time(0.0);
            }
        }
    }

    pub fn toggle(&mut self, record: &RecordDocument) -> Result<bool, String> {
        let Some(source) = record.audio_source() else {
            return Err(record
                .media_page
                .corruption_reason
                .clone()
                .unwrap_or_else(|| "record media is corrupted".to_string()));
        };

        let same_record = self.active_record_id.as_deref() == Some(record.id.as_str())
            && self.active_source.as_deref() == Some(source);

        if !same_record {
            let audio = HtmlAudioElement::new_with_src(source)
                .map_err(|_| "browser audio element could not be created".to_string())?;
            audio.set_preload("auto");
            self.element = Some(audio);
            self.active_record_id = Some(record.id.clone());
            self.active_source = Some(source.to_string());
            self.last_error = None;
        }

        let audio = self
            .element
            .as_ref()
            .ok_or_else(|| "audio transport is unavailable".to_string())?;

        if audio.paused() || audio.ended() {
            let _ = audio.set_current_time(if audio.ended() {
                0.0
            } else {
                audio.current_time()
            });
            let _ = audio.play().map_err(|_| {
                let message = "browser blocked playback or transport failed".to_string();
                self.last_error = Some(message.clone());
                message
            })?;
            self.last_error = None;
            Ok(true)
        } else {
            audio.pause().map_err(|_| {
                let message = "browser transport refused to pause".to_string();
                self.last_error = Some(message.clone());
                message
            })?;
            Ok(false)
        }
    }

    pub fn view_for(&self, record: &RecordDocument) -> PlaybackView {
        if record.media_health() == MediaHealth::Corrupted {
            return PlaybackView {
                source: None,
                state_label: "CORRUPTED".to_string(),
                detail_label: record
                    .media_page
                    .corruption_reason
                    .clone()
                    .unwrap_or_else(|| "no mounted transport".to_string()),
                progress_label: None,
                is_actionable: false,
                is_playing: false,
            };
        }

        let Some(source) = record.audio_source() else {
            return PlaybackView {
                source: None,
                state_label: "CORRUPTED".to_string(),
                detail_label: "no mounted transport".to_string(),
                progress_label: None,
                is_actionable: false,
                is_playing: false,
            };
        };

        let is_active = self.active_record_id.as_deref() == Some(record.id.as_str());
        if is_active {
            if let Some(error) = &self.last_error {
                return PlaybackView {
                    source: Some(source.to_string()),
                    state_label: "transport error".to_string(),
                    detail_label: error.clone(),
                    progress_label: None,
                    is_actionable: true,
                    is_playing: false,
                };
            }
        }

        if is_active {
            if let Some(audio) = &self.element {
                let duration = audio.duration();
                let current_time = audio.current_time();
                let progress_label = if duration.is_finite() && duration > 0.0 {
                    Some(format_time(current_time, duration))
                } else {
                    None
                };
                let is_playing = !audio.paused() && !audio.ended();
                return PlaybackView {
                    source: Some(source.to_string()),
                    state_label: if is_playing {
                        "transport playing".to_string()
                    } else {
                        "transport ready".to_string()
                    },
                    detail_label: if is_playing {
                        format!("press P to pause {}", record.id)
                    } else {
                        format!("press P to play {}", record.id)
                    },
                    progress_label,
                    is_actionable: true,
                    is_playing,
                };
            }
        }

        PlaybackView {
            source: Some(source.to_string()),
            state_label: "transport ready".to_string(),
            detail_label: format!("press P to play {}", record.id),
            progress_label: None,
            is_actionable: true,
            is_playing: false,
        }
    }
}

fn format_time(current: f64, duration: f64) -> String {
    let current = current.max(0.0) as u32;
    let duration = duration.max(0.0) as u32;
    format!(
        "{:02}:{:02} / {:02}:{:02}",
        current / 60,
        current % 60,
        duration / 60,
        duration % 60
    )
}
