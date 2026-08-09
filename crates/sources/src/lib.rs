//! Kilder. `Source`-traiten dekker ustrukturerte kilder (RSS/HTML) som skal
//! gjennom LLM-steget. Strukturerte API-er (Open-Meteo, EskomSePush, iCal)
//! har egne typede moduler og går aldri innom Claude.

pub mod article;
pub mod calendar;
pub mod eventbrite;
pub mod events;
pub mod linear;
pub mod loadshedding;
pub mod marine;
pub mod rss;
pub mod ticketmaster;
pub mod weather;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct RawItem {
    pub source: String,
    pub title: String,
    pub summary: String,
    /// Full artikkeltekst der feeden leverer den (content:encoded o.l.).
    /// Mangler den, forsøker collectoren å hente artikkelsiden selv.
    pub full_text: Option<String>,
    pub url: String,
    pub published: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> &'static str;
    async fn fetch(&self, client: &reqwest::Client) -> Result<Vec<RawItem>>;
}

/// Grov HTML-vask for RSS-beskrivelser: fjern tags, dekod de vanligste entitetene.
pub fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#8217;", "’")
        .replace("&#8216;", "‘")
        .replace("&#8220;", "“")
        .replace("&#8221;", "”")
        .replace("&nbsp;", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Miljøvariabel som er satt OG ikke-tom. `.env`-maler og Actions-secrets
/// uten verdi gir tom streng, som skal bety «ikke konfigurert».
pub fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Kutt en streng på nærmeste char-grense ≤ max_bytes (String::truncate
/// panikker midt i en multibyte-char).
pub fn truncate_chars(s: &mut String, max_bytes: usize) {
    if s.len() <= max_bytes {
        return;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
}
