use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

include!(concat!(env!("OUT_DIR"), "/embedded_archive.rs"));

#[derive(Clone, Debug)]
pub struct ArchiveStore {
    categories: Vec<ArchiveCategory>,
    records: Vec<FlatRecordEntry>,
    record_by_id: HashMap<String, (usize, usize)>,
}

#[derive(Clone, Debug)]
pub struct ArchiveCategory {
    pub id: String,
    pub label: String,
    pub path: String,
    pub description: String,
    pub records: Vec<RecordDocument>,
}

#[derive(Clone, Debug)]
pub struct FlatRecordEntry {
    pub record_id: String,
    pub category_id: String,
    pub category_label: String,
    pub category_path: String,
    pub category_description: String,
    category_idx: usize,
    record_idx: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RecordDocument {
    pub id: String,
    pub category_id: String,
    pub kind: RecordKind,
    pub title: String,
    pub subtitle: String,
    pub timeline: String,
    pub recovered_source: String,
    pub collapse_vector: String,
    pub signal_integrity: String,
    pub access_level: AccessLevel,
    pub status_label: String,
    pub tags: Vec<String>,
    pub related_record_ids: Vec<String>,
    pub overview: OverviewPage,
    pub dossier: DossierPage,
    pub timeline_page: TimelinePage,
    pub metadata_page: MetadataPage,
    pub notes_page: NotesPage,
    pub media_page: MediaPage,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OverviewPage {
    pub summary: String,
    pub thesis: String,
    pub viewer_hint: String,
    pub content_warning: String,
    pub status_callout: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DossierPage {
    pub sections: Vec<DossierSection>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DossierSection {
    pub title: String,
    pub body: String,
    pub emphasis: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimelinePage {
    pub events: Vec<TimelineEvent>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimelineEvent {
    pub label: String,
    pub timestamp_text: String,
    pub body: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MetadataPage {
    pub classification: String,
    pub source_chain: Vec<String>,
    pub facts: Vec<MetadataFact>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MetadataFact {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NotesPage {
    pub tty0_annotation: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MediaPage {
    pub audio: Option<AudioArtifact>,
    pub artifact_note: String,
    pub transcript_excerpt: String,
    pub waveform_mode: WaveformMode,
    #[serde(default)]
    pub visualizer: Option<TrackVisualizerConfig>,
    pub corruption_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AudioArtifact {
    pub path: String,
    pub title: String,
    pub duration_hint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TrackVisualizerConfig {
    pub mode: TrackVisualizerMode,
    #[serde(default)]
    pub params: TrackVisualizerParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<ChromaticBulgeGridAutomation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<ChromaticBulgeGridClipTimeline>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackVisualizerMode {
    DiplomaticSignalBloom,
    HexWalkerRelay,
    ContainmentLattice,
    ChromaticBulgeGrid,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TrackVisualizerParams {
    #[serde(default = "default_motion_rate")]
    pub motion_rate: f32,
    #[serde(default = "default_energy_gain")]
    pub energy_gain: f32,
    #[serde(default = "default_bass_gain")]
    pub bass_gain: f32,
    #[serde(default = "default_mid_gain")]
    pub mid_gain: f32,
    #[serde(default = "default_treble_gain")]
    pub treble_gain: f32,
    #[serde(default = "default_ring_count")]
    pub ring_count: u16,
    #[serde(default = "default_particle_count")]
    pub particle_count: u16,
    #[serde(default = "default_lattice_density")]
    pub lattice_density: u16,
    #[serde(default)]
    pub shader_states: Option<ChromaticBulgeGridShaderStates>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridShaderStates {
    #[serde(default)]
    pub playing: ChromaticBulgeGridShaderState,
    #[serde(default, alias = "not_playing")]
    pub idle: ChromaticBulgeGridShaderState,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridShaderState {
    #[serde(default = "default_motion_rate")]
    pub motion_rate: f32,
    #[serde(default = "default_shader_lattice_density")]
    pub lattice_density: f32,
    #[serde(default = "default_circle_radius")]
    pub circle_radius: f32,
    #[serde(default = "default_circle_falloff_start")]
    pub circle_falloff_start: f32,
    #[serde(default = "default_circle_falloff_end")]
    pub circle_falloff_end: f32,
    #[serde(default = "default_bulge_amount")]
    pub bulge_amount: f32,
    #[serde(default = "default_rim_guard")]
    pub rim_guard: f32,
    #[serde(default = "default_rim_exponent")]
    pub rim_exponent: f32,
    #[serde(default = "default_rim_warp")]
    pub rim_warp: f32,
    #[serde(default = "default_spacing_max_px")]
    pub spacing_max_px: f32,
    #[serde(default = "default_spacing_min_px")]
    pub spacing_min_px: f32,
    #[serde(default = "default_dot_size")]
    pub dot_size: f32,
    #[serde(default = "default_outer_dot_scale")]
    pub outer_dot_scale: f32,
    #[serde(default = "default_edge_softness")]
    pub edge_softness: f32,
    #[serde(default = "default_chromatic_aberration")]
    pub chromatic_aberration: f32,
    #[serde(default = "default_scroll_base")]
    pub scroll_base: f32,
    #[serde(default = "default_scroll_motion_scale")]
    pub scroll_motion_scale: f32,
    #[serde(default = "default_scroll_motion_floor")]
    pub scroll_motion_floor: f32,
    #[serde(default = "default_scroll_motion_ceiling")]
    pub scroll_motion_ceiling: f32,
    #[serde(default = "default_cold_color")]
    pub cold_color: [f32; 3],
    #[serde(default = "default_hot_color")]
    pub hot_color: [f32; 3],
    #[serde(default = "default_color_cycle_rate")]
    pub color_cycle_rate: f32,
    #[serde(default = "default_inner_alpha")]
    pub inner_alpha: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridAutomation {
    pub bpm: f32,
    pub measures: u32,
    #[serde(default = "default_beats_per_measure")]
    pub beats_per_measure: u32,
    #[serde(default)]
    pub lanes: ChromaticBulgeGridAutomationLanes,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridClipTimeline {
    pub bpm: f32,
    pub measures: u32,
    #[serde(default = "default_beats_per_measure")]
    pub beats_per_measure: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<ChromaticBulgeGridClip>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrangement: Vec<ClipPlacement>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridClip {
    pub id: String,
    pub name: String,
    pub length_beats: f32,
    #[serde(default = "default_clip_color")]
    pub color: [f32; 3],
    #[serde(default)]
    pub lanes: ChromaticBulgeGridAutomationLanes,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClipPlacement {
    pub clip_id: String,
    pub start_beat: f32,
    #[serde(default = "default_repeat_count")]
    pub repeats: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ChromaticBulgeGridAutomationLanes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub motion_rate: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lattice_density: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circle_radius: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circle_falloff_start: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circle_falloff_end: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bulge_amount: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rim_guard: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rim_exponent: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rim_warp: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spacing_max_px: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spacing_min_px: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dot_size: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_dot_scale: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_softness: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chromatic_aberration: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scroll_base: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scroll_motion_scale: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scroll_motion_floor: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scroll_motion_ceiling: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cold_color: Vec<ColorKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hot_color: Vec<ColorKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub color_cycle_rate: Vec<FloatKeyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_alpha: Vec<FloatKeyframe>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct FloatKeyframe {
    pub beat: f32,
    pub value: f32,
    #[serde(default)]
    pub interpolation: InterpolationMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ColorKeyframe {
    pub beat: f32,
    pub value: [f32; 3],
    #[serde(default)]
    pub interpolation: InterpolationMode,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMode {
    #[default]
    Hold,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackClock {
    pub current_time_secs: f32,
    pub visual_time_secs: f32,
    pub duration_secs: Option<f32>,
    pub is_playing: bool,
    pub timeline_preview: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromaticBulgeGridResolvedState {
    pub uniforms: ChromaticBulgeGridShaderState,
    pub current_beat: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Incident,
    WitnessTestimony,
    Transmission,
    Analysis,
    Residue,
    PrivateLog,
    Tooling,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Readable,
    IndexOnly,
    Sealed,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WaveformMode {
    Text,
    Waveform,
    Spectrum,
    Corrupted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaHealth {
    Mounted,
    Corrupted,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveManifestDoc {
    pub categories: Vec<ArchiveCategoryDoc>,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveCategoryDoc {
    pub id: String,
    pub label: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Error)]
pub enum ArchiveLoadError {
    #[error("failed to parse {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("archive validation failed: {0}")]
    Validation(String),
}

pub struct ArchiveLoader;

impl ArchiveLoader {
    pub fn load_embedded() -> Result<ArchiveStore, ArchiveLoadError> {
        Self::load_from_strs(MANIFEST_JSON, RECORD_JSONS)
    }

    fn load_from_strs(
        manifest_json: &str,
        record_jsons: &[(&str, &str)],
    ) -> Result<ArchiveStore, ArchiveLoadError> {
        let manifest = parse_json::<ArchiveManifestDoc>("archive_manifest.json", manifest_json)?;
        validate_unique_category_ids(&manifest)?;

        let mut categories = manifest
            .categories
            .into_iter()
            .map(|category| ArchiveCategory {
                id: category.id,
                label: category.label,
                path: category.path,
                description: category.description,
                records: Vec::new(),
            })
            .collect::<Vec<_>>();

        let mut category_index = HashMap::new();
        for (index, category) in categories.iter().enumerate() {
            category_index.insert(category.id.clone(), index);
        }

        let mut record_by_id = HashMap::new();
        let mut related_edges = Vec::new();

        for (record_file, record_json) in record_jsons {
            let record = parse_json::<RecordDocument>(record_file, record_json)?;
            validate_record_visualizer(&record)?;
            let category_idx = category_index
                .get(&record.category_id)
                .copied()
                .ok_or_else(|| {
                    ArchiveLoadError::Validation(format!(
                        "record {} references unknown category {}",
                        record.id, record.category_id
                    ))
                })?;

            if record_by_id.contains_key(record.id.as_str()) {
                return Err(ArchiveLoadError::Validation(format!(
                    "duplicate record id {}",
                    record.id
                )));
            }

            let record_idx = categories[category_idx].records.len();
            related_edges.push((record.id.clone(), record.related_record_ids.clone()));
            record_by_id.insert(record.id.clone(), (category_idx, record_idx));
            categories[category_idx].records.push(record);
        }

        for (record_id, related_ids) in related_edges {
            for related_id in related_ids {
                if !record_by_id.contains_key(related_id.as_str()) {
                    return Err(ArchiveLoadError::Validation(format!(
                        "record {} references unknown related record {}",
                        record_id, related_id
                    )));
                }
            }
        }

        let mut records = categories
            .iter()
            .enumerate()
            .flat_map(|(category_idx, category)| {
                category
                    .records
                    .iter()
                    .enumerate()
                    .map(move |(record_idx, record)| FlatRecordEntry {
                        record_id: record.id.clone(),
                        category_id: category.id.clone(),
                        category_label: category.label.clone(),
                        category_path: category.path.clone(),
                        category_description: category.description.clone(),
                        category_idx,
                        record_idx,
                    })
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.record_id.cmp(&right.record_id));

        Ok(ArchiveStore {
            categories,
            records,
            record_by_id,
        })
    }
}

impl ArchiveStore {
    pub fn records(&self) -> &[FlatRecordEntry] {
        &self.records
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn record_at(&self, index: usize) -> Option<&RecordDocument> {
        let entry = self.records.get(index)?;
        self.categories
            .get(entry.category_idx)
            .and_then(|category| category.records.get(entry.record_idx))
    }

    pub fn record_meta_at(&self, index: usize) -> Option<&FlatRecordEntry> {
        self.records.get(index)
    }

    pub fn record_by_id(&self, id: &str) -> Option<&RecordDocument> {
        let (category_idx, record_idx) = self.record_by_id.get(id)?;
        self.categories
            .get(*category_idx)
            .and_then(|category| category.records.get(*record_idx))
    }
}

impl RecordDocument {
    pub fn page_count(&self) -> usize {
        6
    }

    pub fn media_health(&self) -> MediaHealth {
        if self.media_page.audio.is_some() {
            MediaHealth::Mounted
        } else {
            MediaHealth::Corrupted
        }
    }

    pub fn audio_source(&self) -> Option<&str> {
        self.media_page
            .audio
            .as_ref()
            .map(|audio| audio.path.as_str())
    }

    pub fn visualizer(&self) -> Option<&TrackVisualizerConfig> {
        self.media_page.visualizer.as_ref()
    }

    pub fn badge_label(&self) -> &'static str {
        match self.access_level {
            AccessLevel::Readable => match self.media_health() {
                MediaHealth::Mounted => "AUD",
                MediaHealth::Corrupted => "COR",
            },
            AccessLevel::IndexOnly => "IDX",
            AccessLevel::Sealed => "LOCK",
        }
    }
}

impl RecordKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Incident => "incident",
            Self::WitnessTestimony => "witness testimony",
            Self::Transmission => "transmission",
            Self::Analysis => "analysis",
            Self::Residue => "signal residue",
            Self::PrivateLog => "private log",
            Self::Tooling => "tool manifest",
        }
    }
}

impl AccessLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Readable => "readable",
            Self::IndexOnly => "index only",
            Self::Sealed => "sealed",
        }
    }
}

impl WaveformMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Waveform => "waveform",
            Self::Spectrum => "spectrum",
            Self::Corrupted => "corrupted",
        }
    }
}

impl MediaHealth {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mounted => "media mounted",
            Self::Corrupted => "media corrupted",
        }
    }
}

impl TrackVisualizerMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::DiplomaticSignalBloom => "diplomatic signal bloom",
            Self::HexWalkerRelay => "hex walker relay",
            Self::ContainmentLattice => "containment lattice",
            Self::ChromaticBulgeGrid => "chromatic bulge grid",
        }
    }
}

impl Default for TrackVisualizerParams {
    fn default() -> Self {
        Self {
            motion_rate: default_motion_rate(),
            energy_gain: default_energy_gain(),
            bass_gain: default_bass_gain(),
            mid_gain: default_mid_gain(),
            treble_gain: default_treble_gain(),
            ring_count: default_ring_count(),
            particle_count: default_particle_count(),
            lattice_density: default_lattice_density(),
            shader_states: None,
        }
    }
}

impl Default for ChromaticBulgeGridShaderStates {
    fn default() -> Self {
        Self {
            playing: ChromaticBulgeGridShaderState::default(),
            idle: ChromaticBulgeGridShaderState::default(),
        }
    }
}

impl Default for ChromaticBulgeGridShaderState {
    fn default() -> Self {
        Self {
            motion_rate: default_motion_rate(),
            lattice_density: default_shader_lattice_density(),
            circle_radius: default_circle_radius(),
            circle_falloff_start: default_circle_falloff_start(),
            circle_falloff_end: default_circle_falloff_end(),
            bulge_amount: default_bulge_amount(),
            rim_guard: default_rim_guard(),
            rim_exponent: default_rim_exponent(),
            rim_warp: default_rim_warp(),
            spacing_max_px: default_spacing_max_px(),
            spacing_min_px: default_spacing_min_px(),
            dot_size: default_dot_size(),
            outer_dot_scale: default_outer_dot_scale(),
            edge_softness: default_edge_softness(),
            chromatic_aberration: default_chromatic_aberration(),
            scroll_base: default_scroll_base(),
            scroll_motion_scale: default_scroll_motion_scale(),
            scroll_motion_floor: default_scroll_motion_floor(),
            scroll_motion_ceiling: default_scroll_motion_ceiling(),
            cold_color: default_cold_color(),
            hot_color: default_hot_color(),
            color_cycle_rate: default_color_cycle_rate(),
            inner_alpha: default_inner_alpha(),
        }
    }
}

impl Default for PlaybackClock {
    fn default() -> Self {
        Self {
            current_time_secs: 0.0,
            visual_time_secs: 0.0,
            duration_secs: None,
            is_playing: false,
            timeline_preview: false,
        }
    }
}

impl ChromaticBulgeGridShaderState {
    pub fn from_legacy_params(params: &TrackVisualizerParams) -> Self {
        Self {
            motion_rate: params.motion_rate,
            lattice_density: params.lattice_density as f32,
            ..Self::default()
        }
    }

    pub fn clamp(self) -> Self {
        Self {
            motion_rate: self.motion_rate.clamp(0.2, 3.0),
            lattice_density: self.lattice_density.clamp(2.0, 12.0),
            circle_radius: self.circle_radius.clamp(0.02, 0.95),
            circle_falloff_start: self.circle_falloff_start.clamp(0.0, 1.0),
            circle_falloff_end: self.circle_falloff_end.clamp(0.0, 1.2),
            bulge_amount: self.bulge_amount.clamp(0.0, 1.5),
            rim_guard: self.rim_guard.clamp(0.01, 2.0),
            rim_exponent: self.rim_exponent.clamp(0.1, 6.0),
            rim_warp: self.rim_warp.clamp(0.0, 64.0),
            spacing_max_px: self.spacing_max_px.clamp(4.0, 64.0),
            spacing_min_px: self.spacing_min_px.clamp(2.0, 48.0),
            dot_size: self.dot_size.clamp(0.02, 1.0),
            outer_dot_scale: self.outer_dot_scale.clamp(0.02, 2.0),
            edge_softness: self.edge_softness.clamp(0.1, 8.0),
            chromatic_aberration: self.chromatic_aberration.clamp(0.0, 24.0),
            scroll_base: self.scroll_base.clamp(0.0, 200.0),
            scroll_motion_scale: self.scroll_motion_scale.clamp(0.0, 200.0),
            scroll_motion_floor: self.scroll_motion_floor.clamp(0.0, 4.0),
            scroll_motion_ceiling: self.scroll_motion_ceiling.clamp(0.0, 8.0),
            cold_color: clamp_color(self.cold_color),
            hot_color: clamp_color(self.hot_color),
            color_cycle_rate: self.color_cycle_rate.clamp(0.0, 6.0),
            inner_alpha: self.inner_alpha.clamp(0.0, 1.0),
        }
    }
}

impl TrackVisualizerConfig {
    pub fn normalized_for_export(&self) -> Self {
        let mut normalized = self.clone();
        normalized.params = normalized.params.normalized();
        if let Some(timeline) = normalized.timeline.take() {
            normalized.timeline = Some(timeline.normalized());
            normalized.automation = None;
        } else {
            normalized.automation = normalized
                .automation
                .map(|automation| automation.normalized());
        }
        normalized
    }

    pub fn resolve_chromatic_bulge_grid(
        &self,
        playback: PlaybackClock,
    ) -> ChromaticBulgeGridResolvedState {
        let states = self.params.shader_states.unwrap_or_else(|| {
            let state = ChromaticBulgeGridShaderState::from_legacy_params(&self.params);
            ChromaticBulgeGridShaderStates {
                playing: state,
                idle: state,
            }
        });
        let bpm = self
            .timeline
            .as_ref()
            .map(|timeline| timeline.bpm)
            .or_else(|| self.automation.as_ref().map(|automation| automation.bpm))
            .unwrap_or_else(default_bpm)
            .max(1.0);
        let current_beat = playback.current_time_secs.max(0.0) * bpm / 60.0;
        let base = if playback.is_playing {
            states.playing
        } else {
            states.idle
        };

        let uniforms = if let Some(timeline) = &self.timeline {
            if playback.is_playing || playback.timeline_preview {
                timeline
                    .resolve_state_at_beat(base, current_beat)
                    .unwrap_or(base)
            } else {
                base
            }
        } else if let Some(automation) = &self.automation {
            if playback.is_playing {
                automation.apply_to_state(base, current_beat)
            } else {
                base
            }
        } else {
            base
        };

        ChromaticBulgeGridResolvedState {
            uniforms: uniforms.clamp(),
            current_beat,
        }
    }
}

impl TrackVisualizerParams {
    fn normalized(&self) -> Self {
        Self {
            motion_rate: self.motion_rate,
            energy_gain: self.energy_gain,
            bass_gain: self.bass_gain,
            mid_gain: self.mid_gain,
            treble_gain: self.treble_gain,
            ring_count: self.ring_count,
            particle_count: self.particle_count,
            lattice_density: self.lattice_density,
            shader_states: self
                .shader_states
                .map(|states| ChromaticBulgeGridShaderStates {
                    playing: states.playing.clamp(),
                    idle: states.idle.clamp(),
                }),
        }
    }
}

impl ChromaticBulgeGridAutomation {
    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.bpm = normalized.bpm.max(1.0);
        normalized.measures = normalized.measures.max(1);
        normalized.beats_per_measure = normalized.beats_per_measure.max(1);
        normalized.lanes.sort_all();
        normalized
    }

    pub fn total_beats(&self) -> f32 {
        self.measures.max(1) as f32 * self.beats_per_measure.max(1) as f32
    }

    pub fn duration_secs(&self) -> f32 {
        self.total_beats() * 60.0 / self.bpm.max(1.0)
    }

    fn apply_to_state(
        &self,
        base: ChromaticBulgeGridShaderState,
        beat: f32,
    ) -> ChromaticBulgeGridShaderState {
        apply_lanes_to_state(base, &self.normalized().lanes, beat)
    }
}

impl ChromaticBulgeGridClipTimeline {
    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.bpm = normalized.bpm.max(1.0);
        normalized.measures = normalized.measures.max(1);
        normalized.beats_per_measure = normalized.beats_per_measure.max(1);
        for clip in &mut normalized.clips {
            clip.length_beats = clip.length_beats.max(0.0001);
            clip.color = clamp_color(clip.color);
            clip.lanes.sort_all();
        }
        for placement in &mut normalized.arrangement {
            placement.start_beat = placement.start_beat.max(0.0);
            placement.repeats = placement.repeats.max(1);
        }
        normalized
    }

    pub fn total_beats(&self) -> f32 {
        self.measures.max(1) as f32 * self.beats_per_measure.max(1) as f32
    }

    pub fn duration_secs(&self) -> f32 {
        self.total_beats() * 60.0 / self.bpm.max(1.0)
    }

    pub fn clip_by_id(&self, clip_id: &str) -> Option<&ChromaticBulgeGridClip> {
        self.clips.iter().find(|clip| clip.id == clip_id)
    }

    pub fn resolve_state_at_beat(
        &self,
        base: ChromaticBulgeGridShaderState,
        beat: f32,
    ) -> Option<ChromaticBulgeGridShaderState> {
        let (clip, local_beat) = self.active_clip_at_beat(beat)?;
        Some(clip.apply_to_state(base, local_beat))
    }

    pub fn active_clip_at_beat(&self, beat: f32) -> Option<(&ChromaticBulgeGridClip, f32)> {
        let beat = beat.max(0.0);
        for placement in &self.arrangement {
            let clip = self.clip_by_id(&placement.clip_id)?;
            let end = placement.end_beat(clip);
            if beat >= placement.start_beat && beat < end {
                let local = if clip.length_beats <= 0.0 {
                    0.0
                } else {
                    ((beat - placement.start_beat) % clip.length_beats)
                        .clamp(0.0, clip.length_beats)
                };
                return Some((clip, local));
            }
        }
        None
    }

    pub fn has_authored_content(&self) -> bool {
        !self.clips.is_empty() || !self.arrangement.is_empty()
    }
}

impl ChromaticBulgeGridClip {
    pub fn apply_to_state(
        &self,
        base: ChromaticBulgeGridShaderState,
        local_beat: f32,
    ) -> ChromaticBulgeGridShaderState {
        apply_lanes_to_state(
            base,
            &self.lanes,
            local_beat.clamp(0.0, self.length_beats.max(0.0)),
        )
    }
}

impl ClipPlacement {
    pub fn end_beat(&self, clip: &ChromaticBulgeGridClip) -> f32 {
        self.start_beat + clip.length_beats * self.repeats.max(1) as f32
    }
}

pub fn legacy_automation_to_timeline(
    automation: &ChromaticBulgeGridAutomation,
) -> ChromaticBulgeGridClipTimeline {
    ChromaticBulgeGridClipTimeline {
        bpm: automation.bpm,
        measures: automation.measures,
        beats_per_measure: automation.beats_per_measure,
        clips: vec![ChromaticBulgeGridClip {
            id: "imported_timeline".to_string(),
            name: "Imported Timeline".to_string(),
            length_beats: automation.total_beats(),
            color: default_clip_color(),
            lanes: automation.lanes.clone(),
        }],
        arrangement: vec![ClipPlacement {
            clip_id: "imported_timeline".to_string(),
            start_beat: 0.0,
            repeats: 1,
        }],
    }
}

impl ChromaticBulgeGridAutomationLanes {
    fn sort_all(&mut self) {
        sort_float_keyframes(&mut self.motion_rate);
        sort_float_keyframes(&mut self.lattice_density);
        sort_float_keyframes(&mut self.circle_radius);
        sort_float_keyframes(&mut self.circle_falloff_start);
        sort_float_keyframes(&mut self.circle_falloff_end);
        sort_float_keyframes(&mut self.bulge_amount);
        sort_float_keyframes(&mut self.rim_guard);
        sort_float_keyframes(&mut self.rim_exponent);
        sort_float_keyframes(&mut self.rim_warp);
        sort_float_keyframes(&mut self.spacing_max_px);
        sort_float_keyframes(&mut self.spacing_min_px);
        sort_float_keyframes(&mut self.dot_size);
        sort_float_keyframes(&mut self.outer_dot_scale);
        sort_float_keyframes(&mut self.edge_softness);
        sort_float_keyframes(&mut self.chromatic_aberration);
        sort_float_keyframes(&mut self.scroll_base);
        sort_float_keyframes(&mut self.scroll_motion_scale);
        sort_float_keyframes(&mut self.scroll_motion_floor);
        sort_float_keyframes(&mut self.scroll_motion_ceiling);
        sort_color_keyframes(&mut self.cold_color);
        sort_color_keyframes(&mut self.hot_color);
        sort_float_keyframes(&mut self.color_cycle_rate);
        sort_float_keyframes(&mut self.inner_alpha);
    }

    pub fn is_empty(&self) -> bool {
        self.motion_rate.is_empty()
            && self.lattice_density.is_empty()
            && self.circle_radius.is_empty()
            && self.circle_falloff_start.is_empty()
            && self.circle_falloff_end.is_empty()
            && self.bulge_amount.is_empty()
            && self.rim_guard.is_empty()
            && self.rim_exponent.is_empty()
            && self.rim_warp.is_empty()
            && self.spacing_max_px.is_empty()
            && self.spacing_min_px.is_empty()
            && self.dot_size.is_empty()
            && self.outer_dot_scale.is_empty()
            && self.edge_softness.is_empty()
            && self.chromatic_aberration.is_empty()
            && self.scroll_base.is_empty()
            && self.scroll_motion_scale.is_empty()
            && self.scroll_motion_floor.is_empty()
            && self.scroll_motion_ceiling.is_empty()
            && self.cold_color.is_empty()
            && self.hot_color.is_empty()
            && self.color_cycle_rate.is_empty()
            && self.inner_alpha.is_empty()
    }

    pub fn automated_lane_count(&self) -> usize {
        [
            !self.motion_rate.is_empty(),
            !self.lattice_density.is_empty(),
            !self.circle_radius.is_empty(),
            !self.circle_falloff_start.is_empty(),
            !self.circle_falloff_end.is_empty(),
            !self.bulge_amount.is_empty(),
            !self.rim_guard.is_empty(),
            !self.rim_exponent.is_empty(),
            !self.rim_warp.is_empty(),
            !self.spacing_max_px.is_empty(),
            !self.spacing_min_px.is_empty(),
            !self.dot_size.is_empty(),
            !self.outer_dot_scale.is_empty(),
            !self.edge_softness.is_empty(),
            !self.chromatic_aberration.is_empty(),
            !self.scroll_base.is_empty(),
            !self.scroll_motion_scale.is_empty(),
            !self.scroll_motion_floor.is_empty(),
            !self.scroll_motion_ceiling.is_empty(),
            !self.cold_color.is_empty(),
            !self.hot_color.is_empty(),
            !self.color_cycle_rate.is_empty(),
            !self.inner_alpha.is_empty(),
        ]
        .into_iter()
        .filter(|used| *used)
        .count()
    }
}

fn default_motion_rate() -> f32 {
    1.0
}

fn default_bpm() -> f32 {
    120.0
}

fn default_beats_per_measure() -> u32 {
    4
}

fn default_repeat_count() -> u32 {
    1
}

fn default_clip_color() -> [f32; 3] {
    [110.0 / 255.0, 220.0 / 255.0, 212.0 / 255.0]
}

fn default_energy_gain() -> f32 {
    1.1
}

fn default_bass_gain() -> f32 {
    1.25
}

fn default_mid_gain() -> f32 {
    1.0
}

fn default_treble_gain() -> f32 {
    1.1
}

fn default_ring_count() -> u16 {
    4
}

fn default_particle_count() -> u16 {
    48
}

fn default_lattice_density() -> u16 {
    6
}

fn default_shader_lattice_density() -> f32 {
    6.0
}

fn default_circle_radius() -> f32 {
    0.24
}

fn default_circle_falloff_start() -> f32 {
    0.78
}

fn default_circle_falloff_end() -> f32 {
    1.0
}

fn default_bulge_amount() -> f32 {
    0.42
}

fn default_rim_guard() -> f32 {
    0.55
}

fn default_rim_exponent() -> f32 {
    1.8
}

fn default_rim_warp() -> f32 {
    0.18
}

fn default_spacing_max_px() -> f32 {
    22.0
}

fn default_spacing_min_px() -> f32 {
    12.0
}

fn default_dot_size() -> f32 {
    0.16
}

fn default_outer_dot_scale() -> f32 {
    0.33
}

fn default_edge_softness() -> f32 {
    1.0
}

fn default_chromatic_aberration() -> f32 {
    0.28
}

fn default_scroll_base() -> f32 {
    28.0
}

fn default_scroll_motion_scale() -> f32 {
    42.0
}

fn default_scroll_motion_floor() -> f32 {
    0.2
}

fn default_scroll_motion_ceiling() -> f32 {
    2.8
}

fn default_cold_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

fn default_hot_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

fn default_color_cycle_rate() -> f32 {
    0.16
}

fn default_inner_alpha() -> f32 {
    0.9
}

fn parse_json<T: for<'de> Deserialize<'de>>(path: &str, json: &str) -> Result<T, ArchiveLoadError> {
    serde_json::from_str(json).map_err(|source| ArchiveLoadError::Json {
        path: path.to_string(),
        source,
    })
}

fn validate_unique_category_ids(manifest: &ArchiveManifestDoc) -> Result<(), ArchiveLoadError> {
    let mut seen = HashSet::new();
    for category in &manifest.categories {
        if !seen.insert(category.id.as_str()) {
            return Err(ArchiveLoadError::Validation(format!(
                "duplicate category id {}",
                category.id
            )));
        }
    }
    Ok(())
}

fn validate_record_visualizer(record: &RecordDocument) -> Result<(), ArchiveLoadError> {
    let Some(visualizer) = record.visualizer() else {
        return Ok(());
    };

    let has_automation = visualizer
        .automation
        .as_ref()
        .is_some_and(|automation| !automation.lanes.is_empty());
    let has_timeline = visualizer
        .timeline
        .as_ref()
        .is_some_and(ChromaticBulgeGridClipTimeline::has_authored_content);
    if has_automation && has_timeline {
        return Err(ArchiveLoadError::Validation(format!(
            "record {} contains both automation and timeline",
            record.id
        )));
    }

    if visualizer.automation.is_some() || visualizer.timeline.is_some() {
        if visualizer.mode != TrackVisualizerMode::ChromaticBulgeGrid {
            return Err(ArchiveLoadError::Validation(format!(
                "record {} uses automation/timeline on unsupported visualizer mode {}",
                record.id,
                visualizer.mode.label()
            )));
        }
    }

    if let Some(automation) = &visualizer.automation {
        validate_automation(record.id.as_str(), automation)?;
    }

    if let Some(timeline) = &visualizer.timeline {
        validate_clip_timeline(record.id.as_str(), timeline)?;
    }

    Ok(())
}

fn validate_automation(
    record_id: &str,
    automation: &ChromaticBulgeGridAutomation,
) -> Result<(), ArchiveLoadError> {
    if !automation.bpm.is_finite() || automation.bpm <= 0.0 {
        return Err(ArchiveLoadError::Validation(format!(
            "record {record_id} has invalid automation bpm"
        )));
    }
    if automation.measures == 0 {
        return Err(ArchiveLoadError::Validation(format!(
            "record {record_id} has invalid automation measures"
        )));
    }
    if automation.beats_per_measure == 0 {
        return Err(ArchiveLoadError::Validation(format!(
            "record {record_id} has invalid automation beats_per_measure"
        )));
    }

    validate_automation_lanes(record_id, "automation", &automation.lanes, None)?;
    Ok(())
}

fn validate_clip_timeline(
    record_id: &str,
    timeline: &ChromaticBulgeGridClipTimeline,
) -> Result<(), ArchiveLoadError> {
    if !timeline.bpm.is_finite() || timeline.bpm <= 0.0 {
        return Err(ArchiveLoadError::Validation(format!(
            "record {record_id} has invalid timeline bpm"
        )));
    }
    if timeline.measures == 0 {
        return Err(ArchiveLoadError::Validation(format!(
            "record {record_id} has invalid timeline measures"
        )));
    }
    if timeline.beats_per_measure == 0 {
        return Err(ArchiveLoadError::Validation(format!(
            "record {record_id} has invalid timeline beats_per_measure"
        )));
    }

    let total_beats = timeline.total_beats();
    let mut clip_ids = HashSet::new();
    for clip in &timeline.clips {
        if clip.id.trim().is_empty() {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has clip with empty id"
            )));
        }
        if !clip_ids.insert(clip.id.as_str()) {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has duplicate clip id {}",
                clip.id
            )));
        }
        if clip.name.trim().is_empty() {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has empty clip name for {}",
                clip.id
            )));
        }
        if !clip.length_beats.is_finite() || clip.length_beats <= 0.0 {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has invalid length_beats for clip {}",
                clip.id
            )));
        }
        validate_automation_lanes(
            record_id,
            format!("clip {}", clip.id).as_str(),
            &clip.lanes,
            Some(clip.length_beats),
        )?;
    }

    let mut spans = Vec::<(f32, f32, &str)>::new();
    for placement in &timeline.arrangement {
        if placement.clip_id.trim().is_empty() {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has placement with empty clip_id"
            )));
        }
        let Some(clip) = timeline.clip_by_id(&placement.clip_id) else {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} placement references missing clip {}",
                placement.clip_id
            )));
        };
        if !placement.start_beat.is_finite() || placement.start_beat < 0.0 {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has invalid placement start_beat for {}",
                placement.clip_id
            )));
        }
        if placement.repeats == 0 {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has invalid placement repeats for {}",
                placement.clip_id
            )));
        }
        let end = placement.end_beat(clip);
        if end > total_beats + 0.0001 {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} placement for {} extends beyond total timeline beats",
                placement.clip_id
            )));
        }
        spans.push((placement.start_beat, end, placement.clip_id.as_str()));
    }

    spans.sort_by(|left, right| left.0.total_cmp(&right.0));
    for window in spans.windows(2) {
        if window[0].1 > window[1].0 + 0.0001 {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has overlapping placements between {} and {}",
                window[0].2, window[1].2
            )));
        }
    }

    Ok(())
}

fn validate_automation_lanes(
    record_id: &str,
    lane_prefix: &str,
    lanes: &ChromaticBulgeGridAutomationLanes,
    max_beat: Option<f32>,
) -> Result<(), ArchiveLoadError> {
    validate_float_lane(
        record_id,
        lane_prefix,
        "motion_rate",
        &lanes.motion_rate,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "lattice_density",
        &lanes.lattice_density,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "circle_radius",
        &lanes.circle_radius,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "circle_falloff_start",
        &lanes.circle_falloff_start,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "circle_falloff_end",
        &lanes.circle_falloff_end,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "bulge_amount",
        &lanes.bulge_amount,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "rim_guard",
        &lanes.rim_guard,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "rim_exponent",
        &lanes.rim_exponent,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "rim_warp",
        &lanes.rim_warp,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "spacing_max_px",
        &lanes.spacing_max_px,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "spacing_min_px",
        &lanes.spacing_min_px,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "dot_size",
        &lanes.dot_size,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "outer_dot_scale",
        &lanes.outer_dot_scale,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "edge_softness",
        &lanes.edge_softness,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "chromatic_aberration",
        &lanes.chromatic_aberration,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "scroll_base",
        &lanes.scroll_base,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "scroll_motion_scale",
        &lanes.scroll_motion_scale,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "scroll_motion_floor",
        &lanes.scroll_motion_floor,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "scroll_motion_ceiling",
        &lanes.scroll_motion_ceiling,
        max_beat,
    )?;
    validate_color_lane(
        record_id,
        lane_prefix,
        "cold_color",
        &lanes.cold_color,
        max_beat,
    )?;
    validate_color_lane(
        record_id,
        lane_prefix,
        "hot_color",
        &lanes.hot_color,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "color_cycle_rate",
        &lanes.color_cycle_rate,
        max_beat,
    )?;
    validate_float_lane(
        record_id,
        lane_prefix,
        "inner_alpha",
        &lanes.inner_alpha,
        max_beat,
    )?;
    Ok(())
}

fn validate_float_lane(
    record_id: &str,
    lane_prefix: &str,
    lane_name: &str,
    keyframes: &[FloatKeyframe],
    max_beat: Option<f32>,
) -> Result<(), ArchiveLoadError> {
    validate_duplicate_beats(
        record_id,
        lane_prefix,
        lane_name,
        keyframes.iter().map(|keyframe| keyframe.beat),
        max_beat,
    )
}

fn validate_color_lane(
    record_id: &str,
    lane_prefix: &str,
    lane_name: &str,
    keyframes: &[ColorKeyframe],
    max_beat: Option<f32>,
) -> Result<(), ArchiveLoadError> {
    validate_duplicate_beats(
        record_id,
        lane_prefix,
        lane_name,
        keyframes.iter().map(|keyframe| keyframe.beat),
        max_beat,
    )
}

fn validate_duplicate_beats(
    record_id: &str,
    lane_prefix: &str,
    lane_name: &str,
    beats: impl Iterator<Item = f32>,
    max_beat: Option<f32>,
) -> Result<(), ArchiveLoadError> {
    let mut unique = Vec::<f32>::new();
    for beat in beats {
        if !beat.is_finite() || beat < 0.0 {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has invalid beat in {lane_prefix} lane {lane_name}"
            )));
        }
        if let Some(limit) = max_beat {
            if beat > limit + 0.0001 {
                return Err(ArchiveLoadError::Validation(format!(
                    "record {record_id} has beat {beat} beyond clip length in {lane_prefix} lane {lane_name}"
                )));
            }
        }
        if unique.iter().any(|seen| (seen - beat).abs() < 0.0001) {
            return Err(ArchiveLoadError::Validation(format!(
                "record {record_id} has duplicate beat {beat} in {lane_prefix} lane {lane_name}"
            )));
        }
        unique.push(beat);
    }
    Ok(())
}

fn apply_lanes_to_state(
    base: ChromaticBulgeGridShaderState,
    lanes: &ChromaticBulgeGridAutomationLanes,
    beat: f32,
) -> ChromaticBulgeGridShaderState {
    ChromaticBulgeGridShaderState {
        motion_rate: sample_float_lane(&lanes.motion_rate, beat, base.motion_rate),
        lattice_density: sample_float_lane(&lanes.lattice_density, beat, base.lattice_density),
        circle_radius: sample_float_lane(&lanes.circle_radius, beat, base.circle_radius),
        circle_falloff_start: sample_float_lane(
            &lanes.circle_falloff_start,
            beat,
            base.circle_falloff_start,
        ),
        circle_falloff_end: sample_float_lane(
            &lanes.circle_falloff_end,
            beat,
            base.circle_falloff_end,
        ),
        bulge_amount: sample_float_lane(&lanes.bulge_amount, beat, base.bulge_amount),
        rim_guard: sample_float_lane(&lanes.rim_guard, beat, base.rim_guard),
        rim_exponent: sample_float_lane(&lanes.rim_exponent, beat, base.rim_exponent),
        rim_warp: sample_float_lane(&lanes.rim_warp, beat, base.rim_warp),
        spacing_max_px: sample_float_lane(&lanes.spacing_max_px, beat, base.spacing_max_px),
        spacing_min_px: sample_float_lane(&lanes.spacing_min_px, beat, base.spacing_min_px),
        dot_size: sample_float_lane(&lanes.dot_size, beat, base.dot_size),
        outer_dot_scale: sample_float_lane(&lanes.outer_dot_scale, beat, base.outer_dot_scale),
        edge_softness: sample_float_lane(&lanes.edge_softness, beat, base.edge_softness),
        chromatic_aberration: sample_float_lane(
            &lanes.chromatic_aberration,
            beat,
            base.chromatic_aberration,
        ),
        scroll_base: sample_float_lane(&lanes.scroll_base, beat, base.scroll_base),
        scroll_motion_scale: sample_float_lane(
            &lanes.scroll_motion_scale,
            beat,
            base.scroll_motion_scale,
        ),
        scroll_motion_floor: sample_float_lane(
            &lanes.scroll_motion_floor,
            beat,
            base.scroll_motion_floor,
        ),
        scroll_motion_ceiling: sample_float_lane(
            &lanes.scroll_motion_ceiling,
            beat,
            base.scroll_motion_ceiling,
        ),
        cold_color: sample_color_lane(&lanes.cold_color, beat, base.cold_color),
        hot_color: sample_color_lane(&lanes.hot_color, beat, base.hot_color),
        color_cycle_rate: sample_float_lane(&lanes.color_cycle_rate, beat, base.color_cycle_rate),
        inner_alpha: sample_float_lane(&lanes.inner_alpha, beat, base.inner_alpha),
    }
}

fn sample_float_lane(keyframes: &[FloatKeyframe], beat: f32, base: f32) -> f32 {
    if keyframes.is_empty() {
        return base;
    }

    let clamped_beat = beat.max(0.0);
    if clamped_beat < keyframes[0].beat {
        return base;
    }

    for window in keyframes.windows(2) {
        let left = window[0];
        let right = window[1];
        if clamped_beat < right.beat {
            return match left.interpolation {
                InterpolationMode::Hold => left.value,
                InterpolationMode::Linear => {
                    let span = (right.beat - left.beat).max(0.0001);
                    let t = ((clamped_beat - left.beat) / span).clamp(0.0, 1.0);
                    lerp(left.value, right.value, t)
                }
            };
        }
    }

    keyframes.last().map_or(base, |keyframe| keyframe.value)
}

fn sample_color_lane(keyframes: &[ColorKeyframe], beat: f32, base: [f32; 3]) -> [f32; 3] {
    if keyframes.is_empty() {
        return base;
    }

    let clamped_beat = beat.max(0.0);
    if clamped_beat < keyframes[0].beat {
        return base;
    }

    for window in keyframes.windows(2) {
        let left = window[0];
        let right = window[1];
        if clamped_beat < right.beat {
            return match left.interpolation {
                InterpolationMode::Hold => left.value,
                InterpolationMode::Linear => {
                    let span = (right.beat - left.beat).max(0.0001);
                    let t = ((clamped_beat - left.beat) / span).clamp(0.0, 1.0);
                    [
                        lerp(left.value[0], right.value[0], t),
                        lerp(left.value[1], right.value[1], t),
                        lerp(left.value[2], right.value[2], t),
                    ]
                }
            };
        }
    }

    keyframes.last().map_or(base, |keyframe| keyframe.value)
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn clamp_color(value: [f32; 3]) -> [f32; 3] {
    [
        value[0].clamp(0.0, 1.0),
        value[1].clamp(0.0, 1.0),
        value[2].clamp(0.0, 1.0),
    ]
}

fn sort_float_keyframes(keyframes: &mut [FloatKeyframe]) {
    keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
}

fn sort_color_keyframes(keyframes: &mut [ColorKeyframe]) {
    keyframes.sort_by(|left, right| left.beat.total_cmp(&right.beat));
}

#[cfg(test)]
mod tests {
    use super::{
        ArchiveLoadError, ArchiveLoader, MediaHealth, TrackVisualizerConfig, TrackVisualizerMode,
        TrackVisualizerParams,
    };

    const MANIFEST: &str = r#"{
      "categories": [
        {
          "id": "witnesses",
          "label": "witnesses",
          "path": "/recovered/humanity/witnesses",
          "description": "Human testimony preserved as evidence."
        }
      ]
    }"#;

    fn readable_record_json(id: &str, audio_path: Option<&str>, related: &[&str]) -> String {
        readable_record_json_with_visualizer(id, audio_path, related, None)
    }

    fn readable_record_json_with_visualizer(
        id: &str,
        audio_path: Option<&str>,
        related: &[&str],
        visualizer: Option<&str>,
    ) -> String {
        let audio = match audio_path {
            Some(path) => format!(
                "{{\"path\":\"{path}\",\"title\":\"Artifact\",\"duration_hint\":\"03:12\"}}"
            ),
            None => "null".to_string(),
        };
        let visualizer = visualizer
            .map(|visualizer| format!(",\"visualizer\":{visualizer}"))
            .unwrap_or_default();
        let related = related
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{
              "id": "{id}",
              "category_id": "witnesses",
              "kind": "witness_testimony",
              "title": "Title",
              "subtitle": "Subtitle",
              "timeline": "SOL-1",
              "recovered_source": "Archive",
              "collapse_vector": "Failure",
              "signal_integrity": "99.0%",
              "access_level": "readable",
              "status_label": "RECOVERED",
              "tags": ["alpha","beta"],
              "related_record_ids": [{related}],
              "overview": {{
                "summary": "Summary",
                "thesis": "Thesis",
                "viewer_hint": "Hint",
                "content_warning": "cw",
                "status_callout": "callout"
              }},
              "dossier": {{
                "sections": [
                  {{"title":"One","body":"Body","emphasis":"e1"}},
                  {{"title":"Two","body":"Body","emphasis":"e2"}},
                  {{"title":"Three","body":"Body","emphasis":"e3"}},
                  {{"title":"Four","body":"Body","emphasis":"e4"}}
                ]
              }},
              "timeline_page": {{
                "events": [
                  {{"label":"One","timestamp_text":"t1","body":"b1"}},
                  {{"label":"Two","timestamp_text":"t2","body":"b2"}},
                  {{"label":"Three","timestamp_text":"t3","body":"b3"}},
                  {{"label":"Four","timestamp_text":"t4","body":"b4"}}
                ]
              }},
              "metadata_page": {{
                "classification": "C",
                "source_chain": ["s1","s2"],
                "facts": [
                  {{"label":"a","value":"1"}},
                  {{"label":"b","value":"2"}},
                  {{"label":"c","value":"3"}},
                  {{"label":"d","value":"4"}},
                  {{"label":"e","value":"5"}},
                  {{"label":"f","value":"6"}},
                  {{"label":"g","value":"7"}},
                  {{"label":"h","value":"8"}}
                ]
              }},
              "notes_page": {{
                "tty0_annotation": ["n1","n2"]
              }},
              "media_page": {{
                "audio": {audio},
                "artifact_note": "artifact",
                "transcript_excerpt": "excerpt",
                "waveform_mode": "waveform"{visualizer},
                "corruption_reason": "missing source"
              }}
            }}"#
        )
    }

    #[test]
    fn parses_manifest_and_records() {
        let records = [("one.json", readable_record_json("one", Some("a.mp3"), &[]))];
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", records[0].1.as_str())])
            .expect("archive store");

        assert_eq!(store.record_count(), 1);
        assert_eq!(store.records()[0].record_id, "one");
        assert_eq!(store.record_at(0).expect("one").page_count(), 6);
    }

    #[test]
    fn rejects_duplicate_ids() {
        let record = readable_record_json("dup", Some("a.mp3"), &[]);
        let error = ArchiveLoader::load_from_strs(
            MANIFEST,
            &[("one.json", &record), ("two.json", &record)],
        )
        .expect_err("duplicate ids should fail");

        assert!(
            matches!(error, ArchiveLoadError::Validation(message) if message.contains("duplicate record id"))
        );
    }

    #[test]
    fn rejects_unknown_category() {
        let record = readable_record_json("bad", Some("a.mp3"), &[]).replace(
            "\"category_id\": \"witnesses\"",
            "\"category_id\": \"missing\"",
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("bad.json", &record)])
            .expect_err("unknown category should fail");

        assert!(
            matches!(error, ArchiveLoadError::Validation(message) if message.contains("unknown category"))
        );
    }

    #[test]
    fn rejects_unknown_related_record() {
        let record = readable_record_json("one", Some("a.mp3"), &["missing"]);
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("unknown related record should fail");

        assert!(
            matches!(error, ArchiveLoadError::Validation(message) if message.contains("unknown related record"))
        );
    }

    #[test]
    fn derives_media_health_from_audio_presence() {
        let mounted = readable_record_json("one", Some("a.mp3"), &[]);
        let corrupted = readable_record_json("two", None, &["one"]);
        let store = ArchiveLoader::load_from_strs(
            MANIFEST,
            &[("one.json", &mounted), ("two.json", &corrupted)],
        )
        .expect("archive store");

        assert_eq!(
            store.record_by_id("one").expect("one").media_health(),
            MediaHealth::Mounted
        );
        assert_eq!(
            store.record_by_id("two").expect("two").media_health(),
            MediaHealth::Corrupted
        );
    }

    #[test]
    fn embedded_fixture_loads_and_all_records_expose_six_pages() {
        let store = ArchiveLoader::load_embedded().expect("embedded archive store");

        for entry in store.records() {
            let record = store
                .record_by_id(&entry.record_id)
                .expect("embedded record should resolve");
            assert_eq!(record.page_count(), 6, "{}", record.id);
        }
    }

    #[test]
    fn flat_record_index_is_sorted_by_record_id() {
        let one = readable_record_json("0x10", Some("a.mp3"), &[]);
        let two = readable_record_json("0x02", None, &["0x10"]);
        let three = readable_record_json("0x07", None, &["0x10"]);

        let store = ArchiveLoader::load_from_strs(
            MANIFEST,
            &[
                ("one.json", &one),
                ("two.json", &two),
                ("three.json", &three),
            ],
        )
        .expect("archive store");

        let ids = store
            .records()
            .iter()
            .map(|entry| entry.record_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["0x02", "0x07", "0x10"]);
    }

    #[test]
    fn flat_record_index_exposes_category_metadata() {
        let record = readable_record_json("one", Some("a.mp3"), &[]);
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");

        let entry = store.record_meta_at(0).expect("flat metadata");
        assert_eq!(entry.category_id, "witnesses");
        assert_eq!(entry.category_label, "witnesses");
        assert_eq!(entry.category_path, "/recovered/humanity/witnesses");
        assert_eq!(store.record_at(0).expect("flat record").id, "one");
    }

    #[test]
    fn visualizer_config_parses_when_present() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{"mode":"diplomatic_signal_bloom","params":{"motion_rate":1.5,"ring_count":5}}"#,
            ),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let visualizer = store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer");

        assert_eq!(visualizer.mode, TrackVisualizerMode::DiplomaticSignalBloom);
        assert_eq!(visualizer.params.motion_rate, 1.5);
        assert_eq!(visualizer.params.ring_count, 5);
    }

    #[test]
    fn containment_lattice_visualizer_config_parses_when_present() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{"mode":"containment_lattice","params":{"motion_rate":0.8,"particle_count":12}}"#,
            ),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let visualizer = store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer");

        assert_eq!(visualizer.mode, TrackVisualizerMode::ContainmentLattice);
        assert_eq!(visualizer.params.motion_rate, 0.8);
        assert_eq!(visualizer.params.particle_count, 12);
    }

    #[test]
    fn chromatic_bulge_grid_visualizer_config_parses_when_present() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{"mode":"chromatic_bulge_grid","params":{"motion_rate":0.9,"lattice_density":10}}"#,
            ),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let visualizer = store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer");

        assert_eq!(visualizer.mode, TrackVisualizerMode::ChromaticBulgeGrid);
        assert_eq!(visualizer.params.motion_rate, 0.9);
        assert_eq!(visualizer.params.lattice_density, 10);
    }

    #[test]
    fn chromatic_bulge_grid_shader_states_parse_when_present() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "params":{
                        "shader_states":{
                            "playing":{
                                "motion_rate":0.7,
                                "lattice_density":9,
                                "circle_radius":0.3,
                                "chromatic_aberration":0.4
                            },
                            "idle":{
                                "motion_rate":0.25,
                                "lattice_density":4,
                                "circle_radius":0.18,
                                "chromatic_aberration":0.1
                            }
                        }
                    }
                }"#,
            ),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let states = store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer")
            .params
            .shader_states
            .expect("shader states");

        assert_eq!(states.playing.motion_rate, 0.7);
        assert_eq!(states.playing.circle_radius, 0.3);
        assert_eq!(states.idle.motion_rate, 0.25);
        assert_eq!(states.idle.chromatic_aberration, 0.1);
    }

    #[test]
    fn chromatic_bulge_grid_automation_parses_when_present() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "params":{
                        "shader_states":{
                            "playing":{"circle_radius":0.24},
                            "idle":{"circle_radius":0.18}
                        }
                    },
                    "automation":{
                        "bpm":132.0,
                        "measures":64,
                        "beats_per_measure":4,
                        "lanes":{
                            "circle_radius":[
                                {"beat":0.0,"value":0.24,"interpolation":"hold"},
                                {"beat":16.0,"value":0.31,"interpolation":"linear"}
                            ],
                            "cold_color":[
                                {"beat":0.0,"value":[1.0,1.0,1.0],"interpolation":"hold"}
                            ]
                        }
                    }
                }"#,
            ),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let automation = store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer")
            .automation
            .as_ref()
            .expect("automation");

        assert_eq!(automation.bpm, 132.0);
        assert_eq!(automation.lanes.circle_radius.len(), 2);
        assert_eq!(automation.lanes.cold_color.len(), 1);
    }

    #[test]
    fn chromatic_bulge_grid_timeline_parses_when_present() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":132.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {
                                "id":"pulse",
                                "name":"Pulse",
                                "length_beats":1.0,
                                "lanes":{
                                    "circle_radius":[
                                        {"beat":0.0,"value":0.24,"interpolation":"hold"},
                                        {"beat":1.0,"value":0.31,"interpolation":"linear"}
                                    ]
                                }
                            }
                        ],
                        "arrangement":[
                            {"clip_id":"pulse","start_beat":4.0,"repeats":4}
                        ]
                    }
                }"#,
            ),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let timeline = store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer")
            .timeline
            .as_ref()
            .expect("timeline");

        assert_eq!(timeline.bpm, 132.0);
        assert_eq!(timeline.clips.len(), 1);
        assert_eq!(timeline.arrangement.len(), 1);
    }

    #[test]
    fn rejects_duplicate_automation_beats_in_same_lane() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "automation":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "lanes":{
                            "circle_radius":[
                                {"beat":4.0,"value":0.24,"interpolation":"hold"},
                                {"beat":4.0,"value":0.31,"interpolation":"linear"}
                            ]
                        }
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("duplicate beats should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("duplicate beat") && message.contains("circle_radius")
        ));
    }

    #[test]
    fn rejects_duplicate_timeline_clip_ids() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {"id":"dup","name":"One","length_beats":1.0},
                            {"id":"dup","name":"Two","length_beats":2.0}
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("duplicate clip ids should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("duplicate clip id")
        ));
    }

    #[test]
    fn rejects_timeline_placement_referencing_missing_clip_id() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {"id":"pulse","name":"Pulse","length_beats":1.0}
                        ],
                        "arrangement":[
                            {"clip_id":"missing","start_beat":0.0,"repeats":1}
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("missing clip reference should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("missing clip")
        ));
    }

    #[test]
    fn rejects_timeline_clip_keyframes_outside_clip_length() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {
                                "id":"pulse",
                                "name":"Pulse",
                                "length_beats":1.0,
                                "lanes":{
                                    "circle_radius":[
                                        {"beat":1.5,"value":0.31,"interpolation":"hold"}
                                    ]
                                }
                            }
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("out of bounds keyframe should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("beyond clip length")
        ));
    }

    #[test]
    fn rejects_duplicate_timeline_local_beats_in_same_lane() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {
                                "id":"pulse",
                                "name":"Pulse",
                                "length_beats":2.0,
                                "lanes":{
                                    "circle_radius":[
                                        {"beat":0.5,"value":0.24,"interpolation":"hold"},
                                        {"beat":0.5,"value":0.31,"interpolation":"linear"}
                                    ]
                                }
                            }
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("duplicate local beats should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("duplicate beat") && message.contains("clip pulse")
        ));
    }

    #[test]
    fn rejects_overlapping_timeline_placements() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {"id":"a","name":"A","length_beats":4.0},
                            {"id":"b","name":"B","length_beats":4.0}
                        ],
                        "arrangement":[
                            {"clip_id":"a","start_beat":0.0,"repeats":1},
                            {"clip_id":"b","start_beat":3.0,"repeats":1}
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("overlap should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("overlapping placements")
        ));
    }

    #[test]
    fn rejects_timeline_placement_extending_beyond_total_beats() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "timeline":{
                        "bpm":120.0,
                        "measures":1,
                        "beats_per_measure":4,
                        "clips":[
                            {"id":"pulse","name":"Pulse","length_beats":2.0}
                        ],
                        "arrangement":[
                            {"clip_id":"pulse","start_beat":3.0,"repeats":1}
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("placement past total beats should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("extends beyond total timeline beats")
        ));
    }

    #[test]
    fn rejects_visualizer_with_both_automation_and_timeline() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(
                r#"{
                    "mode":"chromatic_bulge_grid",
                    "automation":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "lanes":{
                            "circle_radius":[
                                {"beat":0.0,"value":0.24,"interpolation":"hold"}
                            ]
                        }
                    },
                    "timeline":{
                        "bpm":120.0,
                        "measures":8,
                        "beats_per_measure":4,
                        "clips":[
                            {"id":"pulse","name":"Pulse","length_beats":1.0}
                        ]
                    }
                }"#,
            ),
        );
        let error = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect_err("dual schema should fail");

        assert!(matches!(
            error,
            ArchiveLoadError::Validation(message)
            if message.contains("both automation and timeline")
        ));
    }

    #[test]
    fn scalar_sampling_uses_base_before_first_keyframe_and_interpolates() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.24,
                        ..Default::default()
                    },
                    idle: Default::default(),
                }),
                ..Default::default()
            },
            automation: Some(super::ChromaticBulgeGridAutomation {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                lanes: super::ChromaticBulgeGridAutomationLanes {
                    circle_radius: vec![
                        super::FloatKeyframe {
                            beat: 4.0,
                            value: 0.30,
                            interpolation: super::InterpolationMode::Linear,
                        },
                        super::FloatKeyframe {
                            beat: 8.0,
                            value: 0.50,
                            interpolation: super::InterpolationMode::Linear,
                        },
                    ],
                    ..Default::default()
                },
            }),
            timeline: None,
        };

        let before = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 1.0,
            visual_time_secs: 1.0,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });
        let held = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 2.5,
            visual_time_secs: 2.5,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });
        let interpolated = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 4.5,
            visual_time_secs: 4.5,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });

        assert!((before.uniforms.circle_radius - 0.24).abs() < 0.001);
        assert!((held.uniforms.circle_radius - 0.35).abs() < 0.001);
        assert!((interpolated.uniforms.circle_radius - 0.50).abs() < 0.001);
    }

    #[test]
    fn color_sampling_interpolates_channel_wise() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        cold_color: [0.0, 0.0, 0.0],
                        ..Default::default()
                    },
                    idle: Default::default(),
                }),
                ..Default::default()
            },
            automation: Some(super::ChromaticBulgeGridAutomation {
                bpm: 60.0,
                measures: 8,
                beats_per_measure: 4,
                lanes: super::ChromaticBulgeGridAutomationLanes {
                    cold_color: vec![
                        super::ColorKeyframe {
                            beat: 0.0,
                            value: [0.0, 0.0, 0.0],
                            interpolation: super::InterpolationMode::Linear,
                        },
                        super::ColorKeyframe {
                            beat: 4.0,
                            value: [1.0, 0.5, 0.25],
                            interpolation: super::InterpolationMode::Linear,
                        },
                    ],
                    ..Default::default()
                },
            }),
            timeline: None,
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 2.0,
            visual_time_secs: 2.0,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });

        assert_eq!(resolved.uniforms.cold_color, [0.5, 0.25, 0.125]);
    }

    #[test]
    fn timeline_base_state_is_returned_before_first_active_placement() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.18,
                        ..Default::default()
                    },
                    idle: Default::default(),
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(super::ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![super::ChromaticBulgeGridClip {
                    id: "pulse".to_string(),
                    name: "Pulse".to_string(),
                    length_beats: 1.0,
                    color: [1.0, 1.0, 1.0],
                    lanes: super::ChromaticBulgeGridAutomationLanes {
                        circle_radius: vec![super::FloatKeyframe {
                            beat: 0.0,
                            value: 0.5,
                            interpolation: super::InterpolationMode::Hold,
                        }],
                        ..Default::default()
                    },
                }],
                arrangement: vec![super::ClipPlacement {
                    clip_id: "pulse".to_string(),
                    start_beat: 4.0,
                    repeats: 1,
                }],
            }),
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 1.0,
            visual_time_secs: 1.0,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });

        assert!((resolved.uniforms.circle_radius - 0.18).abs() < 0.001);
    }

    #[test]
    fn timeline_single_placement_samples_local_clip_beats_correctly() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.24,
                        ..Default::default()
                    },
                    idle: Default::default(),
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(super::ChromaticBulgeGridClipTimeline {
                bpm: 60.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![super::ChromaticBulgeGridClip {
                    id: "pulse".to_string(),
                    name: "Pulse".to_string(),
                    length_beats: 2.0,
                    color: [1.0, 1.0, 1.0],
                    lanes: super::ChromaticBulgeGridAutomationLanes {
                        circle_radius: vec![
                            super::FloatKeyframe {
                                beat: 0.0,
                                value: 0.3,
                                interpolation: super::InterpolationMode::Linear,
                            },
                            super::FloatKeyframe {
                                beat: 2.0,
                                value: 0.5,
                                interpolation: super::InterpolationMode::Linear,
                            },
                        ],
                        ..Default::default()
                    },
                }],
                arrangement: vec![super::ClipPlacement {
                    clip_id: "pulse".to_string(),
                    start_beat: 4.0,
                    repeats: 1,
                }],
            }),
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 5.0,
            visual_time_secs: 5.0,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });

        assert!((resolved.uniforms.circle_radius - 0.4).abs() < 0.001);
    }

    #[test]
    fn repeated_timeline_placement_wraps_local_beat_correctly() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.24,
                        ..Default::default()
                    },
                    idle: Default::default(),
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(super::ChromaticBulgeGridClipTimeline {
                bpm: 60.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![super::ChromaticBulgeGridClip {
                    id: "pulse".to_string(),
                    name: "Pulse".to_string(),
                    length_beats: 2.0,
                    color: [1.0, 1.0, 1.0],
                    lanes: super::ChromaticBulgeGridAutomationLanes {
                        circle_radius: vec![
                            super::FloatKeyframe {
                                beat: 0.0,
                                value: 0.3,
                                interpolation: super::InterpolationMode::Linear,
                            },
                            super::FloatKeyframe {
                                beat: 2.0,
                                value: 0.5,
                                interpolation: super::InterpolationMode::Linear,
                            },
                        ],
                        ..Default::default()
                    },
                }],
                arrangement: vec![super::ClipPlacement {
                    clip_id: "pulse".to_string(),
                    start_beat: 4.0,
                    repeats: 2,
                }],
            }),
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 7.0,
            visual_time_secs: 7.0,
            duration_secs: None,
            is_playing: true,
            timeline_preview: false,
        });

        assert!((resolved.uniforms.circle_radius - 0.4).abs() < 0.001);
    }

    #[test]
    fn imported_legacy_automation_timeline_matches_legacy_output() {
        let legacy = super::ChromaticBulgeGridAutomation {
            bpm: 120.0,
            measures: 8,
            beats_per_measure: 4,
            lanes: super::ChromaticBulgeGridAutomationLanes {
                circle_radius: vec![
                    super::FloatKeyframe {
                        beat: 0.0,
                        value: 0.3,
                        interpolation: super::InterpolationMode::Linear,
                    },
                    super::FloatKeyframe {
                        beat: 8.0,
                        value: 0.5,
                        interpolation: super::InterpolationMode::Linear,
                    },
                ],
                ..Default::default()
            },
        };
        let base_states = super::ChromaticBulgeGridShaderStates {
            playing: super::ChromaticBulgeGridShaderState {
                circle_radius: 0.24,
                ..Default::default()
            },
            idle: super::ChromaticBulgeGridShaderState {
                circle_radius: 0.18,
                ..Default::default()
            },
        };
        let legacy_config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(base_states),
                ..Default::default()
            },
            automation: Some(legacy.clone()),
            timeline: None,
        };
        let timeline_config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(base_states),
                ..Default::default()
            },
            automation: None,
            timeline: Some(super::legacy_automation_to_timeline(&legacy)),
        };

        for secs in [0.0_f32, 1.5, 2.75, 4.0] {
            let playback = super::PlaybackClock {
                current_time_secs: secs,
                visual_time_secs: secs,
                duration_secs: None,
                is_playing: true,
                timeline_preview: false,
            };
            let legacy_resolved = legacy_config.resolve_chromatic_bulge_grid(playback);
            let timeline_resolved = timeline_config.resolve_chromatic_bulge_grid(playback);
            assert!(
                (legacy_resolved.uniforms.circle_radius - timeline_resolved.uniforms.circle_radius)
                    .abs()
                    < 0.001
            );
        }
    }

    #[test]
    fn paused_timeline_without_preview_uses_base_idle_state() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.40,
                        ..Default::default()
                    },
                    idle: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.18,
                        ..Default::default()
                    },
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(super::ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![super::ChromaticBulgeGridClip {
                    id: "pulse".to_string(),
                    name: "Pulse".to_string(),
                    length_beats: 1.0,
                    color: [1.0, 1.0, 1.0],
                    lanes: super::ChromaticBulgeGridAutomationLanes {
                        circle_radius: vec![super::FloatKeyframe {
                            beat: 0.0,
                            value: 0.5,
                            interpolation: super::InterpolationMode::Hold,
                        }],
                        ..Default::default()
                    },
                }],
                arrangement: vec![super::ClipPlacement {
                    clip_id: "pulse".to_string(),
                    start_beat: 0.0,
                    repeats: 1,
                }],
            }),
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 0.0,
            visual_time_secs: 0.0,
            duration_secs: None,
            is_playing: false,
            timeline_preview: false,
        });

        assert!((resolved.uniforms.circle_radius - 0.18).abs() < 0.001);
    }

    #[test]
    fn paused_timeline_preview_applies_clip_state() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.40,
                        ..Default::default()
                    },
                    idle: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.18,
                        ..Default::default()
                    },
                }),
                ..Default::default()
            },
            automation: None,
            timeline: Some(super::ChromaticBulgeGridClipTimeline {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                clips: vec![super::ChromaticBulgeGridClip {
                    id: "pulse".to_string(),
                    name: "Pulse".to_string(),
                    length_beats: 1.0,
                    color: [1.0, 1.0, 1.0],
                    lanes: super::ChromaticBulgeGridAutomationLanes {
                        circle_radius: vec![super::FloatKeyframe {
                            beat: 0.0,
                            value: 0.5,
                            interpolation: super::InterpolationMode::Hold,
                        }],
                        ..Default::default()
                    },
                }],
                arrangement: vec![super::ClipPlacement {
                    clip_id: "pulse".to_string(),
                    start_beat: 0.0,
                    repeats: 1,
                }],
            }),
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 0.0,
            visual_time_secs: 0.0,
            duration_secs: None,
            is_playing: false,
            timeline_preview: true,
        });

        assert!((resolved.uniforms.circle_radius - 0.5).abs() < 0.001);
    }

    #[test]
    fn paused_transport_uses_idle_state() {
        let config = TrackVisualizerConfig {
            mode: TrackVisualizerMode::ChromaticBulgeGrid,
            params: TrackVisualizerParams {
                shader_states: Some(super::ChromaticBulgeGridShaderStates {
                    playing: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.40,
                        ..Default::default()
                    },
                    idle: super::ChromaticBulgeGridShaderState {
                        circle_radius: 0.18,
                        ..Default::default()
                    },
                }),
                ..Default::default()
            },
            automation: Some(super::ChromaticBulgeGridAutomation {
                bpm: 120.0,
                measures: 8,
                beats_per_measure: 4,
                lanes: super::ChromaticBulgeGridAutomationLanes {
                    circle_radius: vec![super::FloatKeyframe {
                        beat: 0.0,
                        value: 0.50,
                        interpolation: super::InterpolationMode::Hold,
                    }],
                    ..Default::default()
                },
            }),
            timeline: None,
        };

        let resolved = config.resolve_chromatic_bulge_grid(super::PlaybackClock {
            current_time_secs: 10.0,
            visual_time_secs: 10.0,
            duration_secs: None,
            is_playing: false,
            timeline_preview: false,
        });

        assert!((resolved.uniforms.circle_radius - 0.18).abs() < 0.001);
    }

    #[test]
    fn missing_visualizer_defaults_to_none() {
        let record = readable_record_json("one", Some("a.mp3"), &[]);
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");

        assert!(store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .is_none());
    }

    #[test]
    fn missing_visualizer_params_use_defaults() {
        let record = readable_record_json_with_visualizer(
            "one",
            Some("a.mp3"),
            &[],
            Some(r#"{"mode":"containment_lattice"}"#),
        );
        let store = ArchiveLoader::load_from_strs(MANIFEST, &[("one.json", &record)])
            .expect("archive store");
        let params = &store
            .record_by_id("one")
            .expect("record")
            .visualizer()
            .expect("visualizer")
            .params;

        assert_eq!(params.motion_rate, 1.0);
        assert_eq!(params.energy_gain, 1.1);
        assert_eq!(params.ring_count, 4);
        assert_eq!(params.particle_count, 48);
        assert_eq!(params.lattice_density, 6);
    }

    #[test]
    fn containment_lattice_label_matches_expected_copy() {
        assert_eq!(
            TrackVisualizerMode::ContainmentLattice.label(),
            "containment lattice"
        );
    }

    #[test]
    fn chromatic_bulge_grid_label_matches_expected_copy() {
        assert_eq!(
            TrackVisualizerMode::ChromaticBulgeGrid.label(),
            "chromatic bulge grid"
        );
    }
}
