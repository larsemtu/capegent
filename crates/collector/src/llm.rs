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
         nøyaktig som oppgitt. Én output-item per input-sak. summary_no er \
         maks 2 setninger for oversikten. detail_no er et fyldig sammendrag \
         på 4-8 setninger basert på hele artikkelteksten der den finnes — \
         få med konkrete fakta: hvem, hva, hvor, tall, sitater.\n",
    );
    for (i, item) in batch.iter().enumerate() {
        prompt.push_str(&format!(
            "\n## Sak {} (kilde: {})\nTittel: {}\nIngress: {}\n",
            i + 1,
            item.source,
            item.title,
            item.summary,
        ));
        if let Some(text) = &item.full_text {
            prompt.push_str(&format!("Artikkeltekst: {text}\n"));
        }
        prompt.push_str(&format!("source_url: {}\n", item.url));
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
        // Batcher med full artikkeltekst tar godt over den globale
        // 30s-timeouten som er ment for feed-/API-henting
        .timeout(std::time::Duration::from_secs(180))
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

const SURF_SYSTEM: &str = "Du er surfguide for to nybegynnere/lett øvede i \
Cape Town. Vurder forholdene per spot for de neste 48 timene. Viktigst: \
vindretning mot spotens offshore-retning — pålandsvind og strøm som presser \
mot land gjør det nesten umulig å padle ut, spesielt på Muizenberg \
(nybegynnerspot, offshore = NV). Big Bay: offshore ≈ ØSØ. Llandudno: \
offshore ≈ Ø, krevende spot for øvede. Cape Doctor (frisk SØ) ødelegger \
Muizenberg men gir offshore på Big Bay. Pek på konkrete tidsvinduer \
(dag + klokkeslett), svellstørrelse/periode og padleforhold. Vanntemp \
betyr våtdrakttykkelse. Vær konkret og ærlig — si «dropp det» når det er dårlig. Ren tekst uten markdown-formatering.";

/// Claude-tolkning av surfforholdene. Kalles kun når varselet har endret
/// seg (hash-cache i redb) — open-meteo oppdaterer modellen ca. hver 6. time.
pub async fn analyze_surf(
    client: &reqwest::Client,
    api_key: &str,
    spots: &[schema::SurfSpot],
) -> Result<schema::SurfAnalysisBatch> {
    let settings = schemars::gen::SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = true;
    });
    let schema_value = serde_json::to_value(
        settings
            .into_generator()
            .into_root_schema_for::<schema::SurfAnalysisBatch>(),
    )?;

    let mut prompt = String::from(
        "Varsel per spot (hver 3. time). Format: tid | svell m @ s fra retning | \
         vind m/s fra retning | tidevann m | strøm km/t fra retning\n",
    );
    for spot in spots {
        prompt.push_str(&format!(
            "\n## {} (vanntemp {})\n",
            spot.name,
            spot.water_temp_c
                .map(|t| format!("{t:.0}°C"))
                .unwrap_or_else(|| "ukjent".into())
        ));
        for h in spot.hourly.iter().step_by(3) {
            prompt.push_str(&format!(
                "{} | {:.1}m @ {:.0}s fra {:.0}° | {:.0} m/s fra {:.0}° | {} | {}\n",
                h.time,
                h.swell_height_m,
                h.swell_period_s,
                h.swell_direction_deg,
                h.wind_ms,
                h.wind_direction_deg,
                h.tide_m.map(|t| format!("{t:+.1}m")).unwrap_or_else(|| "-".into()),
                match (h.current_kmh, h.current_direction_deg) {
                    (Some(v), Some(d)) => format!("{v:.1} fra {d:.0}°"),
                    _ => "-".into(),
                },
            ));
        }
    }
    prompt.push_str("\nGi summary_no (2-3 setninger: beste spot/vindu fremover) og analysis_no per spot.");

    let body = json!({
        "model": MODEL,
        "max_tokens": 2000,
        "system": SURF_SYSTEM,
        "messages": [{"role": "user", "content": prompt}],
        "tools": [{
            "name": "emit_surf_analysis",
            "description": "Lever surfvurderingen",
            "input_schema": schema_value,
        }],
        "tool_choice": {"type": "tool", "name": "emit_surf_analysis"},
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .timeout(std::time::Duration::from_secs(120))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .context("POST surf-analyse til Claude API")?;

    let status = resp.status();
    let value: Value = resp.json().await.context("les Claude-svar")?;
    if !status.is_success() {
        bail!("Claude API ga {status}: {value}");
    }
    let input = value["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"] == "tool_use"))
        .map(|b| b["input"].clone())
        .context("ingen tool_use-blokk i surf-analysen")?;
    Ok(serde_json::from_value(input).context("parse SurfAnalysisBatch")?)
}

/// Interesseprofilen kurateringen vurderer mot. Rediger fritt.
const EVENT_PROFILE: &str = "To norske menn, 29 år, MBA-studenter i Green Point, \
Cape Town, med bredt spekter: uteliv og fester (First Thursdays var et høydepunkt), \
DJ-opplevelser og klubbkvelder — spesielt amapiano, afrobeats og afro-house \
(Black Coffee, Burna Boy, Fireboy DML, Tyla og lignende er ekstremt aktuelt). \
Konserter og musikkfestivaler generelt. Sport: rugby er kritisk (Springboks/All \
Blacks = relevans 5), løping/run clubs, surfing. Mat- og nattmarkeder, \
matopplevelser. Kunstutstillinger og kultur. Både små lettbeinte ting og store \
hovedeventer teller — det viktige er at det gir liv og røre eller en ekte \
opplevelse. EKSTRA VIKTIG: events der man møter og blir kjent med unge folk \
(20-30) — lokale, expats og tilreisende. De vil sosialisere seg og bygge \
nettverk i byen, så sosiale settinger med mingling (uteliv, fester, run clubs, \
sosiale markeder, quiz/spillkvelder med ung profil) skal vektes opp. IKKE aktuelt: barnearrangementer, religiøse møter, bedriftsseminarer, \
nettverkstreff for bransjer vi ikke er i, pensjonistarrangementer.";

/// Claude scorer hvert event mot profilen. Kalles kun når event-listen
/// endrer seg (hash-cache i redb).
pub async fn curate_events(
    client: &reqwest::Client,
    api_key: &str,
    events: &[schema::EventItem],
) -> Result<schema::EventCurationBatch> {
    let settings = schemars::gen::SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = true;
    });
    let schema_value = serde_json::to_value(
        settings
            .into_generator()
            .into_root_schema_for::<schema::EventCurationBatch>(),
    )?;

    let mut prompt = format!(
        "Interesseprofil:\n{EVENT_PROFILE}\n\nVurder relevansen (1-5) for hvert \
         event under. Vær streng: 4-5 er unntaket og skal samlet være maks 6-7 \
         events — de som faktisk ville fått dem til å si «dit må vi». Er flere \
         events nesten like (samme festival/serie), gi kun det beste av dem 4+, \
         resten 3. why_no kun for relevans 4-5: én kort setning på norsk om \
         hvorfor dette treffer. Ett output-item per event, url uendret.\n\n"
    );
    for e in events {
        prompt.push_str(&format!("- {} | {} | {}\n  url: {}\n", e.title, e.start, e.venue, e.url));
    }

    let body = json!({
        "model": MODEL,
        "max_tokens": 4000,
        "messages": [{"role": "user", "content": prompt}],
        "tools": [{
            "name": "emit_curation",
            "description": "Lever event-vurderingen",
            "input_schema": schema_value,
        }],
        "tool_choice": {"type": "tool", "name": "emit_curation"},
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .timeout(std::time::Duration::from_secs(120))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .context("POST event-kuratering til Claude API")?;

    let status = resp.status();
    let value: Value = resp.json().await.context("les Claude-svar")?;
    if !status.is_success() {
        bail!("Claude API ga {status}: {value}");
    }
    let input = value["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"] == "tool_use"))
        .map(|b| b["input"].clone())
        .context("ingen tool_use-blokk i kurateringen")?;
    Ok(serde_json::from_value(input).context("parse EventCurationBatch")?)
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
            detail_no: raw.full_text.clone().unwrap_or_default(),
            category: Category::Other,
            urgency: 2,
            source_url: raw.url.clone(),
        },
        source: raw.source.clone(),
        published_at: raw.published,
    }
}
