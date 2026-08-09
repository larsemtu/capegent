//! Events fra Quicket. Sidene embedder schema.org JSON-LD med Event-objekter
//! (verifisert 2026-08) — deterministisk parsing uten headless browser.
//! URL-lokasjonsfilteret deres virker ikke, så vi filtrerer på stedsnavn selv.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use schema::EventItem;
use serde_json::Value;
use tracing::warn;

const PAGES: &[&str] = &[
    "https://www.quicket.co.za/events/",
    "https://www.quicket.co.za/events/?page=2",
    "https://www.quicket.co.za/events/?page=3",
];

/// Stor-Cape Town pluss vinlandet — dashbordet er for inspirasjon, så
/// Stellenbosch/Franschhoek er innenfor rekkevidde.
const AREA_TERMS: &[&str] = &[
    "cape town", "kaapstad", "sea point", "green point", "camps bay", "clifton",
    "century city", "woodstock", "observatory", "salt river", "gardens", "bo-kaap",
    "waterfront", "v&a", "claremont", "newlands", "kirstenbosch", "rondebosch",
    "muizenberg", "kalk bay", "fish hoek", "hout bay", "noordhoek", "kommetjie",
    "bellville", "durbanville", "milnerton", "blouberg", "big bay", "table view",
    "stellenbosch", "franschhoek", "paarl", "somerset west", "strand", "khayelitsha",
    "athlone", "goodwood", "parow", "constantia", "tokai", "wynberg",
];

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<EventItem>> {
    let mut events = vec![];
    for url in PAGES {
        match fetch_page(client, url).await {
            Ok(mut page_events) => events.append(&mut page_events),
            Err(e) => warn!("quicket-side feilet ({url}): {e:#}"),
        }
    }

    let now = Utc::now();
    let horizon = now + Duration::days(60);
    events.retain(|e: &EventItem| {
        DateTime::parse_from_rfc3339(&e.start)
            .map(|dt| {
                let dt = dt.with_timezone(&Utc);
                dt >= now - Duration::hours(12) && dt <= horizon
            })
            .unwrap_or(false)
    });
    events.sort_by(|a, b| a.start.cmp(&b.start));
    events.dedup_by(|a, b| a.url == b.url);
    events.truncate(15);
    Ok(events)
}

async fn fetch_page(client: &reqwest::Client, url: &str) -> Result<Vec<EventItem>> {
    let html = client
        .get(url)
        .send()
        .await
        .context("GET quicket")?
        .error_for_status()?
        .text()
        .await?;

    let mut events = vec![];
    for block in extract_ld_json(&html) {
        let Ok(value) = serde_json::from_str::<Value>(&block) else {
            continue;
        };
        let items: Vec<&Value> = match &value {
            Value::Array(a) => a.iter().collect(),
            v => vec![v],
        };
        for item in items {
            if item["@type"] != "Event" {
                continue;
            }
            let title = item["name"].as_str().unwrap_or_default().trim().to_string();
            let start = item["startDate"].as_str().unwrap_or_default().to_string();
            let url = item["url"].as_str().unwrap_or_default().to_string();
            let venue = item["location"]["name"].as_str().unwrap_or_default();
            let street = item["location"]["address"]["streetAddress"]
                .as_str()
                .unwrap_or_default();
            let locality = item["location"]["address"]["addressLocality"]
                .as_str()
                .unwrap_or_default();

            if title.is_empty() || start.is_empty() || url.is_empty() {
                continue;
            }
            let haystack = format!("{venue} {street} {locality}").to_lowercase();
            if !AREA_TERMS.iter().any(|t| haystack.contains(t)) {
                continue;
            }
            events.push(EventItem {
                title,
                start,
                venue: if venue.is_empty() { locality.to_string() } else { venue.to_string() },
                url,
            });
        }
    }
    Ok(events)
}

/// Plukk ut <script type="application/ld+json">-innhold uten full HTML-parser.
fn extract_ld_json(html: &str) -> Vec<String> {
    let mut blocks = vec![];
    let mut rest = html;
    while let Some(start) = rest.find("application/ld+json") {
        let after = &rest[start..];
        let Some(open) = after.find('>') else { break };
        let body = &after[open + 1..];
        let Some(close) = body.find("</script>") else { break };
        blocks.push(body[..close].trim().to_string());
        rest = &body[close..];
    }
    blocks
}
