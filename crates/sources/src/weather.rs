//! Vær fra MET Norway (Yr) Locationforecast 2.0 — nøyaktig samme data som
//! yr.no viser for Cape Town. Gratis, CC-BY 4.0, krever identifiserende
//! User-Agent (satt på klienten). Tider fra API-et er UTC; vi konverterer
//! til SAST (fast UTC+2, ingen sommertid).

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, Timelike, Utc};
use schema::{DayForecast, HourlyForecast, PeriodForecast, Weather};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const GREEN_POINT: (f64, f64) = (-33.906, 18.410);

fn sast() -> FixedOffset {
    FixedOffset::east_opt(2 * 3600).unwrap()
}

#[derive(Deserialize)]
struct MetResp {
    properties: Properties,
}

#[derive(Deserialize)]
struct Properties {
    timeseries: Vec<Point>,
}

#[derive(Deserialize)]
struct Point {
    time: DateTime<Utc>,
    data: PointData,
}

#[derive(Deserialize)]
struct PointData {
    instant: Instant,
    next_1_hours: Option<NextHours>,
    next_6_hours: Option<NextHours>,
}

#[derive(Deserialize)]
struct Instant {
    details: InstantDetails,
}

#[derive(Deserialize)]
struct InstantDetails {
    air_temperature: f64,
    wind_speed: f64,
    wind_from_direction: f64,
}

#[derive(Deserialize)]
struct NextHours {
    summary: Option<Summary>,
    details: Option<NextDetails>,
}

#[derive(Deserialize)]
struct Summary {
    symbol_code: String,
}

#[derive(Deserialize, Default)]
struct NextDetails {
    #[serde(default)]
    precipitation_amount: f64,
}

/// MET-symbolkoder har _day/_night/_polartwilight-suffiks — stripp dem,
/// frontend velger visning selv.
fn base_symbol(code: &str) -> String {
    code.split('_').next().unwrap_or(code).to_string()
}

pub async fn fetch(client: &reqwest::Client) -> Result<Weather> {
    let (lat, lon) = GREEN_POINT;
    let url = format!(
        "https://api.met.no/weatherapi/locationforecast/2.0/complete?lat={lat}&lon={lon}"
    );
    let resp: MetResp = client
        .get(&url)
        .send()
        .await
        .context("GET met.no locationforecast")?
        .error_for_status()?
        .json()
        .await
        .context("parse met.no locationforecast")?;

    let ts = &resp.properties.timeseries;
    let first = ts.first().context("tom timeserie fra met.no")?;

    // Time-for-time så lenge next_1_hours finnes (MET gir ~60-70 t)
    let hourly: Vec<HourlyForecast> = ts
        .iter()
        .filter_map(|p| {
            let n1 = p.data.next_1_hours.as_ref()?;
            let local = p.time.with_timezone(&sast());
            Some(HourlyForecast {
                time: local.format("%Y-%m-%dT%H:%M").to_string(),
                temp_c: p.data.instant.details.air_temperature,
                symbol: n1
                    .summary
                    .as_ref()
                    .map(|s| base_symbol(&s.symbol_code))
                    .unwrap_or_default(),
                precipitation_mm: n1
                    .details
                    .as_ref()
                    .map(|d| d.precipitation_amount)
                    .unwrap_or(0.0),
                wind_ms: p.data.instant.details.wind_speed,
                wind_direction_deg: p.data.instant.details.wind_from_direction,
            })
        })
        .collect();

    // Dager: grupper på lokal dato. Fire 6-timersperioder per dag — første
    // punkt i hver bøtte [0-6, 6-12, 12-18, 18-24) som har next_6_hours.
    let mut by_day: BTreeMap<String, Vec<&Point>> = BTreeMap::new();
    for p in ts {
        let local = p.time.with_timezone(&sast());
        by_day
            .entry(local.format("%Y-%m-%d").to_string())
            .or_default()
            .push(p);
    }

    let days: Vec<DayForecast> = by_day
        .into_iter()
        .map(|(date, points)| {
            let temps: Vec<f64> = points
                .iter()
                .map(|p| p.data.instant.details.air_temperature)
                .collect();
            let wind_max = points
                .iter()
                .map(|p| p.data.instant.details.wind_speed)
                .fold(0.0_f64, f64::max);

            let mut periods = vec![];
            for bucket in 0..4u8 {
                let (lo, hi) = (bucket * 6, bucket * 6 + 6);
                let point = points.iter().find(|p| {
                    let h = p.time.with_timezone(&sast()).hour() as u8;
                    h >= lo && h < hi && p.data.next_6_hours.is_some()
                });
                let Some(p) = point else { continue };
                let n6 = p.data.next_6_hours.as_ref().unwrap();
                periods.push(PeriodForecast {
                    from_hour: p.time.with_timezone(&sast()).hour() as u8,
                    symbol: n6
                        .summary
                        .as_ref()
                        .map(|s| base_symbol(&s.symbol_code))
                        .unwrap_or_default(),
                    temp_c: p.data.instant.details.air_temperature,
                    precipitation_mm: n6
                        .details
                        .as_ref()
                        .map(|d| d.precipitation_amount)
                        .unwrap_or(0.0),
                    wind_ms: p.data.instant.details.wind_speed,
                    wind_direction_deg: p.data.instant.details.wind_from_direction,
                });
            }
            let precip_total = periods.iter().map(|p| p.precipitation_mm).sum();

            DayForecast {
                date,
                temp_min_c: temps.iter().copied().fold(f64::INFINITY, f64::min),
                temp_max_c: temps.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                precipitation_mm_total: precip_total,
                wind_max_ms: wind_max,
                periods,
            }
        })
        // Dager helt uten perioder (halespiss av serien) er ikke visbare
        .filter(|d| !d.periods.is_empty())
        .collect();

    let current_symbol = first
        .data
        .next_1_hours
        .as_ref()
        .or(first.data.next_6_hours.as_ref())
        .and_then(|n| n.summary.as_ref())
        .map(|s| base_symbol(&s.symbol_code))
        .unwrap_or_default();

    Ok(Weather {
        temp_c: first.data.instant.details.air_temperature,
        symbol: current_symbol,
        precipitation_mm: first
            .data
            .next_1_hours
            .as_ref()
            .and_then(|n| n.details.as_ref())
            .map(|d| d.precipitation_amount)
            .unwrap_or(0.0),
        wind_ms: first.data.instant.details.wind_speed,
        wind_direction_deg: first.data.instant.details.wind_from_direction,
        hourly,
        days,
    })
}
