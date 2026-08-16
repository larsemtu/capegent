//! Kronekurs fra Frankfurter (ECB, gratis, uten nøkkel). LAR-25.
//! Primærvisning: 100 ZAR -> NOK (reiseperspektivet: «hva koster randen»).

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use schema::{Currency, FxPoint};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Resp {
    rates: BTreeMap<String, Rate>,
}

#[derive(Deserialize)]
struct Rate {
    #[serde(rename = "NOK")]
    nok: f64,
}

pub async fn fetch(client: &reqwest::Client) -> Result<Currency> {
    let from = (Utc::now() - Duration::days(32)).format("%Y-%m-%d");
    let url = format!("https://api.frankfurter.dev/v1/{from}..?base=ZAR&symbols=NOK");
    let resp: Resp = client
        .get(&url)
        .send()
        .await
        .context("GET frankfurter")?
        .error_for_status()?
        .json()
        .await
        .context("parse frankfurter")?;

    let series: Vec<FxPoint> = resp
        .rates
        .into_iter()
        // 100 ZAR i NOK
        .map(|(date, r)| FxPoint { date, rate: r.nok * 100.0 })
        .collect();
    let (first, last) = match (series.first(), series.last()) {
        (Some(f), Some(l)) => (f.rate, l.rate),
        _ => anyhow::bail!("tom kursserie"),
    };
    Ok(Currency {
        rate: last,
        change_pct_30d: (last - first) / first * 100.0,
        updated: series.last().map(|p| p.date.clone()).unwrap_or_default(),
        series,
    })
}
