use crate::types::{ExportResult, ExportTarget, IntegrationConfig, Note, Settings};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

fn integration<'a>(settings: &'a Settings, id: &str) -> Option<&'a IntegrationConfig> {
    settings.integrations.iter().find(|c| c.id == id)
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '-' })
        .collect();
    let trimmed = cleaned.trim().replace(' ', "-");
    if trimmed.is_empty() { "note".to_string() } else { trimmed }
}

fn markdown_document(note: &Note) -> String {
    format!(
        "# {}\n\n_{}_\n\n{}\n",
        if note.title.is_empty() { "Untitled" } else { &note.title },
        note.created_at,
        note.content
    )
}

fn write_markdown(folder: PathBuf, note: &Note) -> Result<ExportResult> {
    std::fs::create_dir_all(&folder)?;
    let date = note.created_at.split('T').next().unwrap_or("note");
    let file = folder.join(format!("{}-{}.md", sanitize(&note.title), date));
    std::fs::write(&file, markdown_document(note))?;
    Ok(ExportResult {
        ok: true,
        location: Some(file.to_string_lossy().to_string()),
        message: format!("Saved to {}", file.display()),
    })
}

fn export_markdown(settings: &Settings, base: &std::path::Path, note: &Note) -> Result<ExportResult> {
    let folder = integration(settings, "markdown")
        .and_then(|c| c.options.get("folder"))
        .filter(|f| !f.trim().is_empty())
        .map(|f| expand_home(f))
        .unwrap_or_else(|| base.join("exports"));
    write_markdown(folder, note)
}

fn export_obsidian(settings: &Settings, note: &Note) -> Result<ExportResult> {
    let folder = integration(settings, "obsidian")
        .and_then(|c| c.options.get("folder"))
        .filter(|f| !f.trim().is_empty())
        .map(|f| expand_home(f))
        .ok_or_else(|| anyhow!("Set your Obsidian vault folder in Settings"))?;
    write_markdown(folder, note)
}

async fn export_slack(settings: &Settings, note: &Note) -> Result<ExportResult> {
    let cfg = integration(settings, "slack").ok_or_else(|| anyhow!("Slack not configured"))?;
    let webhook = cfg.options.get("webhook").filter(|w| !w.trim().is_empty())
        .ok_or_else(|| anyhow!("Add your Slack incoming webhook URL in Settings"))?;
    let title = if note.title.is_empty() { "Untitled" } else { &note.title };
    let text = format!("*{title}*\n\n{}", note.content);
    let resp = reqwest::Client::new()
        .post(webhook)
        .json(&json!({ "text": text }))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Slack returned {}", resp.status()));
    }
    Ok(ExportResult { ok: true, location: None, message: "Posted to Slack".into() })
}

async fn export_webhook(settings: &Settings, note: &Note) -> Result<ExportResult> {
    let cfg = integration(settings, "webhook").ok_or_else(|| anyhow!("Webhook not configured"))?;
    let url = cfg.options.get("url").filter(|u| !u.trim().is_empty())
        .ok_or_else(|| anyhow!("Add a webhook URL in Settings"))?;
    let resp = reqwest::Client::new()
        .post(url)
        .json(&json!({
            "id": note.id,
            "title": note.title,
            "createdAt": note.created_at,
            "content": note.content,
            "durationSecs": note.duration_secs
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Webhook returned {}", resp.status()));
    }
    Ok(ExportResult { ok: true, location: None, message: "Sent to webhook".into() })
}

fn export_clipboard(app: &AppHandle, note: &Note, format: &str) -> Result<ExportResult> {
    let text = if format == "plain" {
        format!("{}\n\n{}", note.title, note.content)
    } else {
        markdown_document(note)
    };
    app.clipboard().write_text(text).map_err(|e| anyhow!(e.to_string()))?;
    Ok(ExportResult { ok: true, location: None, message: "Copied to clipboard".into() })
}

async fn export_notion(settings: &Settings, note: &Note) -> Result<ExportResult> {
    let cfg = integration(settings, "notion").ok_or_else(|| anyhow!("Notion not configured"))?;
    let token = cfg.options.get("token").filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow!("Add your Notion token in Settings"))?;
    let database_id = cfg.options.get("database").filter(|d| !d.is_empty())
        .ok_or_else(|| anyhow!("Add a Notion database ID in Settings"))?;

    let client = reqwest::Client::new();
    let version = "2022-06-28";

    let db: Value = client
        .get(format!("https://api.notion.com/v1/databases/{database_id}"))
        .bearer_auth(token)
        .header("Notion-Version", version)
        .send()
        .await?
        .json()
        .await?;
    let title_prop = db["properties"]
        .as_object()
        .and_then(|props| props.iter().find(|(_, v)| v["type"] == "title").map(|(k, _)| k.clone()))
        .unwrap_or_else(|| "Name".to_string());

    let children: Vec<Value> = note
        .content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(100)
        .map(|l| {
            let content: String = l.chars().take(2000).collect();
            json!({
                "object": "block",
                "type": "paragraph",
                "paragraph": { "rich_text": [{ "type": "text", "text": { "content": content } }] }
            })
        })
        .collect();

    let body = json!({
        "parent": { "database_id": database_id },
        "properties": { title_prop: { "title": [{ "text": { "content": note.title } }] } },
        "children": children
    });

    let resp = client
        .post("https://api.notion.com/v1/pages")
        .bearer_auth(token)
        .header("Notion-Version", version)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        return Err(anyhow!("Notion API returned {status}"));
    }
    let page: Value = resp.json().await?;
    let url = page["url"].as_str().map(|s| s.to_string());
    Ok(ExportResult { ok: true, location: url.clone(), message: "Sent to Notion".into() })
}

pub async fn export(
    app: &AppHandle,
    base: &std::path::Path,
    settings: &Settings,
    note: &Note,
    target: ExportTarget,
) -> ExportResult {
    let result = match target {
        ExportTarget::Markdown => export_markdown(settings, base, note),
        ExportTarget::Obsidian => export_obsidian(settings, note),
        ExportTarget::Clipboard { format } => export_clipboard(app, note, &format),
        ExportTarget::Notion => export_notion(settings, note).await,
        ExportTarget::Slack => export_slack(settings, note).await,
        ExportTarget::Webhook => export_webhook(settings, note).await,
    };
    match result {
        Ok(r) => r,
        Err(e) => ExportResult { ok: false, location: None, message: e.to_string() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Settings;

    fn note() -> Note {
        Note {
            id: "n1".into(),
            title: "My Meeting".into(),
            created_at: "2026-07-16T09:30:00Z".into(),
            updated_at: "2026-07-16T09:30:00Z".into(),
            style_id: "meeting".into(),
            content: "## Summary\nWe shipped it.".into(),
            transcript: None,
            audio_path: None,
            duration_secs: 60.0,
        }
    }

    fn set_folder(settings: &mut Settings, id: &str, folder: &str) {
        let cfg = settings.integrations.iter_mut().find(|c| c.id == id).unwrap();
        cfg.enabled = true;
        cfg.options.insert("folder".into(), folder.into());
    }

    #[test]
    fn sanitize_strips_unsafe_chars() {
        assert_eq!(sanitize("Team: sync / weekly"), "Team--sync---weekly");
        assert_eq!(sanitize("   "), "note");
        assert_eq!(sanitize("Clean Name"), "Clean-Name");
    }

    #[test]
    fn expand_home_handles_tilde_and_plain() {
        assert_eq!(expand_home("/abs/path"), PathBuf::from("/abs/path"));
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap();
        assert_eq!(expand_home("~/notes"), PathBuf::from(home).join("notes"));
    }

    #[test]
    fn markdown_document_has_title_and_body() {
        let doc = markdown_document(&note());
        assert!(doc.contains("# My Meeting"));
        assert!(doc.contains("We shipped it."));
    }

    #[test]
    fn markdown_export_writes_to_configured_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        set_folder(&mut settings, "markdown", dir.path().to_str().unwrap());
        let res = export_markdown(&settings, dir.path(), &note()).unwrap();
        assert!(res.ok);
        let contents = std::fs::read_to_string(res.location.unwrap()).unwrap();
        assert!(contents.contains("# My Meeting"));
    }

    #[test]
    fn markdown_export_falls_back_to_base_exports() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::default();
        let res = export_markdown(&settings, dir.path(), &note()).unwrap();
        assert!(res.location.unwrap().starts_with(dir.path().join("exports").to_str().unwrap()));
    }

    #[test]
    fn obsidian_requires_folder() {
        let settings = Settings::default();
        assert!(export_obsidian(&settings, &note()).is_err());

        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        set_folder(&mut settings, "obsidian", dir.path().to_str().unwrap());
        assert!(export_obsidian(&settings, &note()).unwrap().ok);
    }
}
