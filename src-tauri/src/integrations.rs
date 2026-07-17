use crate::types::{ExportResult, ExportTarget, IntegrationConfig, Note, Settings};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
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

// Writes to the OS clipboard via the plugin; excluded from coverage because it
// touches the real system clipboard (no display in CI).
#[cfg_attr(coverage_nightly, coverage(off))]
fn export_clipboard<R: Runtime>(app: &AppHandle<R>, note: &Note, format: &str) -> Result<ExportResult> {
    let text = if format == "plain" {
        format!("{}\n\n{}", note.title, note.content)
    } else {
        markdown_document(note)
    };
    app.clipboard().write_text(text).map_err(|e| anyhow!(e.to_string()))?;
    Ok(ExportResult { ok: true, location: None, message: "Copied to clipboard".into() })
}

/// Split a string into lowercase alphanumeric word tokens (len > 2).
fn tokenize(s: &str) -> std::collections::HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect()
}

/// Collect (id, title) of every database in a Notion `/v1/search` response.
fn parse_databases(json: &Value) -> Vec<(String, String)> {
    json["results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|r| r["object"] == "database")
                .filter_map(|r| {
                    let id = r["id"].as_str()?;
                    let title = r["title"]
                        .as_array()
                        .and_then(|t| t.first())
                        .and_then(|t| t["plain_text"].as_str())
                        .unwrap_or("");
                    Some((id.to_string(), title.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Pick the database whose title best overlaps the note title; the first
/// database is the fallback when nothing overlaps.
fn pick_database(dbs: &[(String, String)], note_title: &str) -> Option<String> {
    let want = tokenize(note_title);
    let mut best: Option<(usize, &str)> = None;
    for (id, title) in dbs {
        let score = tokenize(title).iter().filter(|t| want.contains(*t)).count();
        if best.map(|(s, _)| score > s).unwrap_or(true) {
            best = Some((score, id));
        }
    }
    best.map(|(_, id)| id.to_string())
}

async fn notion_search_database(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    version: &str,
    query: &str,
) -> Result<Option<String>> {
    let resp = client
        .post(format!("{base_url}/v1/search"))
        .bearer_auth(token)
        .header("Notion-Version", version)
        .json(&json!({
            "query": query,
            "filter": { "property": "object", "value": "database" },
            "page_size": 20
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Notion search returned {}", resp.status()));
    }
    let body: Value = resp.json().await?;
    Ok(pick_database(&parse_databases(&body), query))
}

async fn export_notion(settings: &Settings, note: &Note) -> Result<ExportResult> {
    let cfg = integration(settings, "notion").ok_or_else(|| anyhow!("Notion not configured"))?;
    let token = cfg.options.get("token").filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow!("Add your Notion token in Settings"))?;
    // Base URL is overridable so tests can target a mock server.
    let base_url = cfg.options.get("base").map(|s| s.as_str()).unwrap_or("https://api.notion.com");

    let client = reqwest::Client::new();
    let version = "2022-06-28";

    // Smart routing: when route == "auto", search the workspace and file the
    // note in the database that best matches its title, falling back to any
    // explicitly configured database.
    let configured = cfg.options.get("database").filter(|d| !d.is_empty()).cloned();
    let route_auto = cfg.options.get("route").map(|r| r == "auto").unwrap_or(false);
    let database_id = if route_auto {
        match notion_search_database(&client, base_url, token, version, &note.title).await? {
            Some(id) => id,
            None => configured.ok_or_else(|| anyhow!("No matching Notion database found"))?,
        }
    } else {
        configured.ok_or_else(|| anyhow!("Add a Notion database ID in Settings"))?
    };

    let db: Value = client
        .get(format!("{base_url}/v1/databases/{database_id}"))
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
        .post(format!("{base_url}/v1/pages"))
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

pub async fn export<R: Runtime>(
    app: &AppHandle<R>,
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
            research: Vec::new(),
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

    fn set_opt(settings: &mut Settings, id: &str, key: &str, val: &str) {
        let cfg = settings.integrations.iter_mut().find(|c| c.id == id).unwrap();
        cfg.enabled = true;
        cfg.options.insert(key.into(), val.into());
    }

    // ---- Slack ----

    #[tokio::test]
    async fn slack_posts_and_reports_errors() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/ok")).respond_with(ResponseTemplate::new(200)).mount(&server).await;
        Mock::given(method("POST")).and(path("/bad")).respond_with(ResponseTemplate::new(500)).mount(&server).await;

        let mut ok = Settings::default();
        set_opt(&mut ok, "slack", "webhook", &format!("{}/ok", server.uri()));
        assert!(export_slack(&ok, &note()).await.unwrap().ok);

        let mut bad = Settings::default();
        set_opt(&mut bad, "slack", "webhook", &format!("{}/bad", server.uri()));
        assert!(export_slack(&bad, &note()).await.is_err());
    }

    #[tokio::test]
    async fn slack_requires_config() {
        assert!(export_slack(&Settings::default(), &note()).await.is_err()); // no webhook
        let mut s = Settings::default();
        s.integrations.clear();
        assert!(export_slack(&s, &note()).await.is_err()); // not configured
    }

    // ---- Webhook ----

    #[tokio::test]
    async fn webhook_posts_and_reports_errors() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/hook")).respond_with(ResponseTemplate::new(200)).mount(&server).await;
        Mock::given(method("POST")).and(path("/err")).respond_with(ResponseTemplate::new(422)).mount(&server).await;

        let mut ok = Settings::default();
        set_opt(&mut ok, "webhook", "url", &format!("{}/hook", server.uri()));
        assert!(export_webhook(&ok, &note()).await.unwrap().ok);

        let mut bad = Settings::default();
        set_opt(&mut bad, "webhook", "url", &format!("{}/err", server.uri()));
        assert!(export_webhook(&bad, &note()).await.is_err());
    }

    #[tokio::test]
    async fn webhook_requires_config() {
        assert!(export_webhook(&Settings::default(), &note()).await.is_err());
        let mut s = Settings::default();
        s.integrations.clear();
        assert!(export_webhook(&s, &note()).await.is_err());
    }

    // ---- Notion ----

    fn notion_settings(base: &str) -> Settings {
        let mut s = Settings::default();
        set_opt(&mut s, "notion", "token", "secret_t");
        set_opt(&mut s, "notion", "database", "db1");
        set_opt(&mut s, "notion", "base", base);
        s
    }

    #[tokio::test]
    async fn notion_creates_page_with_detected_title_prop() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/databases/db1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "properties": { "Title": { "type": "title" } } })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "url": "https://notion.so/p" })))
            .mount(&server)
            .await;

        let res = export_notion(&notion_settings(&server.uri()), &note()).await.unwrap();
        assert!(res.ok);
        assert_eq!(res.location.as_deref(), Some("https://notion.so/p"));
    }

    #[tokio::test]
    async fn notion_defaults_title_prop_and_reports_api_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/databases/db1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "properties": {} })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;

        assert!(export_notion(&notion_settings(&server.uri()), &note()).await.is_err());
    }

    #[tokio::test]
    async fn notion_requires_config() {
        let mut none = Settings::default();
        none.integrations.clear();
        assert!(export_notion(&none, &note()).await.is_err()); // not configured
        assert!(export_notion(&Settings::default(), &note()).await.is_err()); // no token

        let mut no_db = Settings::default();
        set_opt(&mut no_db, "notion", "token", "t");
        assert!(export_notion(&no_db, &note()).await.is_err()); // no database
    }

    #[test]
    fn tokenize_and_pick_database() {
        let dbs = vec![
            ("db-lectures".to_string(), "Biology Lectures".to_string()),
            ("db-personal".to_string(), "Personal Journal".to_string()),
        ];
        // Overlap on "biology" -> lectures database.
        assert_eq!(pick_database(&dbs, "Biology 101 — Cells").as_deref(), Some("db-lectures"));
        // No overlap -> first database as fallback.
        assert_eq!(pick_database(&dbs, "Grocery run").as_deref(), Some("db-lectures"));
        // No databases -> None.
        assert_eq!(pick_database(&[], "anything"), None);
    }

    #[test]
    fn parse_databases_filters_non_databases() {
        let body = json!({
            "results": [
                { "object": "database", "id": "d1", "title": [{ "plain_text": "Notes" }] },
                { "object": "page", "id": "p1" },
                { "object": "database", "id": "d2" }
            ]
        });
        let dbs = parse_databases(&body);
        assert_eq!(dbs, vec![("d1".into(), "Notes".into()), ("d2".into(), "".into())]);
        assert!(parse_databases(&json!({})).is_empty());
    }

    #[tokio::test]
    async fn notion_auto_route_files_into_best_database() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{ "object": "database", "id": "meeting-db", "title": [{ "plain_text": "My Meeting Notes" }] }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/databases/meeting-db"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "properties": { "Name": { "type": "title" } } })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "url": "https://notion.so/routed" })))
            .mount(&server)
            .await;

        let mut s = Settings::default();
        set_opt(&mut s, "notion", "token", "secret_t");
        set_opt(&mut s, "notion", "route", "auto");
        set_opt(&mut s, "notion", "base", &server.uri());
        let res = export_notion(&s, &note()).await.unwrap();
        assert_eq!(res.location.as_deref(), Some("https://notion.so/routed"));
    }

    #[tokio::test]
    async fn notion_auto_route_falls_back_and_errors_without_target() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "results": [] })))
            .mount(&server)
            .await;

        // No search hit and no configured database -> error.
        let mut none = Settings::default();
        set_opt(&mut none, "notion", "token", "t");
        set_opt(&mut none, "notion", "route", "auto");
        set_opt(&mut none, "notion", "base", &server.uri());
        assert!(export_notion(&none, &note()).await.is_err());
    }

    #[tokio::test]
    async fn notion_auto_route_reports_search_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/search"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let mut s = Settings::default();
        set_opt(&mut s, "notion", "token", "t");
        set_opt(&mut s, "notion", "route", "auto");
        set_opt(&mut s, "notion", "base", &server.uri());
        assert!(export_notion(&s, &note()).await.is_err());
    }

    // ---- Dispatcher ----

    #[tokio::test]
    async fn dispatcher_routes_every_target() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).mount(&server).await;

        let dir = tempfile::tempdir().unwrap();
        let app = tauri::test::mock_app();
        let base = dir.path();

        let mut settings = Settings::default();
        set_folder(&mut settings, "markdown", dir.path().to_str().unwrap());
        set_folder(&mut settings, "obsidian", dir.path().to_str().unwrap());
        set_opt(&mut settings, "slack", "webhook", &format!("{}/s", server.uri()));
        set_opt(&mut settings, "webhook", "url", &format!("{}/w", server.uri()));

        for target in [ExportTarget::Markdown, ExportTarget::Obsidian, ExportTarget::Slack, ExportTarget::Webhook] {
            let r = export(app.handle(), base, &settings, &note(), target).await;
            assert!(r.ok, "expected ok for target");
        }

        // Error path: Notion with no config yields ok:false rather than panicking.
        let r = export(app.handle(), base, &settings, &note(), ExportTarget::Notion).await;
        assert!(!r.ok);
    }

    #[test]
    fn markdown_document_defaults_untitled() {
        let mut n = note();
        n.title = String::new();
        assert!(markdown_document(&n).contains("# Untitled"));
    }

    #[tokio::test]
    async fn slack_handles_untitled_note() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/u")).respond_with(ResponseTemplate::new(200)).mount(&server).await;
        let mut s = Settings::default();
        set_opt(&mut s, "slack", "webhook", &format!("{}/u", server.uri()));
        let mut n = note();
        n.title = String::new();
        assert!(export_slack(&s, &n).await.unwrap().ok);
    }

    #[tokio::test]
    async fn dispatcher_handles_clipboard() {
        let dir = tempfile::tempdir().unwrap();
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_clipboard_manager::init())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        // Exercises the clipboard match arm; the write itself may fail headless.
        let _ = export(
            app.handle(),
            dir.path(),
            &Settings::default(),
            &note(),
            ExportTarget::Clipboard { format: "markdown".into() },
        )
        .await;
    }
}
