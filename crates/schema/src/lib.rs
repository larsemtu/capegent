//! Delte typer. `NewsItem`/`NewsBatch` er Claude-outputtypene og genererer
//! JSON Schema via schemars. Resten er ren serde for `data/latest.json`.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Crime,
    Traffic,
    Weather,
    Politics,
    Event,
    Infrastructure,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NewsItem {
    /// Overskrift oversatt til norsk
    pub headline_no: String,
    /// Norsk sammendrag, maks 2 setninger
    pub summary_no: String,
    pub category: Category,
    /// 1 (uviktig) til 5 (kritisk for beboere i Cape Town)
    pub urgency: u8,
    /// Kilde-URL, uendret fra input
    pub source_url: String,
}

/// Wrapper fordi Anthropic-tools krever et objekt som rot-schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NewsBatch {
    pub items: Vec<NewsItem>,
}

/// Det som faktisk lagres i latest.json: LLM-output pluss metadata vi
/// kobler på selv (matchet på source_url).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsEntry {
    #[serde(flatten)]
    pub item: NewsItem,
    pub source: String,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weather {
    pub temp_c: f64,
    pub apparent_temp_c: f64,
    pub precipitation_mm: f64,
    pub weather_code: u8,
    pub wind_speed_kmh: f64,
    pub wind_direction_deg: f64,
    pub wind_gusts_kmh: f64,
    pub daily: Vec<DailyForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyForecast {
    /// ISO-dato i lokal tid (Africa/Johannesburg)
    pub date: String,
    pub temp_min_c: f64,
    pub temp_max_c: f64,
    pub weather_code: u8,
    pub precipitation_probability_pct: Option<f64>,
    pub wind_speed_max_kmh: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfSpot {
    pub name: String,
    pub wave_height_m: f64,
    pub swell_height_m: f64,
    pub swell_period_s: f64,
    pub swell_direction_deg: f64,
    pub wind_speed_kmh: f64,
    pub wind_direction_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadShedding {
    /// 0 = ingen load shedding
    pub stage: i32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// ISO 8601, lokal tid
    pub start: String,
    pub title: String,
    pub all_day: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventItem {
    pub title: String,
    pub date: String,
    pub venue: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardData {
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub weather: Option<Weather>,
    #[serde(default)]
    pub surf: Vec<SurfSpot>,
    #[serde(default)]
    pub news: Vec<NewsEntry>,
    #[serde(default)]
    pub load_shedding: Option<LoadShedding>,
    #[serde(default)]
    pub calendar: Vec<CalendarEvent>,
    #[serde(default)]
    pub events: Vec<EventItem>,
}
