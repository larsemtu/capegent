//! Best-effort-henting av artikkeltekst fra en nyhets-URL, for lange
//! sammendrag. Feiler mykt — RSS-ingressen er alltid fallback.

use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::truncate_chars;

const MAX_BYTES: usize = 6000;

pub async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let html = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET artikkel {url}"))?
        .error_for_status()?
        .text()
        .await?;

    let doc = Html::parse_document(&html);
    // <article> først (mest presist), ellers alle avsnitt
    let mut text = collect_paragraphs(&doc, "article p");
    if text.len() < 300 {
        text = collect_paragraphs(&doc, "p");
    }
    truncate_chars(&mut text, MAX_BYTES);
    Ok(text)
}

fn collect_paragraphs(doc: &Html, selector: &str) -> String {
    let Ok(sel) = Selector::parse(selector) else {
        return String::new();
    };
    let mut out = String::new();
    for p in doc.select(&sel) {
        let t: String = p.text().collect::<Vec<_>>().join(" ");
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        // Hopp over navigasjon/byline-smuler
        if t.len() < 60 {
            continue;
        }
        out.push_str(&t);
        out.push('\n');
        if out.len() > MAX_BYTES {
            break;
        }
    }
    out
}
