use anyhow::{Context, Result};
use schema::{SurfHour, SurfRating, SurfSpot};
use serde::Deserialize;

/// Spot-profil for kvalitetsvurdering. `offshore_deg` er vindretningen som
/// gir offshore (vind FRA den retningen blåser mot havet). Cape Doctor (SØ)
/// ødelegger vestkyst-spotene, derfor er retning viktigere enn styrke.
pub struct SpotDef {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub offshore_deg: f64,
    /// Svellhøyde-intervall der spoten fungerer best (meter, målt ved spoten)
    pub swell_min: f64,
    pub swell_best_max: f64,
}

pub const SPOTS: &[SpotDef] = &[
    SpotDef { name: "Muizenberg", lat: -34.108, lon: 18.470, offshore_deg: 315.0, swell_min: 0.5, swell_best_max: 2.0 },
    SpotDef { name: "Big Bay", lat: -33.795, lon: 18.457, offshore_deg: 110.0, swell_min: 0.8, swell_best_max: 2.5 },
    SpotDef { name: "Llandudno", lat: -34.006, lon: 18.341, offshore_deg: 90.0, swell_min: 1.0, swell_best_max: 3.0 },
];

/// Surfline-inspirert rating: svellhøyde (0-4) + periode (0-3) + vind (0-3),
/// bucketet til poor→good. Bevisst enkel og deterministisk.
pub fn rate(
    spot: &SpotDef,
    swell_m: f64,
    period_s: f64,
    wind_kmh: f64,
    wind_deg: f64,
) -> SurfRating {
    // Svellhøyde: ramp opp mot optimalt intervall, straff over
    let swell_score = if swell_m < 0.3 {
        0.0
    } else if swell_m < spot.swell_min {
        1.5 * (swell_m - 0.3) / (spot.swell_min - 0.3)
    } else if swell_m <= spot.swell_best_max {
        4.0
    } else if swell_m <= spot.swell_best_max + 1.5 {
        2.5
    } else {
        1.0
    };

    // Periode: groundswell (11s+) er gull, vindsjø (<8s) er rot
    let period_score = if period_s >= 13.0 {
        3.0
    } else if period_s >= 11.0 {
        2.5
    } else if period_s >= 9.0 {
        2.0
    } else if period_s >= 7.0 {
        1.0
    } else {
        0.3
    };

    // Vind: retning relativt til offshore
    let mut diff = (wind_deg - spot.offshore_deg).abs() % 360.0;
    if diff > 180.0 {
        diff = 360.0 - diff;
    }
    let wind_score = if wind_kmh < 8.0 {
        2.7 // glassy uansett retning
    } else if diff <= 45.0 {
        if wind_kmh < 35.0 { 3.0 } else { 1.5 } // offshore
    } else if diff <= 90.0 {
        if wind_kmh < 20.0 { 2.0 } else { 1.0 } // cross
    } else if wind_kmh < 15.0 {
        1.2 // svak onshore
    } else if wind_kmh < 25.0 {
        0.5
    } else {
        0.0 // frisk onshore = vaskemaskin
    };

    let total = swell_score + period_score + wind_score;
    match total {
        t if t < 3.0 => SurfRating::Poor,
        t if t < 4.5 => SurfRating::PoorFair,
        t if t < 6.0 => SurfRating::Fair,
        t if t < 7.5 => SurfRating::FairGood,
        _ => SurfRating::Good,
    }
}

#[derive(Deserialize)]
struct MarineResp {
    current: MarineCurrent,
    hourly: MarineHourly,
}

#[derive(Deserialize)]
struct MarineCurrent {
    wave_height: f64,
    swell_wave_height: f64,
    swell_wave_period: f64,
    swell_wave_direction: f64,
}

#[derive(Deserialize)]
struct MarineHourly {
    time: Vec<String>,
    swell_wave_height: Vec<f64>,
    swell_wave_period: Vec<f64>,
    swell_wave_direction: Vec<f64>,
}

#[derive(Deserialize)]
struct WindResp {
    current: WindCurrent,
    hourly: WindHourly,
}

#[derive(Deserialize)]
struct WindCurrent {
    wind_speed_10m: f64,
    wind_direction_10m: f64,
}

#[derive(Deserialize)]
struct WindHourly {
    time: Vec<String>,
    wind_speed_10m: Vec<f64>,
    wind_direction_10m: Vec<f64>,
}

/// Bølgedata fra marine-API-et + vind fra forecast-API-et for samme punkt,
/// nå + 48 timer frem, med beregnet rating per time.
pub async fn fetch_spot(client: &reqwest::Client, spot: &SpotDef) -> Result<SurfSpot> {
    let marine_url = format!(
        "https://marine-api.open-meteo.com/v1/marine?latitude={}&longitude={}\
         &current=wave_height,swell_wave_height,swell_wave_period,swell_wave_direction\
         &hourly=swell_wave_height,swell_wave_period,swell_wave_direction\
         &timezone=Africa%2FJohannesburg&forecast_hours=48",
        spot.lat, spot.lon
    );
    let wind_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
         &current=wind_speed_10m,wind_direction_10m\
         &hourly=wind_speed_10m,wind_direction_10m\
         &timezone=Africa%2FJohannesburg&forecast_hours=48",
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

    // Time-seriene skal være parallelle (samme API, samme parametre), men
    // zip er robust hvis lengdene skulle avvike
    let hourly: Vec<SurfHour> = marine
        .hourly
        .time
        .iter()
        .enumerate()
        .filter_map(|(i, time)| {
            let (Some(&sh), Some(&sp), Some(&sd)) = (
                marine.hourly.swell_wave_height.get(i),
                marine.hourly.swell_wave_period.get(i),
                marine.hourly.swell_wave_direction.get(i),
            ) else {
                return None;
            };
            // Vind-serien kan starte på et annet klokkeslett — match på tid
            let wi = wind.hourly.time.iter().position(|t| t == time)?;
            let (ws, wd) = (wind.hourly.wind_speed_10m[wi], wind.hourly.wind_direction_10m[wi]);
            Some(SurfHour {
                time: time.clone(),
                swell_height_m: sh,
                swell_period_s: sp,
                swell_direction_deg: sd,
                wind_speed_kmh: ws,
                wind_direction_deg: wd,
                rating: rate(spot, sh, sp, ws, wd),
            })
        })
        .collect();

    Ok(SurfSpot {
        name: spot.name.to_string(),
        wave_height_m: marine.current.wave_height,
        swell_height_m: marine.current.swell_wave_height,
        swell_period_s: marine.current.swell_wave_period,
        swell_direction_deg: marine.current.swell_wave_direction,
        wind_speed_kmh: wind.current.wind_speed_10m,
        wind_direction_deg: wind.current.wind_direction_10m,
        rating: rate(
            spot,
            marine.current.swell_wave_height,
            marine.current.swell_wave_period,
            wind.current.wind_speed_10m,
            wind.current.wind_direction_10m,
        ),
        hourly,
    })
}
