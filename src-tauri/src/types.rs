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

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub setup_complete: bool,
    pub default_style_id: String,
    pub input_device_id: Option<String>,
    pub capture_system_audio: bool,
    pub capture_microphone: bool,
    /// When online, enrich generated notes with web research (Wikipedia).
    #[serde(default = "default_true")]
    pub web_research: bool,
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
            web_research: true,
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
    /// Web-research findings folded into the note (empty when offline/disabled).
    #[serde(default)]
    pub research: Vec<ResearchFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchFinding {
    pub title: String,
    pub summary: String,
    pub url: String,
    pub source: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = Settings::default();
        assert!(!s.setup_complete);
        assert!(s.capture_system_audio);
        assert!(s.capture_microphone);
        assert_eq!(s.default_style_id, "meeting");
        for id in ["markdown", "slack", "notion", "google-calendar", "apple-calendar", "microsoft-calendar"] {
            assert!(s.integrations.iter().any(|c| c.id == id), "missing {id}");
        }
    }

    #[test]
    fn settings_serde_roundtrip_is_camel_case() {
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("setupComplete"));
        assert!(json.contains("captureSystemAudio"));
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.default_style_id, "meeting");
    }

    #[test]
    fn export_target_deserializes_by_kind() {
        let clip: ExportTarget = serde_json::from_str(r#"{"kind":"clipboard","format":"plain"}"#).unwrap();
        assert!(matches!(clip, ExportTarget::Clipboard { format } if format == "plain"));
        assert!(matches!(serde_json::from_str::<ExportTarget>(r#"{"kind":"markdown"}"#).unwrap(), ExportTarget::Markdown));
        assert!(matches!(serde_json::from_str::<ExportTarget>(r#"{"kind":"slack"}"#).unwrap(), ExportTarget::Slack));
    }

    #[test]
    fn web_research_defaults_true_when_absent() {
        // Old settings files predate the field; it must default on.
        let json = r#"{"setupComplete":false,"defaultStyleId":"meeting","inputDeviceId":null,"captureSystemAudio":true,"captureMicrophone":true,"integrations":[]}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.web_research);
        assert!(Settings::default().web_research);
    }

    #[test]
    fn note_research_defaults_empty_and_roundtrips() {
        let json = r#"{"id":"n","title":"t","createdAt":"","updatedAt":"","styleId":"meeting","content":"","transcript":null,"audioPath":null,"durationSecs":0.0}"#;
        let note: Note = serde_json::from_str(json).unwrap();
        assert!(note.research.is_empty());

        let finding = ResearchFinding {
            title: "Photosynthesis".into(),
            summary: "A process".into(),
            url: "https://en.wikipedia.org/wiki/Photosynthesis".into(),
            source: "Wikipedia".into(),
        };
        let back: ResearchFinding =
            serde_json::from_str(&serde_json::to_string(&finding).unwrap()).unwrap();
        assert_eq!(back, finding);
    }

    #[test]
    fn model_kind_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&ModelKind::Whisper).unwrap(), "\"whisper\"");
        assert_eq!(serde_json::to_string(&ModelKind::Llm).unwrap(), "\"llm\"");
    }
}
