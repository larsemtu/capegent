use anyhow::{Context, Result};
use schema::{DailyForecast, HourlyForecast, Weather};
use serde::Deserialize;

pub const GREEN_POINT: (f64, f64) = (-33.906, 18.410);

#[derive(Deserialize)]
struct ForecastResp {
    current: Current,
    hourly: Hourly,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    apparent_temperature: f64,
    precipitation: f64,
    weather_code: u8,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
    wind_gusts_10m: f64,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    weather_code: Vec<u8>,
    precipitation_probability: Vec<Option<f64>>,
    precipitation: Vec<f64>,
    wind_speed_10m: Vec<f64>,
    wind_direction_10m: Vec<f64>,
    wind_gusts_10m: Vec<f64>,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    weather_code: Vec<u8>,
    precipitation_probability_max: Vec<Option<f64>>,
    wind_speed_10m_max: Vec<f64>,
}

pub async fn fetch(client: &reqwest::Client) -> Result<Weather> {
    let (lat, lon) = GREEN_POINT;
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,apparent_temperature,precipitation,weather_code,\
         wind_speed_10m,wind_direction_10m,wind_gusts_10m\
         &hourly=temperature_2m,weather_code,precipitation_probability,precipitation,\
         wind_speed_10m,wind_direction_10m,wind_gusts_10m\
         &daily=temperature_2m_max,temperature_2m_min,weather_code,\
         precipitation_probability_max,wind_speed_10m_max\
         &timezone=Africa%2FJohannesburg&forecast_days=7&forecast_hours=48"
    );
    let resp: ForecastResp = client
        .get(&url)
        .send()
        .await
        .context("GET open-meteo forecast")?
        .error_for_status()?
        .json()
        .await
        .context("parse open-meteo forecast")?;

    let hourly = resp
        .hourly
        .time
        .iter()
        .enumerate()
        .map(|(i, time)| HourlyForecast {
            time: time.clone(),
            temp_c: resp.hourly.temperature_2m[i],
            weather_code: resp.hourly.weather_code[i],
            precipitation_probability_pct: resp.hourly.precipitation_probability[i],
            precipitation_mm: resp.hourly.precipitation[i],
            wind_speed_kmh: resp.hourly.wind_speed_10m[i],
            wind_direction_deg: resp.hourly.wind_direction_10m[i],
            wind_gusts_kmh: resp.hourly.wind_gusts_10m[i],
        })
        .collect();

    let daily = resp
        .daily
        .time
        .iter()
        .enumerate()
        .map(|(i, date)| DailyForecast {
            date: date.clone(),
            temp_min_c: resp.daily.temperature_2m_min[i],
            temp_max_c: resp.daily.temperature_2m_max[i],
            weather_code: resp.daily.weather_code[i],
            precipitation_probability_pct: resp.daily.precipitation_probability_max[i],
            wind_speed_max_kmh: resp.daily.wind_speed_10m_max[i],
        })
        .collect();

    Ok(Weather {
        temp_c: resp.current.temperature_2m,
        apparent_temp_c: resp.current.apparent_temperature,
        precipitation_mm: resp.current.precipitation,
        weather_code: resp.current.weather_code,
        wind_speed_kmh: resp.current.wind_speed_10m,
        wind_direction_deg: resp.current.wind_direction_10m,
        wind_gusts_kmh: resp.current.wind_gusts_10m,
        hourly,
        daily,
    })
}
