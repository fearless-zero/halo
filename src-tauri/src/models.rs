use crate::types::{ModelDownloadProgress, ModelInfo, ModelKind};
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

pub struct ModelSpec {
    pub id: &'static str,
    pub kind: ModelKind,
    pub name: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub approx_size: u64,
    pub license: &'static str,
}

/// The two fixed, fully-open models Halo ships with. No user selection: the
/// note writer is Qwen3-4B-Instruct (Apache-2.0) and transcription is Whisper
/// base (MIT). Both run entirely on-device.
pub fn specs() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: "whisper-base",
            kind: ModelKind::Whisper,
            name: "Whisper Base (multilingual)",
            file_name: "ggml-base.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            approx_size: 147_951_465,
            license: "MIT",
        },
        ModelSpec {
            id: "qwen3-4b",
            kind: ModelKind::Llm,
            name: "Qwen3-4B Instruct",
            file_name: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
            url: "https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
            approx_size: 2_497_000_000,
            license: "Apache-2.0",
        },
    ]
}

fn spec_by_id(id: &str) -> Option<ModelSpec> {
    specs().into_iter().find(|s| s.id == id)
}

/// A download counts as installed only if it reached ~90% of the expected
/// size, guarding against truncated downloads.
fn size_ok(actual: u64, expected: u64) -> bool {
    actual as f64 >= expected as f64 * 0.9
}

fn is_installed(spec: &ModelSpec, models_dir: &Path) -> bool {
    let path = models_dir.join(spec.file_name);
    match std::fs::metadata(&path) {
        Ok(meta) => size_ok(meta.len(), spec.approx_size),
        Err(_) => false,
    }
}

pub fn whisper_path(models_dir: &Path) -> PathBuf {
    models_dir.join("ggml-base.bin")
}

pub fn llm_path(models_dir: &Path) -> PathBuf {
    models_dir.join("Qwen3-4B-Instruct-2507-Q4_K_M.gguf")
}

pub fn model_infos(models_dir: &Path) -> Vec<ModelInfo> {
    specs()
        .iter()
        .map(|s| ModelInfo {
            id: s.id.to_string(),
            kind: s.kind,
            name: s.name.to_string(),
            size_bytes: s.approx_size,
            installed: is_installed(s, models_dir),
            license: s.license.to_string(),
        })
        .collect()
}

pub fn all_installed(models_dir: &Path) -> bool {
    specs().iter().all(|s| is_installed(s, models_dir))
}

async fn download_one(app: &AppHandle, spec: &ModelSpec, models_dir: &Path) -> Result<()> {
    let final_path = models_dir.join(spec.file_name);
    let part_path = models_dir.join(format!("{}.part", spec.file_name));

    let resp = reqwest::get(spec.url).await?;
    if !resp.status().is_success() {
        return Err(anyhow!("download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(spec.approx_size);
    let mut file = tokio::fs::File::create(&part_path).await?;
    let mut downloaded: u64 = 0;
    let mut last_pct: i64 = -1;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        let pct = ((downloaded as f64 / total.max(1) as f64) * 100.0) as i64;
        if pct != last_pct {
            last_pct = pct;
            let _ = app.emit(
                "model-download-progress",
                ModelDownloadProgress {
                    model_id: spec.id.to_string(),
                    downloaded_bytes: downloaded,
                    total_bytes: total,
                    done: false,
                    error: None,
                },
            );
        }
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(&part_path, &final_path).await?;

    let _ = app.emit(
        "model-download-progress",
        ModelDownloadProgress {
            model_id: spec.id.to_string(),
            downloaded_bytes: downloaded,
            total_bytes: total,
            done: true,
            error: None,
        },
    );
    Ok(())
}

pub async fn download(app: &AppHandle, models_dir: &Path, ids: Vec<String>) -> Result<()> {
    std::fs::create_dir_all(models_dir)?;
    for id in ids {
        let Some(spec) = spec_by_id(&id) else { continue };
        if is_installed(&spec, models_dir) {
            continue;
        }
        if let Err(e) = download_one(app, &spec, models_dir).await {
            let _ = app.emit(
                "model-download-progress",
                ModelDownloadProgress {
                    model_id: spec.id.to_string(),
                    downloaded_bytes: 0,
                    total_bytes: spec.approx_size,
                    done: false,
                    error: Some(e.to_string()),
                },
            );
            return Err(e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_two_fully_open_models() {
        let s = specs();
        assert_eq!(s.len(), 2);
        assert!(s.iter().any(|m| m.id == "whisper-base" && m.license == "MIT"));
        assert!(s.iter().any(|m| m.id == "qwen3-4b" && m.license == "Apache-2.0"));
    }

    #[test]
    fn spec_lookup() {
        assert!(spec_by_id("qwen3-4b").is_some());
        assert!(spec_by_id("nope").is_none());
    }

    #[test]
    fn model_paths_use_expected_filenames() {
        let dir = Path::new("/models");
        assert!(whisper_path(dir).ends_with("ggml-base.bin"));
        assert!(llm_path(dir).ends_with("Qwen3-4B-Instruct-2507-Q4_K_M.gguf"));
    }

    #[test]
    fn size_threshold() {
        assert!(size_ok(90, 100));
        assert!(size_ok(100, 100));
        assert!(!size_ok(89, 100));
    }

    #[test]
    fn nothing_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!all_installed(dir.path()));
        let infos = model_infos(dir.path());
        assert_eq!(infos.len(), 2);
        assert!(infos.iter().all(|m| !m.installed));
    }

    #[test]
    fn truncated_file_is_not_installed() {
        // Exercises the metadata-present branch without writing a ~150MB file:
        // a small (truncated) file must not count as installed.
        let dir = tempfile::tempdir().unwrap();
        let spec = spec_by_id("whisper-base").unwrap();
        std::fs::write(dir.path().join(spec.file_name), b"partial").unwrap();
        assert!(!is_installed(&spec, dir.path()));
    }
}
