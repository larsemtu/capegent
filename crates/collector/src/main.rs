//! Orkestrering: hent alle kilder parallelt, dedup mot cache, send nye
//! artikler til Claude, skriv data/latest.json. Ved feil i én kilde brukes
//! forrige kjørings data for den delen — én død feed svarter aldri skjermen.

mod dedup;
mod llm;

use anyhow::Result;
use futures::future::join_all;
use schema::{DashboardData, NewsEntry, SurfSpot};
use sources::{calendar, loadshedding, marine, rss::RssSource, weather, RawItem, Source};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tracing::{info, warn};

const FEEDS: &[(&str, &str)] = &[
    // EWN droppet RSS ved siste redesign — derfor ikke med (verifisert 2026-08)
    ("groundup", "https://www.groundup.org.za/sitenews/rss/"),
    ("dailymaverick", "https://www.dailymaverick.co.za/dmrss/"),
    ("iol", "https://rss.iol.io/iol/news/south-africa/western-cape"),
];

const MAX_NEWS: usize = 20;
const LLM_BATCH: usize = 12;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; capetown-dashboard/0.1)")
        .timeout(Duration::from_secs(30))
        .build()?;

    let out_path = Path::new("data/latest.json");
    let previous: DashboardData = std::fs::read_to_string(out_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let cache = dedup::Cache::open(Path::new(".cache/dedup.redb"))?;

    let (weather_res, surf, loadshedding_res, calendar_res, raw_news) = tokio::join!(
        weather::fetch(&client),
        fetch_surf(&client),
        loadshedding::fetch(&client),
        calendar::fetch(&client),
        fetch_news(&client),
    );

    let news = process_news(&client, &cache, raw_news).await;

    let data = DashboardData {
        generated_at: chrono::Utc::now().to_rfc3339(),
        weather: match weather_res {
            Ok(w) => Some(w),
            Err(e) => {
                warn!("vær feilet, bruker forrige: {e:#}");
                previous.weather
            }
        },
        surf: if surf.is_empty() { previous.surf } else { surf },
        news: if news.is_empty() { previous.news } else { news },
        load_shedding: match loadshedding_res {
            Ok(ls) => ls.or(previous.load_shedding),
            Err(e) => {
                warn!("load shedding feilet, bruker forrige: {e:#}");
                previous.load_shedding
            }
        },
        calendar: match calendar_res {
            Ok(events) => events,
            Err(e) => {
                warn!("kalender feilet, bruker forrige: {e:#}");
                previous.calendar
            }
        },
        // Events-kilder (Quicket/Webtickets) kommer senere
        events: previous.events,
    };

    std::fs::create_dir_all("data")?;
    let mut json = serde_json::to_string_pretty(&data)?;
    json.push('\n');
    std::fs::write(out_path, json)?;
    info!(
        news = data.news.len(),
        surf = data.surf.len(),
        "skrev data/latest.json"
    );
    Ok(())
}

async fn fetch_surf(client: &reqwest::Client) -> Vec<SurfSpot> {
    let results = join_all(
        marine::SPOTS
            .iter()
            .map(|spot| marine::fetch_spot(client, spot)),
    )
    .await;
    results
        .into_iter()
        .filter_map(|r| r.map_err(|e| warn!("surf-spot feilet: {e:#}")).ok())
        .collect()
}

async fn fetch_news(client: &reqwest::Client) -> Vec<RawItem> {
    let sources: Vec<Box<dyn Source>> = FEEDS
        .iter()
        .map(|(id, url)| Box::new(RssSource::new(id, url)) as Box<dyn Source>)
        .collect();

    let results = join_all(sources.iter().map(|s| s.fetch(client))).await;

    let mut items = vec![];
    for (source, result) in sources.iter().zip(results) {
        match result {
            Ok(mut fetched) => {
                info!(source = source.id(), count = fetched.len(), "hentet feed");
                items.append(&mut fetched);
            }
            Err(e) => warn!(source = source.id(), "feed feilet: {e:#}"),
        }
    }
    items
}

async fn process_news(
    client: &reqwest::Client,
    cache: &dedup::Cache,
    mut raw: Vec<RawItem>,
) -> Vec<NewsEntry> {
    let mut seen = HashSet::new();
    raw.retain(|r| seen.insert(r.url.clone()));
    raw.sort_by(|a, b| b.published.cmp(&a.published));
    raw.truncate(MAX_NEWS + 10);

    let mut entries = vec![];
    let mut fresh = vec![];
    for item in raw {
        match cache.get(&item.url) {
            Some(entry) => entries.push(entry),
            None => fresh.push(item),
        }
    }
    info!(cached = entries.len(), new = fresh.len(), "dedup mot cache");

    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    for chunk in fresh.chunks(LLM_BATCH) {
        match &api_key {
            Some(key) => match llm::summarize(client, key, chunk).await {
                Ok(processed) => {
                    if let Err(e) = cache.put(&processed) {
                        warn!("cache-skriving feilet: {e:#}");
                    }
                    entries.extend(processed);
                }
                Err(e) => {
                    // Fallback vises, men caches ikke — prøves igjen neste kjøring
                    warn!("Claude-kall feilet, viser uoversatt: {e:#}");
                    entries.extend(chunk.iter().map(llm::fallback));
                }
            },
            None => entries.extend(chunk.iter().map(llm::fallback)),
        }
    }

    entries.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    entries.truncate(MAX_NEWS);
    entries
}
