use std::collections::{HashMap, HashSet};

use serde::Deserialize;
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

#[derive(Clone, Debug, Deserialize)]
pub struct AudioArtifact {
    pub path: String,
    pub title: String,
    pub duration_hint: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TrackVisualizerConfig {
    pub mode: TrackVisualizerMode,
    #[serde(default)]
    pub params: TrackVisualizerParams,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackVisualizerMode {
    DiplomaticSignalBloom,
    HexWalkerRelay,
    ContainmentLattice,
}

#[derive(Clone, Debug, Deserialize)]
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
        }
    }
}

fn default_motion_rate() -> f32 {
    1.0
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

#[cfg(test)]
mod tests {
    use super::{ArchiveLoadError, ArchiveLoader, MediaHealth, TrackVisualizerMode};

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
}
