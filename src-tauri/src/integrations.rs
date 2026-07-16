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

fn export_markdown(settings: &Settings, base: &std::path::Path, note: &Note) -> Result<ExportResult> {
    let folder = integration(settings, "markdown")
        .and_then(|c| c.options.get("folder"))
        .filter(|f| !f.trim().is_empty())
        .map(|f| expand_home(f))
        .unwrap_or_else(|| base.join("exports"));
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
        ExportTarget::Clipboard { format } => export_clipboard(app, note, &format),
        ExportTarget::Notion => export_notion(settings, note).await,
        ExportTarget::Calendar => Err(anyhow!("Calendar export is not available yet")),
    };
    match result {
        Ok(r) => r,
        Err(e) => ExportResult { ok: false, location: None, message: e.to_string() },
    }
}
