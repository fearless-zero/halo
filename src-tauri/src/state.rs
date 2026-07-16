use crate::audio::Recorder;
use crate::llm::LlmServer;
use crate::storage;
use crate::types::Settings;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub base_dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub recorder: Mutex<Option<Recorder>>,
    pub recording_note: Mutex<Option<String>>,
    pub llm: tokio::sync::Mutex<Option<LlmServer>>,
}

impl AppState {
    pub fn new(base_dir: PathBuf) -> Self {
        let settings = storage::load_settings(&base_dir);
        AppState {
            settings: Mutex::new(settings),
            recorder: Mutex::new(None),
            recording_note: Mutex::new(None),
            llm: tokio::sync::Mutex::new(None),
            base_dir,
        }
    }

    pub fn models_dir(&self) -> PathBuf {
        storage::models_dir(&self.base_dir)
    }
}
