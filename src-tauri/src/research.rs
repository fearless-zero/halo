use crate::types::ResearchFinding;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashSet;

/// Default source: the English Wikipedia (keyless, fully open content).
pub const WIKI_BASE: &str = "https://en.wikipedia.org";

const RESEARCH_HEADING: &str = "## Background & Research";

/// Structural headings that are not worth researching on their own.
const GENERIC_HEADINGS: [&str; 11] = [
    "summary",
    "key points",
    "decisions",
    "action items",
    "questions to review",
    "overview",
    "important definitions",
    "background & research",
    "further reading",
    "recommendation",
    "notable quotes",
];

/// Derive up to `max` search queries from a note's title and section headings.
pub fn extract_queries(title: &str, content: &str, max: usize) -> Vec<String> {
    let mut queries: Vec<String> = Vec::new();
    let mut add = |candidate: &str| {
        let q = candidate.trim();
        if q.len() >= 3
            && !GENERIC_HEADINGS.contains(&q.to_lowercase().as_str())
            && !queries.iter().any(|e| e.eq_ignore_ascii_case(q))
        {
            queries.push(q.to_string());
        }
    };

    let t = title.trim();
    if !t.is_empty() && !t.eq_ignore_ascii_case("New recording") && !t.eq_ignore_ascii_case("Untitled note") {
        add(t);
    }
    for line in content.lines() {
        let l = line.trim();
        for prefix in ["### ", "## "] {
            if let Some(rest) = l.strip_prefix(prefix) {
                add(rest);
                break;
            }
        }
    }
    queries.truncate(max);
    queries
}

/// Percent-encode a Wikipedia page title for use in a REST path segment.
fn encode_title(title: &str) -> String {
    let mut out = String::new();
    for b in title.replace(' ', "_").bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Extract page titles from a Wikipedia `list=search` response.
fn parse_search_titles(json: &Value) -> Vec<String> {
    json["query"]["search"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|h| h["title"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Turn a REST summary payload into a finding, or `None` if it has no extract.
fn parse_summary(json: &Value) -> Option<ResearchFinding> {
    let title = json["title"].as_str()?;
    let extract = json["extract"].as_str().unwrap_or("").trim();
    if extract.is_empty() {
        return None;
    }
    let url = json["content_urls"]["desktop"]["page"]
        .as_str()
        .or_else(|| json["content_urls"]["mobile"]["page"].as_str())
        .unwrap_or("")
        .to_string();
    Some(ResearchFinding {
        title: title.to_string(),
        summary: extract.chars().take(600).collect(),
        url,
        source: "Wikipedia".to_string(),
    })
}

/// Render findings as a Markdown section appended to a note.
pub fn render_section(findings: &[ResearchFinding]) -> String {
    if findings.is_empty() {
        return String::new();
    }
    let mut out = format!("\n\n{RESEARCH_HEADING}\n\n_Researched automatically from Wikipedia._\n");
    for f in findings {
        out.push_str(&format!("\n### {}\n{}\n", f.title, f.summary));
        if !f.url.is_empty() {
            out.push_str(&format!("\n[Read more →]({})\n", f.url));
        }
    }
    out
}

/// Remove a previously-appended research section so re-running is idempotent.
pub fn strip_section(content: &str) -> String {
    match content.find(RESEARCH_HEADING) {
        Some(idx) => content[..idx].trim_end().to_string(),
        None => content.trim_end().to_string(),
    }
}

async fn find_one(client: &reqwest::Client, base: &str, query: &str) -> Result<Option<ResearchFinding>> {
    let search: Value = {
        let resp = client
            .get(format!("{base}/w/api.php"))
            .query(&[
                ("action", "query"),
                ("list", "search"),
                ("srsearch", query),
                ("srlimit", "1"),
                ("format", "json"),
            ])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Wikipedia search returned {}", resp.status()));
        }
        resp.json().await?
    };

    let Some(first) = parse_search_titles(&search).into_iter().next() else {
        return Ok(None);
    };

    let resp = client
        .get(format!("{base}/api/rest_v1/page/summary/{}", encode_title(&first)))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let summary: Value = resp.json().await?;
    Ok(parse_summary(&summary))
}

/// Research each query against `base`, returning de-duplicated findings. Any
/// network failure for a query is swallowed so offline use degrades gracefully.
pub async fn research(base: &str, queries: &[String]) -> Vec<ResearchFinding> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let mut findings = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for q in queries {
        if let Ok(Some(f)) = find_one(&client, base, q).await {
            if seen.insert(f.title.clone()) {
                findings.push(f);
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn extract_queries_uses_title_and_headings_skipping_generic() {
        let content = "## Summary\ntext\n## Photosynthesis\nmore\n### Calvin Cycle\n## Photosynthesis\n";
        let qs = extract_queries("Biology 101", content, 5);
        assert_eq!(qs, vec!["Biology 101", "Photosynthesis", "Calvin Cycle"]);
    }

    #[test]
    fn extract_queries_ignores_placeholder_titles_and_truncates() {
        let qs = extract_queries("New recording", "## Alpha\n## Beta\n## Gamma", 2);
        assert_eq!(qs, vec!["Alpha", "Beta"]);
        assert!(extract_queries("Untitled note", "", 3).is_empty());
        // Short headings (< 3 chars) are dropped.
        assert!(extract_queries("", "## x", 3).is_empty());
    }

    #[test]
    fn encode_title_escapes_unsafe_chars() {
        assert_eq!(encode_title("Calvin Cycle"), "Calvin_Cycle");
        assert_eq!(encode_title("C/C++"), "C%2FC%2B%2B");
    }

    #[test]
    fn parse_search_titles_reads_array_or_empty() {
        let v = json!({ "query": { "search": [{ "title": "A" }, { "title": "B" }, {}] } });
        assert_eq!(parse_search_titles(&v), vec!["A", "B"]);
        assert!(parse_search_titles(&json!({})).is_empty());
    }

    #[test]
    fn parse_summary_builds_finding_and_skips_empty() {
        let v = json!({
            "title": "Photosynthesis",
            "extract": "  A process used by plants.  ",
            "content_urls": { "desktop": { "page": "https://en.wikipedia.org/wiki/Photosynthesis" } }
        });
        let f = parse_summary(&v).unwrap();
        assert_eq!(f.title, "Photosynthesis");
        assert_eq!(f.summary, "A process used by plants.");
        assert_eq!(f.url, "https://en.wikipedia.org/wiki/Photosynthesis");
        assert_eq!(f.source, "Wikipedia");

        assert!(parse_summary(&json!({ "title": "X", "extract": "" })).is_none());
        assert!(parse_summary(&json!({ "extract": "y" })).is_none());
    }

    #[test]
    fn parse_summary_falls_back_to_mobile_url() {
        let v = json!({
            "title": "T",
            "extract": "e",
            "content_urls": { "mobile": { "page": "https://m/wiki/T" } }
        });
        assert_eq!(parse_summary(&v).unwrap().url, "https://m/wiki/T");
        // No url at all -> empty string.
        let v2 = json!({ "title": "T", "extract": "e" });
        assert_eq!(parse_summary(&v2).unwrap().url, "");
    }

    #[test]
    fn render_and_strip_section_are_inverse() {
        assert!(render_section(&[]).is_empty());
        let findings = vec![
            ResearchFinding { title: "A".into(), summary: "sa".into(), url: "http://a".into(), source: "Wikipedia".into() },
            ResearchFinding { title: "B".into(), summary: "sb".into(), url: String::new(), source: "Wikipedia".into() },
        ];
        let section = render_section(&findings);
        assert!(section.contains("### A"));
        assert!(section.contains("[Read more →](http://a)"));
        assert!(!section.contains("[Read more →]()"));

        let base = "# Notes\n\nbody";
        let combined = format!("{base}{section}");
        assert_eq!(strip_section(&combined), base);
        // No section present -> just trimmed.
        assert_eq!(strip_section("plain\n\n"), "plain");
    }

    #[tokio::test]
    async fn research_collects_and_dedupes_findings() {
        let server = MockServer::start().await;
        // Both queries resolve to the same page "Cells" -> deduped to one.
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .and(query_param("srsearch", "Biology"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "query": { "search": [{ "title": "Cells" }] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .and(query_param("srsearch", "Cell biology"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "query": { "search": [{ "title": "Cells" }] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/rest_v1/page/summary/Cells"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "title": "Cells",
                "extract": "The basic unit of life.",
                "content_urls": { "desktop": { "page": "https://en.wikipedia.org/wiki/Cell" } }
            })))
            .mount(&server)
            .await;

        let findings = research(&server.uri(), &["Biology".into(), "Cell biology".into()]).await;
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "Cells");
    }

    #[tokio::test]
    async fn research_skips_no_results_and_missing_summaries() {
        let server = MockServer::start().await;
        // "empty" -> no search hits.
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .and(query_param("srsearch", "empty"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "query": { "search": [] } })))
            .mount(&server)
            .await;
        // "missing" -> a hit whose summary 404s.
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .and(query_param("srsearch", "missing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "query": { "search": [{ "title": "Gone" }] } })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/rest_v1/page/summary/Gone"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let findings = research(&server.uri(), &["empty".into(), "missing".into()]).await;
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn research_ignores_query_when_search_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        assert!(research(&server.uri(), &["boom".into()]).await.is_empty());
    }
}
