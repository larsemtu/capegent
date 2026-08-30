//! Vær fra MET Norway (Yr) Locationforecast 2.0 — nøyaktig samme data som
//! yr.no viser for Cape Town. Gratis, CC-BY 4.0, krever identifiserende
//! User-Agent (satt på klienten). Tider fra API-et er UTC; vi konverterer
//! til SAST (fast UTC+2, ingen sommertid).

use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::warn;
use chrono::{DateTime, FixedOffset, Timelike, Utc};
use schema::{DayForecast, HourlyForecast, PeriodForecast, Weather};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const GREEN_POINT: (f64, f64) = (-33.906, 18.410);

fn sast() -> FixedOffset {
    FixedOffset::east_opt(2 * 3600).unwrap()
}

#[derive(Deserialize)]
struct OmResp {
    hourly: OmHourly,
    daily: OmDaily,
}

#[derive(Deserialize)]
struct OmHourly {
    time: Vec<String>,
    wind_gusts_10m: Vec<Option<f64>>,
    #[serde(default)]
    uv_index: Vec<Option<f64>>,
}

#[derive(Deserialize)]
struct OmDaily {
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

struct OmSupplement {
    gusts: HashMap<String, f64>,
    uv_today: Vec<schema::UvPoint>,
    sunrise: Option<String>,
    sunset: Option<String>,
}

/// Kast, UV og sol finnes ikke i MET-data utenfor Norden — suppler fra
/// Open-Meteo i ett kall. Best effort: feiler dette, mangler bare disse.
async fn fetch_supplement(client: &reqwest::Client) -> Result<OmSupplement> {
    let (lat, lon) = GREEN_POINT;
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &hourly=wind_gusts_10m,uv_index&wind_speed_unit=ms\
         &daily=sunrise,sunset\
         &timezone=Africa%2FJohannesburg&forecast_days=3"
    );
    let resp: OmResp = client.get(&url).send().await?.error_for_status()?.json().await?;
    let today = resp
        .daily
        .sunrise
        .first()
        .map(|s| s[..10].to_string())
        .unwrap_or_default();
    let uv_today = resp
        .hourly
        .time
        .iter()
        .zip(&resp.hourly.uv_index)
        .filter(|(t, _)| t.starts_with(&today))
        .filter_map(|(t, uv)| {
            Some(schema::UvPoint { time: t[11..16].to_string(), uv: (*uv)? })
        })
        .collect();
    let gusts = resp
        .hourly
        .time
        .into_iter()
        .zip(resp.hourly.wind_gusts_10m)
        .filter_map(|(t, g)| Some((t, g?)))
        .collect();
    Ok(OmSupplement {
        gusts,
        uv_today,
        sunrise: resp.daily.sunrise.first().map(|s| s[11..16].to_string()),
        sunset: resp.daily.sunset.first().map(|s| s[11..16].to_string()),
    })
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
    let (met_res, gusts_res) = tokio::join!(
        async {
            client
                .get(&url)
                .send()
                .await
                .context("GET met.no locationforecast")?
                .error_for_status()?
                .json::<MetResp>()
                .await
                .context("parse met.no locationforecast")
        },
        fetch_supplement(client)
    );
    let resp = met_res?;
    let supplement = gusts_res.unwrap_or_else(|e| {
        warn!("Open-Meteo-supplement (kast/UV/sol) feilet: {e:#}");
        OmSupplement { gusts: HashMap::new(), uv_today: vec![], sunrise: None, sunset: None }
    });
    let gusts = supplement.gusts;

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
                gust_ms: gusts
                    .get(&local.format("%Y-%m-%dT%H:%M").to_string())
                    .copied(),
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
        sunrise: supplement.sunrise,
        sunset: supplement.sunset,
        uv_today: supplement.uv_today,
        hourly,
        days,
    })
}
