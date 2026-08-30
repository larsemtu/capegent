//! Events fra Eventbrite — by-filtrerte sider med schema.org JSON-LD
//! ItemList (verifisert 2026-08). Dekker tech-meetups, business og
//! entreprenørscenen som Quicket ikke har.

use crate::events::{extract_ld_json, AREA_TERMS};
use anyhow::{Context, Result};
use schema::EventItem;
use serde_json::Value;
use tracing::warn;

const PAGES: &[&str] = &[
    "https://www.eventbrite.com/d/south-africa--cape-town/events/",
    "https://www.eventbrite.com/d/south-africa--cape-town/science-and-tech--events/",
    "https://www.eventbrite.com/d/south-africa--cape-town/business--events/",
    "https://www.eventbrite.com/d/south-africa--cape-town/music--events/",
];

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<EventItem>> {
    let mut events = vec![];
    for url in PAGES {
        match fetch_page(client, url).await {
            Ok(mut page) => events.append(&mut page),
            Err(e) => warn!("eventbrite-side feilet ({url}): {e:#}"),
        }
    }
    events.sort_by(|a, b| a.url.cmp(&b.url));
    events.dedup_by(|a, b| a.url == b.url);
    Ok(events)
}

async fn fetch_page(client: &reqwest::Client, url: &str) -> Result<Vec<EventItem>> {
    let html = client
        .get(url)
        .send()
        .await
        .context("GET eventbrite")?
        .error_for_status()?
        .text()
        .await?;

    let mut events = vec![];
    for block in extract_ld_json(&html) {
        let Ok(value) = serde_json::from_str::<Value>(&block) else {
            continue;
        };
        let empty = vec![];
        let list = value["itemListElement"].as_array().unwrap_or(&empty);
        for li in list {
            let item = &li["item"];
            let title = item["name"]
                .as_str()
                .or_else(|| li["name"].as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            let start = item["startDate"].as_str().unwrap_or_default().to_string();
            let url = item["url"].as_str().unwrap_or_default().to_string();
            let locality = item["location"]["address"]["addressLocality"]
                .as_str()
                .unwrap_or_default();
            let venue = item["location"]["name"].as_str().unwrap_or(locality);
            if title.is_empty() || start.is_empty() || url.is_empty() {
                continue;
            }
            let haystack = format!("{venue} {locality}").to_lowercase();
            if !AREA_TERMS.iter().any(|t| haystack.contains(t)) {
                continue;
            }
            // Dato uten tid -> midt på dagen så tidsfiltre ikke vraker den
            let start = if start.contains('T') { start } else { format!("{start}T12:00:00Z") };
            events.push(EventItem {
                title,
                start,
                venue: venue.to_string(),
                url,
                relevance: None,
                why: None,
            });
        }
    }
    Ok(events)
}
