use crate::audio::{self, Recorder};
use crate::integrations;
use crate::llm::LlmServer;
use crate::models;
use crate::state::AppState;
use crate::storage;
use crate::transcribe;
use crate::types::*;
use tauri::{AppHandle, State};

type R<T> = Result<T, String>;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn derive_title(content: &str) -> String {
    let line = content
        .lines()
        .map(|l| l.trim_start_matches(['#', '-', '*', '>', ' ']).trim())
        .find(|l| !l.is_empty())
        .unwrap_or("Untitled note");
    line.chars().take(60).collect()
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_app_status(state: State<'_, AppState>) -> AppStatus {
    let setup_complete = state.settings.lock().unwrap().setup_complete;
    AppStatus {
        setup_complete,
        models_ready: models::all_installed(&state.models_dir()),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn update_settings(state: State<'_, AppState>, settings: Settings) -> R<Settings> {
    storage::save_settings(&state.base_dir, &settings).map_err(err)?;
    *state.settings.lock().unwrap() = settings.clone();
    Ok(settings)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn list_audio_inputs() -> Vec<AudioDevice> {
    audio::list_inputs()
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_models(state: State<'_, AppState>) -> Vec<ModelInfo> {
    models::model_infos(&state.models_dir())
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn download_models(
    app: AppHandle,
    state: State<'_, AppState>,
    model_ids: Vec<String>,
) -> R<()> {
    let dir = state.models_dir();
    models::download(&app, &dir, model_ids).await.map_err(err)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_note_styles(state: State<'_, AppState>) -> Vec<NoteStyle> {
    storage::load_styles(&state.base_dir)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn save_note_style(state: State<'_, AppState>, style: NoteStyle) -> R<NoteStyle> {
    storage::save_style(&state.base_dir, &style).map_err(err)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn delete_note_style(state: State<'_, AppState>, id: String) -> R<()> {
    storage::delete_style(&state.base_dir, &id).map_err(err)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn create_note(state: State<'_, AppState>, title: String) -> R<Note> {
    let ts = now();
    let style_id = state.settings.lock().unwrap().default_style_id.clone();
    let note = Note {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        created_at: ts.clone(),
        updated_at: ts,
        style_id,
        content: String::new(),
        transcript: None,
        audio_path: None,
        duration_secs: 0.0,
    };
    storage::save_note(&state.base_dir, &note).map_err(err)?;
    Ok(note)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    device_id: Option<String>,
) -> R<()> {
    let mut settings = state.settings.lock().unwrap().clone();
    if device_id.is_some() {
        settings.input_device_id = device_id;
    }
    let recorder = Recorder::start(&app, &settings).map_err(err)?;
    *state.recorder.lock().unwrap() = Some(recorder);
    *state.recording_note.lock().unwrap() = Some(note_id);
    Ok(())
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn stop_recording(state: State<'_, AppState>) -> R<f64> {
    let recorder = state.recorder.lock().unwrap().take();
    let note_id = state.recording_note.lock().unwrap().take();
    let (recorder, note_id) = match (recorder, note_id) {
        (Some(r), Some(n)) => (r, n),
        _ => return Err("not recording".into()),
    };
    let wav = storage::audio_path(&state.base_dir, &note_id);
    let duration = recorder.stop(&wav).map_err(err)?;

    if let Ok(mut note) = storage::load_note(&state.base_dir, &note_id) {
        note.audio_path = Some(wav.to_string_lossy().to_string());
        note.duration_secs = duration;
        note.updated_at = now();
        let _ = storage::save_note(&state.base_dir, &note);
    }
    Ok(duration)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn cancel_recording(state: State<'_, AppState>) -> R<()> {
    if let Some(recorder) = state.recorder.lock().unwrap().take() {
        recorder.cancel();
    }
    let _ = state.recording_note.lock().unwrap().take();
    Ok(())
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn transcribe(app: AppHandle, state: State<'_, AppState>, note_id: String) -> R<Transcript> {
    let base = state.base_dir.clone();
    let wav = storage::audio_path(&base, &note_id);
    let model = models::whisper_path(&state.models_dir());
    let transcript = transcribe::transcribe(&app, &note_id, &model, &wav).await.map_err(err)?;

    if let Ok(mut note) = storage::load_note(&base, &note_id) {
        note.transcript = Some(transcript.clone());
        note.updated_at = now();
        let _ = storage::save_note(&base, &note);
    }
    Ok(transcript)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn generate_notes(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    style_id: String,
) -> R<Note> {
    let base = state.base_dir.clone();
    let mut note = storage::load_note(&base, &note_id).map_err(err)?;
    let transcript = note.transcript.as_ref().map(|t| t.text.clone()).unwrap_or_default();
    if transcript.trim().is_empty() {
        return Err("No transcript to summarise yet".into());
    }
    let style = storage::get_style(&base, &style_id).ok_or_else(|| "unknown note style".to_string())?;
    let prompt = if style.prompt.contains("{transcript}") {
        style.prompt.replace("{transcript}", &transcript)
    } else {
        format!("{}\n\n{}", style.prompt, transcript)
    };

    let model = models::llm_path(&state.models_dir());
    let mut guard = state.llm.lock().await;
    if guard.is_none() {
        let server = LlmServer::start(&app, &model).await.map_err(err)?;
        *guard = Some(server);
    }
    let content = guard.as_ref().unwrap().generate(&app, &note_id, &prompt).await.map_err(err)?;
    drop(guard);

    note.content = content;
    note.style_id = style_id;
    note.updated_at = now();
    if note.title.trim().is_empty() || note.title == "New recording" {
        note.title = derive_title(&note.content);
    }
    storage::save_note(&base, &note).map_err(err)?;
    Ok(note)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn list_notes(state: State<'_, AppState>) -> R<Vec<NoteSummary>> {
    storage::list_notes(&state.base_dir).map_err(err)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_note(state: State<'_, AppState>, id: String) -> R<Note> {
    storage::load_note(&state.base_dir, &id).map_err(err)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn save_note(state: State<'_, AppState>, note: Note) -> R<Note> {
    let mut note = note;
    note.updated_at = now();
    storage::save_note(&state.base_dir, &note).map_err(err)?;
    Ok(note)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn delete_note(state: State<'_, AppState>, id: String) -> R<()> {
    storage::delete_note(&state.base_dir, &id).map_err(err)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn export_note(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    target: ExportTarget,
) -> R<ExportResult> {
    let base = state.base_dir.clone();
    let note = storage::load_note(&base, &id).map_err(err)?;
    let settings = state.settings.lock().unwrap().clone();
    Ok(integrations::export(&app, &base, &settings, &note, target).await)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn get_calendar_events(state: State<'_, AppState>) -> R<Vec<CalendarEvent>> {
    let settings = state.settings.lock().unwrap().clone();
    Ok(crate::calendar::list_events(&settings).await)
}

#[tauri::command]
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn suggested_note_title(state: State<'_, AppState>) -> R<String> {
    let settings = state.settings.lock().unwrap().clone();
    let events = crate::calendar::list_events(&settings).await;
    Ok(crate::calendar::current_or_next(&events, chrono::Utc::now())
        .map(|e| e.title.clone())
        .unwrap_or_else(|| "New recording".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{derive_title, err, now};

    #[test]
    fn err_stringifies() {
        assert_eq!(err("boom"), "boom");
    }

    #[test]
    fn derive_title_from_heading() {
        assert_eq!(derive_title("# Meeting notes\n\nbody"), "Meeting notes");
    }

    #[test]
    fn derive_title_from_first_text_line() {
        assert_eq!(derive_title("\n\nFirst real line\nmore"), "First real line");
    }

    #[test]
    fn derive_title_empty_is_untitled() {
        assert_eq!(derive_title("   \n  "), "Untitled note");
    }

    #[test]
    fn derive_title_truncates_to_60() {
        let long = "a".repeat(100);
        assert_eq!(derive_title(&long).len(), 60);
    }

    #[test]
    fn now_is_rfc3339() {
        assert!(chrono::DateTime::parse_from_rfc3339(&now()).is_ok());
    }
}
