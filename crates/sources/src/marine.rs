use anyhow::{Context, Result};
use schema::SurfSpot;
use serde::Deserialize;

pub struct SpotDef {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
}

pub const SPOTS: &[SpotDef] = &[
    SpotDef { name: "Muizenberg", lat: -34.108, lon: 18.470 },
    SpotDef { name: "Big Bay", lat: -33.795, lon: 18.457 },
    SpotDef { name: "Llandudno", lat: -34.006, lon: 18.341 },
];

#[derive(Deserialize)]
struct MarineResp {
    current: MarineCurrent,
}

#[derive(Deserialize)]
struct MarineCurrent {
    wave_height: f64,
    swell_wave_height: f64,
    swell_wave_period: f64,
    swell_wave_direction: f64,
}

#[derive(Deserialize)]
struct WindResp {
    current: WindCurrent,
}

#[derive(Deserialize)]
struct WindCurrent {
    wind_speed_10m: f64,
    wind_direction_10m: f64,
}

/// Bølgedata fra marine-API-et + vind fra forecast-API-et for samme punkt.
/// Vindretningen avgjør om en spot fungerer, så den hentes per spot.
pub async fn fetch_spot(client: &reqwest::Client, spot: &SpotDef) -> Result<SurfSpot> {
    let marine_url = format!(
        "https://marine-api.open-meteo.com/v1/marine?latitude={}&longitude={}\
         &current=wave_height,swell_wave_height,swell_wave_period,swell_wave_direction\
         &timezone=Africa%2FJohannesburg",
        spot.lat, spot.lon
    );
    let wind_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
         &current=wind_speed_10m,wind_direction_10m&timezone=Africa%2FJohannesburg",
        spot.lat, spot.lon
    );

    let (marine, wind) = tokio::try_join!(
        async {
            client
                .get(&marine_url)
                .send()
                .await
                .with_context(|| format!("GET marine {}", spot.name))?
                .error_for_status()?
                .json::<MarineResp>()
                .await
                .with_context(|| format!("parse marine {}", spot.name))
        },
        async {
            client
                .get(&wind_url)
                .send()
                .await
                .with_context(|| format!("GET vind {}", spot.name))?
                .error_for_status()?
                .json::<WindResp>()
                .await
                .with_context(|| format!("parse vind {}", spot.name))
        }
    )?;

    Ok(SurfSpot {
        name: spot.name.to_string(),
        wave_height_m: marine.current.wave_height,
        swell_height_m: marine.current.swell_wave_height,
        swell_period_s: marine.current.swell_wave_period,
        swell_direction_deg: marine.current.swell_wave_direction,
        wind_speed_kmh: wind.current.wind_speed_10m,
        wind_direction_deg: wind.current.wind_direction_10m,
    })
}
