//! NOK/ZAR fra Frankfurter (ECB-kurser, gratis, uten nøkkel). LAR-25.

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
    #[serde(rename = "ZAR")]
    zar: f64,
}

pub async fn fetch(client: &reqwest::Client) -> Result<Currency> {
    let from = (Utc::now() - Duration::days(32)).format("%Y-%m-%d");
    let url = format!("https://api.frankfurter.dev/v1/{from}..?base=NOK&symbols=ZAR");
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
        .map(|(date, r)| FxPoint { date, rate: r.zar })
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
