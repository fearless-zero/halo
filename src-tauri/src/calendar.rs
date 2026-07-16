use crate::types::{CalendarEvent, Settings};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// Which configured integrations are calendar feeds, and their display labels.
fn providers() -> [(&'static str, &'static str); 3] {
    [
        ("google-calendar", "Google"),
        ("apple-calendar", "Apple"),
        ("microsoft-calendar", "Microsoft"),
    ]
}

/// Unfold RFC 5545 folded lines (continuation lines begin with a space or tab).
fn unfold(ics: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in ics.replace('\r', "").lines() {
        if (raw.starts_with(' ') || raw.starts_with('\t')) && !out.is_empty() {
            let last = out.last_mut().unwrap();
            last.push_str(&raw[1..]);
        } else {
            out.push(raw.to_string());
        }
    }
    out
}

fn unescape(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

/// Parse an ICS date-time value (the part after the property's colon).
/// Handles UTC (`...Z`), floating/local, and date-only forms.
fn parse_dt(value: &str) -> Option<DateTime<Utc>> {
    let v = value.trim();
    if v.ends_with('Z') {
        let nd = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%SZ").ok()?;
        Some(Utc.from_utc_datetime(&nd))
    } else if v.len() == 8 {
        let d = NaiveDate::parse_from_str(v, "%Y%m%d").ok()?;
        Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?))
    } else {
        let nd = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
        Local.from_local_datetime(&nd).single().map(|dt| dt.with_timezone(&Utc))
    }
}

pub fn parse_ics(ics: &str, provider: &str) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let mut in_event = false;
    let mut title = String::new();
    let mut start: Option<DateTime<Utc>> = None;
    let mut end: Option<DateTime<Utc>> = None;

    for line in unfold(ics) {
        match line.as_str() {
            "BEGIN:VEVENT" => {
                in_event = true;
                title.clear();
                start = None;
                end = None;
            }
            "END:VEVENT" => {
                if let Some(s) = start {
                    let e = end.unwrap_or(s);
                    events.push(CalendarEvent {
                        title: if title.is_empty() { "Untitled event".to_string() } else { title.clone() },
                        start: s.to_rfc3339(),
                        end: e.to_rfc3339(),
                        provider: provider.to_string(),
                    });
                }
                in_event = false;
            }
            _ if in_event => {
                let Some((name_params, value)) = line.split_once(':') else { continue };
                let name = name_params.split(';').next().unwrap_or("");
                match name {
                    "SUMMARY" => title = unescape(value),
                    "DTSTART" => start = parse_dt(value),
                    "DTEND" => end = parse_dt(value),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    events
}

fn to_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("webcal://") {
        format!("https://{rest}")
    } else {
        url.to_string()
    }
}

/// Fetch and parse every configured calendar feed. Per-feed failures are logged
/// and skipped so one bad URL never breaks the rest.
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn list_events(settings: &Settings) -> Vec<CalendarEvent> {
    let client = reqwest::Client::new();
    let mut all = Vec::new();

    for (id, label) in providers() {
        let Some(cfg) = settings.integrations.iter().find(|c| c.id == id) else { continue };
        if !cfg.enabled {
            continue;
        }
        let Some(url) = cfg.options.get("ics").filter(|u| !u.trim().is_empty()) else { continue };
        match client.get(to_https(url)).send().await {
            Ok(resp) => match resp.text().await {
                Ok(body) => all.extend(parse_ics(&body, label)),
                Err(e) => eprintln!("calendar {label}: read failed: {e}"),
            },
            Err(e) => eprintln!("calendar {label}: fetch failed: {e}"),
        }
    }

    all.sort_by(|a, b| a.start.cmp(&b.start));
    all
}

/// The event overlapping `now`, or the nearest one starting within 15 minutes.
pub fn current_or_next(events: &[CalendarEvent], now: DateTime<Utc>) -> Option<&CalendarEvent> {
    let parse = |s: &str| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc));
    if let Some(ev) = events.iter().find(|e| {
        match (parse(&e.start), parse(&e.end)) {
            (Some(s), Some(en)) => s <= now && now < en,
            _ => false,
        }
    }) {
        return Some(ev);
    }
    events
        .iter()
        .filter_map(|e| parse(&e.start).map(|s| (e, s)))
        .filter(|(_, s)| *s >= now && (*s - now).num_minutes() <= 15)
        .min_by_key(|(_, s)| *s)
        .map(|(e, _)| e)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Weekly sync\r\nDTSTART:20260716T140000Z\r\nDTEND:20260716T143000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:Design\\, review\r\nDTSTART:20260716T150000Z\r\nDTEND:20260716T160000Z\r\nEND:VEVENT\r\nEND:VCALENDAR";

    #[test]
    fn parses_events_and_unescapes() {
        let events = parse_ics(SAMPLE, "Google");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Weekly sync");
        assert_eq!(events[1].title, "Design, review");
        assert_eq!(events[0].provider, "Google");
    }

    #[test]
    fn finds_current_event() {
        let events = parse_ics(SAMPLE, "Google");
        let now = Utc.with_ymd_and_hms(2026, 7, 16, 14, 15, 0).unwrap();
        assert_eq!(current_or_next(&events, now).unwrap().title, "Weekly sync");
    }

    #[test]
    fn finds_next_event_within_window() {
        let events = parse_ics(SAMPLE, "Google");
        let now = Utc.with_ymd_and_hms(2026, 7, 16, 14, 50, 0).unwrap();
        assert_eq!(current_or_next(&events, now).unwrap().title, "Design, review");
    }

    #[test]
    fn no_event_when_nothing_matches() {
        let events = parse_ics(SAMPLE, "Google");
        let now = Utc.with_ymd_and_hms(2026, 7, 16, 20, 0, 0).unwrap();
        assert!(current_or_next(&events, now).is_none());
        assert!(current_or_next(&[], now).is_none());
    }

    #[test]
    fn unfolds_continuation_lines() {
        let folded = "SUMMARY:Long\r\n  title\r\nDTSTART:x";
        let lines = unfold(folded);
        assert_eq!(lines[0], "SUMMARY:Long title");
        assert_eq!(lines[1], "DTSTART:x");
    }

    #[test]
    fn unescapes_ics_text() {
        assert_eq!(unescape("a\\,b\\;c\\nd\\\\e"), "a,b;c\nd\\e");
    }

    #[test]
    fn parses_datetime_forms() {
        assert!(parse_dt("20260716T140000Z").is_some());
        assert!(parse_dt("20260716").is_some()); // date-only
        assert!(parse_dt("20260716T140000").is_some()); // floating/local
        assert!(parse_dt("garbage").is_none());
    }

    #[test]
    fn missing_dtend_defaults_to_start() {
        let ics = "BEGIN:VEVENT\r\nSUMMARY:One\r\nDTSTART:20260716T140000Z\r\nEND:VEVENT";
        let events = parse_ics(ics, "Apple");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].start, events[0].end);
    }

    #[test]
    fn event_without_start_is_skipped() {
        let ics = "BEGIN:VEVENT\r\nSUMMARY:No start\r\nEND:VEVENT";
        assert!(parse_ics(ics, "Apple").is_empty());
    }

    #[test]
    fn defaults_untitled_summary() {
        let ics = "BEGIN:VEVENT\r\nDTSTART:20260716T140000Z\r\nEND:VEVENT";
        assert_eq!(parse_ics(ics, "Apple")[0].title, "Untitled event");
    }

    #[test]
    fn webcal_becomes_https() {
        assert_eq!(to_https("webcal://host/cal.ics"), "https://host/cal.ics");
        assert_eq!(to_https("https://host/cal.ics"), "https://host/cal.ics");
    }

    #[tokio::test]
    async fn list_events_fetches_enabled_feeds_only() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cal.ics"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SAMPLE))
            .mount(&server)
            .await;

        let mut settings = Settings::default();
        // google-calendar enabled with a real (mock) feed; the others stay off.
        let cfg = settings.integrations.iter_mut().find(|c| c.id == "google-calendar").unwrap();
        cfg.enabled = true;
        cfg.options.insert("ics".into(), format!("{}/cal.ics", server.uri()));

        let events = list_events(&settings).await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].provider, "Google");
    }

    #[test]
    fn current_or_next_ignores_unparseable_dates() {
        let bad = vec![CalendarEvent {
            title: "x".into(),
            start: "not-a-date".into(),
            end: "nope".into(),
            provider: "P".into(),
        }];
        let now = Utc.with_ymd_and_hms(2026, 7, 16, 14, 0, 0).unwrap();
        assert!(current_or_next(&bad, now).is_none());
    }

    #[test]
    fn parse_ics_skips_unknown_props_and_colonless_lines() {
        let ics = "BEGIN:VEVENT\r\nSUMMARY:Meeting\r\nLOCATION:HQ\r\nNOCOLONLINE\r\nDTSTART:20260716T140000Z\r\nEND:VEVENT";
        let events = parse_ics(ics, "Apple");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Meeting");
    }
}
