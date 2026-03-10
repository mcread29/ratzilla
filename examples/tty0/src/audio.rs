use crate::{
    archive::{MediaHealth, RecordDocument},
    track_visualizer::{decay_snapshot_toward_idle, AudioAnalysisSnapshot},
};
use web_sys::{
    AnalyserNode, AudioContext, AudioContextState, HtmlAudioElement, MediaElementAudioSourceNode,
};

const FFT_SIZE: u32 = 256;
const SMOOTHING_TIME_CONSTANT: f64 = 0.80;

pub struct AudioController {
    element: Option<HtmlAudioElement>,
    active_record_id: Option<String>,
    active_source: Option<String>,
    last_error: Option<String>,
    analysis_pipeline: Option<AudioAnalysisPipeline>,
    analysis_snapshot: Option<AudioAnalysisSnapshot>,
}

#[derive(Clone, Debug)]
pub struct PlaybackView {
    pub source: Option<String>,
    pub state_label: String,
    pub detail_label: String,
    pub progress_label: Option<String>,
    pub progress_ratio: Option<f32>,
    pub elapsed_secs: Option<f32>,
    pub duration_secs: Option<f32>,
    pub is_actionable: bool,
    pub is_playing: bool,
}

struct AudioAnalysisPipeline {
    context: AudioContext,
    #[allow(dead_code)]
    source_node: MediaElementAudioSourceNode,
    analyser: AnalyserNode,
    bins: Vec<u8>,
}

impl AudioController {
    pub fn new() -> Self {
        Self {
            element: None,
            active_record_id: None,
            active_source: None,
            last_error: None,
            analysis_pipeline: None,
            analysis_snapshot: None,
        }
    }

    pub fn stop(&mut self) {
        if let Some(audio) = &self.element {
            let _ = audio.pause();
            let _ = audio.set_current_time(0.0);
        }
        self.element = None;
        self.active_record_id = None;
        self.active_source = None;
        self.analysis_pipeline = None;
        self.analysis_snapshot = None;
    }

    pub fn sync(&mut self) {
        let Some(audio) = &self.element else {
            self.analysis_snapshot = None;
            return;
        };

        if audio.ended() {
            let _ = audio.set_current_time(0.0);
            self.analysis_snapshot = Some(AudioAnalysisSnapshot::idle(0.0));
            return;
        }

        let progress_ratio =
            compute_progress_ratio(audio.current_time(), audio.duration()).unwrap_or_default();
        let is_playing = !audio.paused() && !audio.ended();

        if is_playing {
            if let Some(pipeline) = &mut self.analysis_pipeline {
                self.analysis_snapshot = Some(pipeline.sample(audio));
            } else {
                self.analysis_snapshot = Some(AudioAnalysisSnapshot {
                    progress_ratio,
                    is_playing: true,
                    ..AudioAnalysisSnapshot::default()
                });
            }
        } else {
            self.analysis_snapshot = Some(match self.analysis_snapshot {
                Some(snapshot) => decay_snapshot_toward_idle(snapshot, progress_ratio),
                None => AudioAnalysisSnapshot::idle(progress_ratio),
            });
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
            self.analysis_pipeline = AudioAnalysisPipeline::new(&audio).ok();
            self.element = Some(audio);
            self.active_record_id = Some(record.id.clone());
            self.active_source = Some(source.to_string());
            self.analysis_snapshot = Some(AudioAnalysisSnapshot::idle(0.0));
            self.last_error = None;
        }

        let audio = self
            .element
            .as_ref()
            .ok_or_else(|| "audio transport is unavailable".to_string())?;

        if let Some(pipeline) = &self.analysis_pipeline {
            pipeline.resume_if_suspended();
        }

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
                progress_ratio: None,
                elapsed_secs: None,
                duration_secs: None,
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
                progress_ratio: None,
                elapsed_secs: None,
                duration_secs: None,
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
                    progress_ratio: None,
                    elapsed_secs: None,
                    duration_secs: None,
                    is_actionable: true,
                    is_playing: false,
                };
            }
        }

        if is_active {
            if let Some(audio) = &self.element {
                let duration = sanitize_duration(audio.duration());
                let current_time = audio.current_time().max(0.0) as f32;
                let progress_ratio = duration.and_then(|duration_secs| {
                    compute_progress_ratio(current_time as f64, duration_secs as f64)
                });
                let progress_label =
                    duration.map(|duration_secs| format_time(current_time, duration_secs));
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
                    progress_ratio,
                    elapsed_secs: Some(current_time),
                    duration_secs: duration,
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
            progress_ratio: None,
            elapsed_secs: None,
            duration_secs: None,
            is_actionable: true,
            is_playing: false,
        }
    }

    pub fn analysis_snapshot_for(&self, record: &RecordDocument) -> Option<AudioAnalysisSnapshot> {
        let is_active = self.active_record_id.as_deref() == Some(record.id.as_str());
        if is_active {
            self.analysis_snapshot
        } else {
            None
        }
    }
}

impl AudioAnalysisPipeline {
    fn new(audio: &HtmlAudioElement) -> Result<Self, String> {
        let context = AudioContext::new()
            .map_err(|_| "browser audio context could not be created".to_string())?;
        let analyser = context
            .create_analyser()
            .map_err(|_| "browser analyser node could not be created".to_string())?;
        analyser.set_fft_size(FFT_SIZE);
        analyser.set_smoothing_time_constant(SMOOTHING_TIME_CONSTANT);

        let source_node = context
            .create_media_element_source(audio)
            .map_err(|_| "audio element could not attach to analysis graph".to_string())?;
        source_node
            .connect_with_audio_node(&analyser)
            .map_err(|_| "media source could not connect to analyser".to_string())?;
        analyser
            .connect_with_audio_node(&context.destination())
            .map_err(|_| "analyser could not connect to output".to_string())?;

        let bins = vec![0; analyser.frequency_bin_count() as usize];

        Ok(Self {
            context,
            source_node,
            analyser,
            bins,
        })
    }

    fn resume_if_suspended(&self) {
        if self.context.state() == AudioContextState::Suspended {
            let _ = self.context.resume();
        }
    }

    fn sample(&mut self, audio: &HtmlAudioElement) -> AudioAnalysisSnapshot {
        self.resume_if_suspended();
        if self.bins.len() != self.analyser.frequency_bin_count() as usize {
            self.bins
                .resize(self.analyser.frequency_bin_count() as usize, 0);
        }
        self.analyser.get_byte_frequency_data(&mut self.bins);
        let (energy, bass, mid, treble, peak) = compute_analysis_levels(&self.bins);
        AudioAnalysisSnapshot {
            energy,
            bass,
            mid,
            treble,
            peak,
            progress_ratio: compute_progress_ratio(audio.current_time(), audio.duration())
                .unwrap_or_default(),
            is_playing: !audio.paused() && !audio.ended(),
        }
    }
}

fn compute_analysis_levels(bins: &[u8]) -> (f32, f32, f32, f32, f32) {
    if bins.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let peak = bins.iter().copied().max().unwrap_or_default() as f32 / 255.0;
    let bass_end = ((bins.len() as f32 * 0.20).round() as usize).clamp(1, bins.len());
    let mid_end = ((bins.len() as f32 * 0.65).round() as usize).clamp(bass_end + 1, bins.len());
    let bass = average_normalized(&bins[..bass_end]);
    let mid = average_normalized(&bins[bass_end..mid_end]);
    let treble = average_normalized(&bins[mid_end..]);
    let energy = average_normalized(bins);
    (energy, bass, mid, treble, peak)
}

fn average_normalized(slice: &[u8]) -> f32 {
    if slice.is_empty() {
        return 0.0;
    }
    let sum = slice.iter().map(|value| *value as f32).sum::<f32>();
    (sum / slice.len() as f32 / 255.0).clamp(0.0, 1.0)
}

fn compute_progress_ratio(current: f64, duration: f64) -> Option<f32> {
    if !duration.is_finite() || duration <= 0.0 || !current.is_finite() {
        return None;
    }
    Some((current / duration).clamp(0.0, 1.0) as f32)
}

fn sanitize_duration(duration: f64) -> Option<f32> {
    if duration.is_finite() && duration > 0.0 {
        Some(duration as f32)
    } else {
        None
    }
}

fn format_time(current: f32, duration: f32) -> String {
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

#[cfg(test)]
mod tests {
    use super::{compute_analysis_levels, compute_progress_ratio, AudioController};
    use crate::archive::{AccessLevel, MediaPage, RecordDocument, RecordKind};

    #[test]
    fn analysis_normalization_uses_expected_band_ranges() {
        let bins = vec![255; 20]
            .into_iter()
            .chain(vec![128; 45])
            .chain(vec![64; 35])
            .collect::<Vec<_>>();

        let (energy, bass, mid, treble, peak) = compute_analysis_levels(&bins);

        assert!((bass - 1.0).abs() < 0.001);
        assert!(mid > treble);
        assert!(energy > 0.4);
        assert!((peak - 1.0).abs() < 0.001);
    }

    #[test]
    fn progress_ratio_is_computed_for_valid_duration() {
        let progress = compute_progress_ratio(45.0, 180.0).expect("progress");
        assert!((progress - 0.25).abs() < 0.001);
        assert!(compute_progress_ratio(10.0, 0.0).is_none());
    }

    #[test]
    fn controller_exposes_no_analysis_snapshot_when_no_track_is_active() {
        let controller = AudioController::new();
        let record = RecordDocument {
            id: "0x07".to_string(),
            category_id: "transmissions".to_string(),
            kind: RecordKind::Transmission,
            title: "Argument List Too Long".to_string(),
            subtitle: "subtitle".to_string(),
            timeline: "SOL-1".to_string(),
            recovered_source: "archive".to_string(),
            collapse_vector: "vector".to_string(),
            signal_integrity: "100%".to_string(),
            access_level: AccessLevel::Readable,
            status_label: "RECOVERED".to_string(),
            tags: vec![],
            related_record_ids: vec![],
            overview: crate::archive::OverviewPage {
                summary: "summary".to_string(),
                thesis: "thesis".to_string(),
                viewer_hint: "hint".to_string(),
                content_warning: "cw".to_string(),
                status_callout: "callout".to_string(),
            },
            dossier: crate::archive::DossierPage { sections: vec![] },
            timeline_page: crate::archive::TimelinePage { events: vec![] },
            metadata_page: crate::archive::MetadataPage {
                classification: "classification".to_string(),
                source_chain: vec![],
                facts: vec![],
            },
            notes_page: crate::archive::NotesPage {
                tty0_annotation: vec![],
            },
            media_page: MediaPage {
                audio: None,
                artifact_note: "artifact".to_string(),
                transcript_excerpt: "excerpt".to_string(),
                waveform_mode: crate::archive::WaveformMode::Corrupted,
                visualizer: None,
                corruption_reason: Some("missing".to_string()),
            },
        };

        assert!(controller.analysis_snapshot_for(&record).is_none());
    }
}
