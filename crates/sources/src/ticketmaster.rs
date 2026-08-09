//! Store konserter/sport fra Ticketmaster Discovery API (Big Concerts m.fl.
//! selger via Ticketmaster i Sør-Afrika). Gratis API-nøkkel fra
//! developer.ticketmaster.com i TICKETMASTER_API_KEY. Hopper over uten nøkkel.

use anyhow::{Context, Result};
use schema::EventItem;
use serde::Deserialize;

#[derive(Deserialize)]
struct Resp {
    #[serde(rename = "_embedded")]
    embedded: Option<Embedded>,
}

#[derive(Deserialize)]
struct Embedded {
    events: Vec<TmEvent>,
}

#[derive(Deserialize)]
struct TmEvent {
    name: String,
    url: Option<String>,
    dates: Dates,
    #[serde(rename = "_embedded")]
    embedded: Option<TmEventEmbedded>,
}

#[derive(Deserialize)]
struct Dates {
    start: Start,
}

#[derive(Deserialize)]
struct Start {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    #[serde(rename = "localDate")]
    local_date: Option<String>,
}

#[derive(Deserialize)]
struct TmEventEmbedded {
    venues: Option<Vec<Venue>>,
}

#[derive(Deserialize)]
struct Venue {
    name: Option<String>,
}

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<EventItem>> {
    let Some(key) = crate::env_nonempty("TICKETMASTER_API_KEY") else {
        return Ok(vec![]);
    };
    let url = format!(
        "https://app.ticketmaster.com/discovery/v2/events.json?apikey={key}\
         &city=Cape%20Town&countryCode=ZA&size=50&sort=date,asc"
    );
    let resp: Resp = client
        .get(&url)
        .send()
        .await
        .context("GET ticketmaster")?
        .error_for_status()?
        .json()
        .await
        .context("parse ticketmaster")?;

    Ok(resp
        .embedded
        .map(|e| e.events)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let start = e
                .dates
                .start
                .date_time
                .or_else(|| e.dates.start.local_date.map(|d| format!("{d}T00:00:00Z")))?;
            Some(EventItem {
                title: e.name,
                start,
                venue: e
                    .embedded
                    .and_then(|em| em.venues)
                    .and_then(|v| v.into_iter().next())
                    .and_then(|v| v.name)
                    .unwrap_or_default(),
                url: e.url.unwrap_or_default(),
                relevance: None,
                why_no: None,
            })
        })
        .collect())
}
