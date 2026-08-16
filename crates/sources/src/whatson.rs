//! whatsonincapetown.com sin redaksjonelle konsertoversikt — oppdateres
//! ukentlig og er fellesnevneren for de store konsertannonseringene
//! (verifisert mot GPT-kuratert liste aug 2026). Redaksjonell HTML uten
//! strukturert data, så teksten trekkes ut her og struktureres av Claude
//! i collectoren (hash-cachet på innholdet).

use anyhow::{Context, Result};
use crate::truncate_chars;

const URL: &str = "https://whatsonincapetown.com/biggest-upcoming-shows-concerts-in-cape-town/";

/// Henter siden og returnerer lesbar tekst med overskrifter og
/// billettlenker bevart, klar for LLM-ekstraksjon.
pub async fn fetch_text(client: &reqwest::Client) -> Result<String> {
    let html = client
        .get(URL)
        .send()
        .await
        .context("GET whatsonincapetown")?
        .error_for_status()?
        .text()
        .await?;

    // Kutt til artikkelkroppen: fra første h2 til "If you liked this article"
    let start = html.find("<h2").unwrap_or(0);
    let end = html.find("If you liked this article").unwrap_or(html.len());
    let body = &html[start..end];

    let mut text = String::with_capacity(body.len() / 4);
    let mut rest = body;
    // Marker overskrifter og behold billettlenker som (url)
    while !rest.is_empty() {
        if let Some(pos) = rest.find('<') {
            text.push_str(&rest[..pos]);
            let tag_end = match rest[pos..].find('>') {
                Some(e) => pos + e + 1,
                None => break,
            };
            let tag = &rest[pos..tag_end];
            let tag_lower = tag.to_lowercase();
            if tag_lower.starts_with("<h2") || tag_lower.starts_with("<h3") {
                text.push_str("\n\n## ");
            } else if tag_lower.starts_with("<p") || tag_lower.starts_with("<br") || tag_lower.starts_with("<li") {
                text.push('\n');
            } else if tag_lower.starts_with("<a ") {
                // Behold billettplattform-lenker
                if let Some(href) = tag.split("href=\"").nth(1).and_then(|h| h.split('"').next()) {
                    let h = href.to_lowercase();
                    if ["webtickets", "quicket", "ticketmaster", "howler", "playy", "bigconcerts", "computicket", "tixsa"]
                        .iter()
                        .any(|d| h.contains(d))
                    {
                        text.push_str(&format!(" (billett: {href}) "));
                    }
                }
            }
            rest = &rest[tag_end..];
        } else {
            text.push_str(rest);
            break;
        }
    }
    let mut text = crate::strip_html(&text.replace("## ", "\n§H§ "))
        .replace("§H§", "\n## ");
    truncate_chars(&mut text, 15000);
    Ok(text)
}
