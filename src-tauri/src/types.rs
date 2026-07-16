use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteStyle {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub builtin: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    Whisper,
    Llm,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub kind: ModelKind,
    pub name: String,
    pub size_bytes: u64,
    pub installed: bool,
    pub license: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationConfig {
    pub id: String,
    pub enabled: bool,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub setup_complete: bool,
    pub default_style_id: String,
    pub input_device_id: Option<String>,
    pub capture_system_audio: bool,
    pub capture_microphone: bool,
    pub integrations: Vec<IntegrationConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            setup_complete: false,
            default_style_id: "meeting".to_string(),
            input_device_id: None,
            capture_system_audio: true,
            capture_microphone: true,
            integrations: vec![
                IntegrationConfig { id: "markdown".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "obsidian".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "clipboard".into(), enabled: true, options: HashMap::new() },
                IntegrationConfig { id: "notion".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "slack".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "webhook".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "google-calendar".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "apple-calendar".into(), enabled: false, options: HashMap::new() },
                IntegrationConfig { id: "microsoft-calendar".into(), enabled: false, options: HashMap::new() },
            ],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub segments: Vec<TranscriptSegment>,
    pub text: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub style_id: String,
    pub content: String,
    pub transcript: Option<Transcript>,
    pub audio_path: Option<String>,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub preview: String,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub setup_complete: bool,
    pub models_ready: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeProgress {
    pub note_id: String,
    pub percent: f32,
    pub partial_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesToken {
    pub note_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ExportTarget {
    Markdown,
    Obsidian,
    Clipboard { format: String },
    Notion,
    Slack,
    Webhook,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub title: String,
    /// RFC3339 UTC timestamps.
    pub start: String,
    pub end: String,
    /// Provider label, e.g. "Google", "Apple", "Microsoft".
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub ok: bool,
    pub location: Option<String>,
    pub message: String,
}
