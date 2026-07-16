use crate::types::{Note, NoteStyle, NoteSummary, Settings};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn ensure_dirs(base: &Path) -> Result<()> {
    fs::create_dir_all(base.join("notes"))?;
    fs::create_dir_all(base.join("audio"))?;
    fs::create_dir_all(base.join("models"))?;
    Ok(())
}

pub fn models_dir(base: &Path) -> PathBuf {
    base.join("models")
}

pub fn audio_path(base: &Path, id: &str) -> PathBuf {
    base.join("audio").join(format!("{id}.wav"))
}

fn note_path(base: &Path, id: &str) -> PathBuf {
    base.join("notes").join(format!("{id}.json"))
}

// ---------- Settings ----------

pub fn load_settings(base: &Path) -> Settings {
    let path = base.join("settings.json");
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(base: &Path, settings: &Settings) -> Result<()> {
    let path = base.join("settings.json");
    fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

// ---------- Note styles ----------

pub fn builtin_styles() -> Vec<NoteStyle> {
    let styles = [
        (
            "meeting",
            "Meeting",
            "Summary, decisions and action items",
            "You are an expert meeting note-taker. From the transcript below, write concise, well-structured notes in Markdown with these sections:\n\n## Summary\nA 2-3 sentence overview.\n\n## Key Points\nBullet points of what was discussed.\n\n## Decisions\nDecisions that were made.\n\n## Action Items\n- [ ] Owner — task\n\nOnly use information present in the transcript. Do not invent details.\n\nTranscript:\n{transcript}",
        ),
        (
            "lecture",
            "Lecture",
            "Structured study notes with key concepts",
            "You are a diligent student. Turn the lecture transcript below into clear study notes in Markdown:\n\n## Topic\n## Key Concepts\nExplain each concept in your own words with bullet points.\n## Important Definitions\n## Questions to Review\n\nStay faithful to the transcript.\n\nTranscript:\n{transcript}",
        ),
        (
            "interview",
            "Interview",
            "Candidate highlights and evaluation",
            "Summarise the interview transcript below in Markdown:\n\n## Overview\n## Strengths\n## Concerns\n## Notable Quotes\n## Recommendation\n\nBase everything strictly on the transcript.\n\nTranscript:\n{transcript}",
        ),
        (
            "standup",
            "Standup",
            "Per-person yesterday / today / blockers",
            "Convert the standup transcript below into Markdown grouped per person:\n\nFor each person mentioned:\n### Name\n- Yesterday:\n- Today:\n- Blockers:\n\nTranscript:\n{transcript}",
        ),
        (
            "personal",
            "Personal",
            "Plain, faithful summary of the conversation",
            "Write clear, friendly notes in Markdown summarising the conversation below. Capture the main points, anything to remember, and any follow-ups. Keep it faithful to what was said.\n\nTranscript:\n{transcript}",
        ),
    ];
    styles
        .into_iter()
        .map(|(id, name, desc, prompt)| NoteStyle {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            prompt: prompt.to_string(),
            builtin: true,
        })
        .collect()
}

fn custom_styles_path(base: &Path) -> PathBuf {
    base.join("styles.json")
}

fn load_custom_styles(base: &Path) -> Vec<NoteStyle> {
    fs::read_to_string(custom_styles_path(base))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_custom_styles(base: &Path, styles: &[NoteStyle]) -> Result<()> {
    fs::write(custom_styles_path(base), serde_json::to_string_pretty(styles)?)?;
    Ok(())
}

pub fn load_styles(base: &Path) -> Vec<NoteStyle> {
    let mut styles = builtin_styles();
    let custom = load_custom_styles(base);
    for c in custom {
        if let Some(existing) = styles.iter_mut().find(|s| s.id == c.id) {
            *existing = c;
        } else {
            styles.push(c);
        }
    }
    styles
}

pub fn save_style(base: &Path, style: &NoteStyle) -> Result<NoteStyle> {
    let mut custom = load_custom_styles(base);
    let mut stored = style.clone();
    // Builtin styles remain flagged as builtin even when their prompt is edited.
    stored.builtin = builtin_styles().iter().any(|s| s.id == stored.id);
    if let Some(existing) = custom.iter_mut().find(|s| s.id == stored.id) {
        *existing = stored.clone();
    } else {
        custom.push(stored.clone());
    }
    save_custom_styles(base, &custom)?;
    Ok(stored)
}

pub fn delete_style(base: &Path, id: &str) -> Result<()> {
    let custom: Vec<NoteStyle> = load_custom_styles(base).into_iter().filter(|s| s.id != id).collect();
    save_custom_styles(base, &custom)?;
    Ok(())
}

pub fn get_style(base: &Path, id: &str) -> Option<NoteStyle> {
    load_styles(base).into_iter().find(|s| s.id == id)
}

// ---------- Notes ----------

pub fn save_note(base: &Path, note: &Note) -> Result<()> {
    fs::write(note_path(base, &note.id), serde_json::to_string_pretty(note)?)?;
    Ok(())
}

pub fn load_note(base: &Path, id: &str) -> Result<Note> {
    let raw = fs::read_to_string(note_path(base, id))
        .with_context(|| format!("note {id} not found"))?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn delete_note(base: &Path, id: &str) -> Result<()> {
    let _ = fs::remove_file(note_path(base, id));
    let _ = fs::remove_file(audio_path(base, id));
    Ok(())
}

fn preview_of(content: &str) -> String {
    let flat: String = content
        .lines()
        .map(|l| l.trim_start_matches(['#', '-', '*', '>', ' ']))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    flat.chars().take(140).collect()
}

pub fn list_notes(base: &Path) -> Result<Vec<NoteSummary>> {
    let dir = base.join("notes");
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(note) = serde_json::from_str::<Note>(&raw) {
                    out.push(NoteSummary {
                        id: note.id.clone(),
                        title: note.title.clone(),
                        created_at: note.created_at.clone(),
                        updated_at: note.updated_at.clone(),
                        preview: preview_of(&note.content),
                        duration_secs: note.duration_secs,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Note, NoteStyle};

    fn base() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        ensure_dirs(dir.path()).unwrap();
        dir
    }

    fn sample_note(id: &str, content: &str) -> Note {
        Note {
            id: id.to_string(),
            title: "T".into(),
            created_at: "2026-07-16T10:00:00Z".into(),
            updated_at: "2026-07-16T10:00:00Z".into(),
            style_id: "meeting".into(),
            content: content.into(),
            transcript: None,
            audio_path: None,
            duration_secs: 12.0,
        }
    }

    #[test]
    fn settings_default_when_missing_then_roundtrip() {
        let d = base();
        assert!(!load_settings(d.path()).setup_complete);
        let mut s = load_settings(d.path());
        s.setup_complete = true;
        s.default_style_id = "lecture".into();
        save_settings(d.path(), &s).unwrap();
        let loaded = load_settings(d.path());
        assert!(loaded.setup_complete);
        assert_eq!(loaded.default_style_id, "lecture");
    }

    #[test]
    fn builtin_styles_present() {
        let styles = builtin_styles();
        assert_eq!(styles.len(), 5);
        assert!(styles.iter().all(|s| s.builtin));
        assert!(styles.iter().any(|s| s.id == "meeting"));
    }

    #[test]
    fn custom_style_crud_and_builtin_flag() {
        let d = base();
        let custom = NoteStyle {
            id: "mine".into(),
            name: "Mine".into(),
            description: "d".into(),
            prompt: "p {transcript}".into(),
            builtin: false,
        };
        let saved = save_style(d.path(), &custom).unwrap();
        assert!(!saved.builtin);
        // Saving the same id again updates the existing custom entry in place.
        let mut renamed = custom.clone();
        renamed.name = "Renamed".into();
        save_style(d.path(), &renamed).unwrap();
        assert_eq!(get_style(d.path(), "mine").unwrap().name, "Renamed");
        assert!(load_styles(d.path()).iter().any(|s| s.id == "mine"));

        // Editing a builtin keeps it flagged builtin.
        let mut edited = get_style(d.path(), "meeting").unwrap();
        edited.prompt = "changed".into();
        assert!(save_style(d.path(), &edited).unwrap().builtin);

        delete_style(d.path(), "mine").unwrap();
        assert!(get_style(d.path(), "mine").is_none());
    }

    #[test]
    fn note_crud_and_listing() {
        let d = base();
        assert!(list_notes(d.path()).unwrap().is_empty());

        save_note(d.path(), &sample_note("a", "# Heading\n- point one")).unwrap();
        save_note(d.path(), &sample_note("b", "plain body")).unwrap();

        let list = list_notes(d.path()).unwrap();
        assert_eq!(list.len(), 2);
        let a = list.iter().find(|n| n.id == "a").unwrap();
        assert!(a.preview.contains("Heading"));
        assert!(!a.preview.contains('#'));

        assert_eq!(load_note(d.path(), "b").unwrap().content, "plain body");

        delete_note(d.path(), "a").unwrap();
        assert!(load_note(d.path(), "a").is_err());
        assert_eq!(list_notes(d.path()).unwrap().len(), 1);
    }

    #[test]
    fn list_notes_on_missing_dir_is_empty() {
        let d = tempfile::tempdir().unwrap(); // no ensure_dirs: notes/ does not exist
        assert!(list_notes(d.path()).unwrap().is_empty());
    }

    #[test]
    fn list_notes_ignores_non_json_and_corrupt_files() {
        let d = base();
        save_note(d.path(), &sample_note("good", "body")).unwrap();
        std::fs::write(d.path().join("notes").join("readme.txt"), "not a note").unwrap();
        std::fs::write(d.path().join("notes").join("broken.json"), "{ not valid json").unwrap();
        // A directory with a .json name passes the extension check but fails to read.
        std::fs::create_dir(d.path().join("notes").join("adir.json")).unwrap();
        let list = list_notes(d.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "good");
    }

    #[test]
    fn malformed_files_fall_back_to_defaults() {
        let d = base();
        std::fs::write(d.path().join("settings.json"), "{ broken").unwrap();
        assert!(!load_settings(d.path()).setup_complete);

        std::fs::write(custom_styles_path(d.path()), "{ broken").unwrap();
        assert!(load_styles(d.path()).iter().any(|s| s.id == "meeting"));
    }
}
