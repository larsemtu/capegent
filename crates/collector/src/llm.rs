//! Claude-steget: rå artikler inn, strukturert NewsBatch ut via tvunget
//! tool-kall. Schemaet genereres fra Rust-typene, så svaret kan ikke
//! parse feil. Uten API-nøkkel finnes en fallback som viser originaltekst.

use anyhow::{bail, Context, Result};
use schema::{Category, NewsBatch, NewsEntry, NewsItem};
use serde_json::{json, Value};
use sources::{truncate_chars, RawItem};

const MODEL: &str = "claude-haiku-4-5-20251001";

const SYSTEM: &str = "Du lager nyhetssammendrag for et veggdashbord hos to \
norske MBA-studenter i Green Point, Cape Town. Oversett til naturlig norsk \
(bokmål). Urgency 5 = akutt fare/stor hendelse i Cape Town, 1 = kuriosa. \
Saker om Western Cape og Cape Town er viktigere enn nasjonale saker.";

pub async fn summarize(
    client: &reqwest::Client,
    api_key: &str,
    batch: &[RawItem],
) -> Result<Vec<NewsEntry>> {
    let settings = schemars::gen::SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = true;
    });
    let schema = settings.into_generator().into_root_schema_for::<NewsBatch>();
    let schema_value = serde_json::to_value(schema)?;

    let mut prompt = String::from(
        "Oppsummer og klassifiser hver av disse sakene. Behold source_url \
         nøyaktig som oppgitt. Én output-item per input-sak.\n",
    );
    for (i, item) in batch.iter().enumerate() {
        prompt.push_str(&format!(
            "\n## Sak {} (kilde: {})\nTittel: {}\nIngress: {}\nsource_url: {}\n",
            i + 1,
            item.source,
            item.title,
            item.summary,
            item.url
        ));
    }

    let body = json!({
        "model": MODEL,
        "max_tokens": 8000,
        "system": SYSTEM,
        "messages": [{"role": "user", "content": prompt}],
        "tools": [{
            "name": "emit_news",
            "description": "Lever de ferdige nyhetssammendragene",
            "input_schema": schema_value,
        }],
        "tool_choice": {"type": "tool", "name": "emit_news"},
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .context("POST til Claude API")?;

    let status = resp.status();
    let value: Value = resp.json().await.context("les Claude-svar")?;
    if !status.is_success() {
        bail!("Claude API ga {status}: {value}");
    }

    let input = value["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"] == "tool_use"))
        .map(|b| b["input"].clone())
        .context("ingen tool_use-blokk i Claude-svaret")?;
    let parsed: NewsBatch = serde_json::from_value(input).context("parse NewsBatch")?;

    // Koble source/published tilbake fra råvarene via URL
    Ok(parsed
        .items
        .into_iter()
        .filter_map(|item| {
            let raw = batch.iter().find(|r| r.url == item.source_url)?;
            Some(NewsEntry {
                item,
                source: raw.source.clone(),
                published_at: raw.published,
            })
        })
        .collect())
}

/// Uten API-nøkkel (eller ved feilet kall): vis originaltekst uoversatt.
/// Disse skal IKKE inn i cachen — de skal oversettes neste gang nøkkelen finnes.
pub fn fallback(raw: &RawItem) -> NewsEntry {
    let mut summary = raw.summary.clone();
    truncate_chars(&mut summary, 200);
    NewsEntry {
        item: NewsItem {
            headline_no: raw.title.clone(),
            summary_no: summary,
            category: Category::Other,
            urgency: 2,
            source_url: raw.url.clone(),
        },
        source: raw.source.clone(),
        published_at: raw.published,
    }
}
