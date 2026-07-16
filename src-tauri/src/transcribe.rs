use crate::types::{TranscribeProgress, Transcript, TranscriptSegment};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

#[derive(Deserialize)]
struct WhisperJson {
    #[serde(default)]
    result: WhisperResult,
    #[serde(default)]
    transcription: Vec<WhisperSegment>,
}

#[derive(Deserialize, Default)]
struct WhisperResult {
    #[serde(default)]
    language: String,
}

#[derive(Deserialize)]
struct WhisperSegment {
    offsets: WhisperOffsets,
    text: String,
}

#[derive(Deserialize)]
struct WhisperOffsets {
    from: i64,
    to: i64,
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn emit_progress(app: &AppHandle, note_id: &str, percent: f32) {
    let _ = app.emit(
        "transcribe-progress",
        TranscribeProgress { note_id: note_id.to_string(), percent, partial_text: None },
    );
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn transcribe(
    app: &AppHandle,
    note_id: &str,
    model_path: &Path,
    wav_path: &Path,
) -> Result<Transcript> {
    if !model_path.exists() {
        return Err(anyhow!("Whisper model is not installed"));
    }
    let out_prefix = wav_path.with_extension("");
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).to_string();

    emit_progress(app, note_id, 1.0);

    let (mut rx, _child) = app
        .shell()
        .sidecar("whisper-cli")
        .context("whisper-cli sidecar not found — run scripts/fetch-sidecars.sh")?
        .args([
            "-m",
            &model_path.to_string_lossy(),
            "-f",
            &wav_path.to_string_lossy(),
            "-l",
            "auto",
            "-t",
            &threads,
            "-oj",
            "-of",
            &out_prefix.to_string_lossy(),
            "--print-progress",
        ])
        .spawn()
        .context("failed to start whisper-cli")?;

    let mut exit_ok = false;
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stderr(bytes) => {
                let line = String::from_utf8_lossy(&bytes);
                if let Some(pct) = parse_progress(&line) {
                    emit_progress(app, note_id, pct);
                }
            }
            CommandEvent::Terminated(payload) => {
                exit_ok = payload.code == Some(0);
            }
            _ => {}
        }
    }

    if !exit_ok {
        return Err(anyhow!("transcription process failed"));
    }

    let json_path = out_prefix.with_extension("json");
    let raw = std::fs::read_to_string(&json_path)
        .with_context(|| format!("transcript output missing at {}", json_path.display()))?;
    let transcript = parse_transcript(&raw)?;
    emit_progress(app, note_id, 100.0);
    Ok(transcript)
}

/// Convert whisper.cpp `-oj` JSON into a `Transcript`. Pure and unit-tested.
fn parse_transcript(raw: &str) -> Result<Transcript> {
    let parsed: WhisperJson = serde_json::from_str(raw).context("could not parse whisper output")?;
    let segments: Vec<TranscriptSegment> = parsed
        .transcription
        .into_iter()
        .map(|s| TranscriptSegment {
            start: s.offsets.from as f64 / 1000.0,
            end: s.offsets.to as f64 / 1000.0,
            text: s.text.trim().to_string(),
        })
        .filter(|s| !s.text.is_empty())
        .collect();
    let text = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
    Ok(Transcript {
        segments,
        text,
        language: if parsed.result.language.is_empty() { "en".into() } else { parsed.result.language },
    })
}

fn parse_progress(line: &str) -> Option<f32> {
    let idx = line.find("progress =")?;
    let rest = &line[idx + "progress =".len()..];
    let num: String = rest.chars().skip_while(|c| c.is_whitespace()).take_while(|c| c.is_ascii_digit()).collect();
    num.parse::<f32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHISPER_JSON: &str = r#"{
        "result": { "language": "en" },
        "transcription": [
            { "offsets": { "from": 0, "to": 1500 }, "text": " Hello there" },
            { "offsets": { "from": 1500, "to": 3000 }, "text": "  " },
            { "offsets": { "from": 3000, "to": 4200 }, "text": " general" }
        ]
    }"#;

    #[test]
    fn parses_segments_and_joins_text() {
        let t = parse_transcript(WHISPER_JSON).unwrap();
        assert_eq!(t.segments.len(), 2, "blank segment should be dropped");
        assert_eq!(t.segments[0].text, "Hello there");
        assert_eq!(t.segments[0].start, 0.0);
        assert_eq!(t.segments[0].end, 1.5);
        assert_eq!(t.text, "Hello there general");
        assert_eq!(t.language, "en");
    }

    #[test]
    fn defaults_language_when_missing() {
        let t = parse_transcript(r#"{"transcription":[]}"#).unwrap();
        assert_eq!(t.language, "en");
        assert!(t.text.is_empty());
    }

    #[test]
    fn errors_on_bad_json() {
        assert!(parse_transcript("not json").is_err());
    }

    #[test]
    fn parses_progress_lines() {
        assert_eq!(parse_progress("whisper_print_progress: progress =  42%"), Some(42.0));
        assert_eq!(parse_progress("progress = 7%"), Some(7.0));
        assert_eq!(parse_progress("no percentage here"), None);
    }
}
