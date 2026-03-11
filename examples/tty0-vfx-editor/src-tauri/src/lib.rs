use std::{
    fs,
    path::{Path, PathBuf},
};

use open::that_detached;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;
use thiserror::Error;
use tty0_vfx_core::{
    legacy_automation_to_timeline, validate_visualizer_config, TrackVisualizerConfig,
};

#[derive(Debug, Error)]
enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Serialize)]
struct ProjectHandle {
    root: String,
    manifest_path: String,
}

#[derive(Clone, Debug, Serialize)]
struct RecordSummary {
    record_id: String,
    title: String,
    record_path: String,
    has_audio: bool,
}

#[derive(Clone, Debug, Serialize)]
struct LoadedRecord {
    record_id: String,
    record_path: String,
    record_title: String,
    audio_url: Option<String>,
    visualizer: TrackVisualizerConfig,
    raw_record_json: String,
    has_legacy_automation: bool,
}

#[derive(Debug, Deserialize)]
struct SaveVisualizerRequest {
    #[allow(dead_code)]
    record_id: String,
    visualizer_json: String,
}

#[tauri::command]
fn open_project_root(path: String) -> Result<ProjectHandle, AppError> {
    let root = PathBuf::from(path);
    validate_project_root(&root)?;
    Ok(ProjectHandle {
        root: root.display().to_string(),
        manifest_path: root
            .join("data/archive_manifest.json")
            .display()
            .to_string(),
    })
}

#[tauri::command]
fn list_records(project_root: String) -> Result<Vec<RecordSummary>, AppError> {
    let root = PathBuf::from(project_root);
    validate_project_root(&root)?;
    let records_dir = root.join("data/records");
    let mut summaries = Vec::new();
    for entry in fs::read_dir(records_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let value: Value = serde_json::from_str(&raw)?;
        summaries.push(RecordSummary {
            record_id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            title: value
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled")
                .to_string(),
            record_path: path.display().to_string(),
            has_audio: value
                .get("media_page")
                .and_then(|media| media.get("audio"))
                .and_then(Value::as_object)
                .is_some(),
        });
    }
    summaries.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    Ok(summaries)
}

#[tauri::command]
fn load_record(project_root: String, record_id: String) -> Result<LoadedRecord, AppError> {
    let root = PathBuf::from(project_root);
    validate_project_root(&root)?;
    let path = record_path(&root, &record_id)?;
    let raw = fs::read_to_string(&path)?;
    let mut value: Value = serde_json::from_str(&raw)?;
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled")
        .to_string();
    let media_page = value
        .get_mut("media_page")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::Message("record missing media_page".to_string()))?;
    let visualizer_value = media_page
        .get("visualizer")
        .cloned()
        .unwrap_or_else(default_visualizer_value);
    let mut visualizer: TrackVisualizerConfig = serde_json::from_value(visualizer_value)?;
    let has_legacy_automation = visualizer.timeline.is_none() && visualizer.automation.is_some();
    if has_legacy_automation {
        if let Some(automation) = visualizer.automation.take() {
            visualizer.timeline = Some(legacy_automation_to_timeline(&automation));
            visualizer.automation = None;
        }
    }
    validate_visualizer_config(record_id.as_str(), &visualizer)
        .map_err(|error| AppError::Message(error.to_string()))?;
    let audio_url = media_page
        .get("audio")
        .and_then(|audio| audio.get("path"))
        .and_then(Value::as_str)
        .map(|relative| root.join(relative).display().to_string());

    Ok(LoadedRecord {
        record_id,
        record_path: path.display().to_string(),
        record_title: title,
        audio_url,
        visualizer,
        raw_record_json: raw,
        has_legacy_automation,
    })
}

#[tauri::command]
fn save_record_visualizer(
    project_root: String,
    record_id: String,
    visualizer_json: String,
) -> Result<String, AppError> {
    let root = PathBuf::from(project_root);
    validate_project_root(&root)?;
    let path = record_path(&root, &record_id)?;
    let mut record_value: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    let mut visualizer: TrackVisualizerConfig = serde_json::from_str(&visualizer_json)?;
    visualizer = visualizer.with_synced_base_states().normalized_for_export();
    validate_visualizer_config(record_id.as_str(), &visualizer)
        .map_err(|error| AppError::Message(error.to_string()))?;
    let media_page = record_value
        .get_mut("media_page")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::Message("record missing media_page".to_string()))?;
    media_page.insert("visualizer".to_string(), serde_json::to_value(&visualizer)?);
    fs::write(&path, serde_json::to_string_pretty(&record_value)? + "\n")?;
    Ok(format!("saved visualizer into {}", path.display()))
}

#[tauri::command]
fn reveal_record_file(project_root: String, record_id: String) -> Result<(), AppError> {
    let root = PathBuf::from(project_root);
    let path = record_path(&root, &record_id)?;
    that_detached(path).map_err(|error| AppError::Message(error.to_string()))
}

fn validate_project_root(root: &Path) -> Result<(), AppError> {
    if !root.exists() {
        return Err(AppError::Message("project root does not exist".to_string()));
    }
    if !root.join("data/archive_manifest.json").exists() || !root.join("data/records").exists() {
        return Err(AppError::Message(
            "project root must contain data/archive_manifest.json and data/records".to_string(),
        ));
    }
    Ok(())
}

fn record_path(root: &Path, record_id: &str) -> Result<PathBuf, AppError> {
    let path = root.join("data/records").join(format!("{record_id}.json"));
    if path.exists() {
        Ok(path)
    } else {
        Err(AppError::Message(format!("record {record_id} not found")))
    }
}

fn default_visualizer_value() -> Value {
    serde_json::json!({
        "mode": "chromatic_bulge_grid",
        "params": {
            "shader_states": {
                "playing": {
                    "motion_rate": 1.0,
                    "lattice_density": 6.0,
                    "circle_radius": 0.24,
                    "circle_falloff_start": 0.78,
                    "circle_falloff_end": 1.0,
                    "bulge_amount": 0.42,
                    "rim_guard": 0.55,
                    "rim_exponent": 1.8,
                    "rim_warp": 0.18,
                    "spacing_max_px": 22.0,
                    "spacing_min_px": 12.0,
                    "dot_size": 0.16,
                    "outer_dot_scale": 0.33,
                    "edge_softness": 1.0,
                    "chromatic_aberration": 0.28,
                    "scroll_base": 28.0,
                    "scroll_motion_scale": 42.0,
                    "scroll_motion_floor": 0.2,
                    "scroll_motion_ceiling": 2.8,
                    "cold_color": [1.0, 1.0, 1.0],
                    "hot_color": [1.0, 1.0, 1.0],
                    "color_cycle_rate": 0.16,
                    "inner_alpha": 0.9
                },
                "idle": {
                    "motion_rate": 1.0,
                    "lattice_density": 6.0,
                    "circle_radius": 0.24,
                    "circle_falloff_start": 0.78,
                    "circle_falloff_end": 1.0,
                    "bulge_amount": 0.42,
                    "rim_guard": 0.55,
                    "rim_exponent": 1.8,
                    "rim_warp": 0.18,
                    "spacing_max_px": 22.0,
                    "spacing_min_px": 12.0,
                    "dot_size": 0.16,
                    "outer_dot_scale": 0.33,
                    "edge_softness": 1.0,
                    "chromatic_aberration": 0.28,
                    "scroll_base": 28.0,
                    "scroll_motion_scale": 42.0,
                    "scroll_motion_floor": 0.2,
                    "scroll_motion_ceiling": 2.8,
                    "cold_color": [1.0, 1.0, 1.0],
                    "hot_color": [1.0, 1.0, 1.0],
                    "color_cycle_rate": 0.16,
                    "inner_alpha": 0.9
                }
            }
        },
        "timeline": {
            "bpm": 132.0,
            "measures": 64,
            "beats_per_measure": 4,
            "clips": [
                {
                    "id": "clip_1",
                    "name": "Clip 1",
                    "length_beats": 4.0,
                    "color": [0.43, 0.86, 0.83],
                    "authoring": {
                        "tracks": [
                            {
                                "lane": "motion_rate",
                                "steps": []
                            }
                        ]
                    },
                    "lanes": {}
                }
            ],
            "arrangement": []
        }
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_project_root,
            list_records,
            load_record,
            save_record_visualizer,
            reveal_record_file
        ])
        .setup(|app| {
            let _ = app.handle();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tty0-vfx-editor");
}
