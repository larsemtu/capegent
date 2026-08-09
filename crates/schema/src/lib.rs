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

impl Category {
    /// Sorteringsrang for dashbordet: krim/vold først, så samfunnsstoff,
    /// deretter hverdag.
    pub fn rank(self) -> u8 {
        match self {
            Category::Crime => 0,
            Category::Politics | Category::Infrastructure | Category::Traffic => 1,
            Category::Weather => 2,
            Category::Event => 3,
            Category::Other => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NewsItem {
    /// Overskrift oversatt til norsk
    pub headline_no: String,
    /// Norsk sammendrag, maks 2 setninger (oversiktsvisning)
    pub summary_no: String,
    /// Utfyllende norsk sammendrag, 4-8 setninger basert på hele
    /// artikkelteksten (detaljvisning)
    pub detail_no: String,
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

/// Vær fra MET Norway (Yr) Locationforecast 2.0 — samme data som yr.no.
/// All vind i m/s. Symbol er MET-symbolkode uten _day/_night-suffiks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weather {
    pub temp_c: f64,
    pub symbol: String,
    /// Nedbør neste time
    pub precipitation_mm: f64,
    pub wind_ms: f64,
    pub wind_direction_deg: f64,
    /// Time for time så langt MET leverer timesoppløsning (~60-70 t)
    #[serde(default)]
    pub hourly: Vec<HourlyForecast>,
    /// Alle dager (~9-10) med fire 6-timersperioder per dag, yr-stil
    #[serde(default)]
    pub days: Vec<DayForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyForecast {
    /// ISO 8601 lokal tid (Africa/Johannesburg)
    pub time: String,
    pub temp_c: f64,
    pub symbol: String,
    pub precipitation_mm: f64,
    pub wind_ms: f64,
    pub wind_direction_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayForecast {
    /// ISO-dato i lokal tid
    pub date: String,
    pub temp_min_c: f64,
    pub temp_max_c: f64,
    /// Summert nedbør for hele dagen — skiller regnværsdag fra korte byger
    pub precipitation_mm_total: f64,
    pub wind_max_ms: f64,
    /// Inntil fire 6-timersperioder
    pub periods: Vec<PeriodForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodForecast {
    /// Lokal starttime for perioden (f.eks. 0, 6, 12, 18)
    pub from_hour: u8,
    pub symbol: String,
    pub temp_c: f64,
    pub precipitation_mm: f64,
    pub wind_ms: f64,
    pub wind_direction_deg: f64,
}

/// Surfline-inspirert kvalitetsvurdering, beregnet fra svell + vind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfRating {
    Poor,
    PoorFair,
    Fair,
    FairGood,
    Good,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfSpot {
    pub name: String,
    pub wave_height_m: f64,
    pub swell_height_m: f64,
    pub swell_period_s: f64,
    pub swell_direction_deg: f64,
    pub wind_ms: f64,
    pub wind_direction_deg: f64,
    pub rating: SurfRating,
    /// Neste 48 timer for detaljvisning
    #[serde(default)]
    pub hourly: Vec<SurfHour>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfHour {
    pub time: String,
    pub swell_height_m: f64,
    pub swell_period_s: f64,
    pub swell_direction_deg: f64,
    pub wind_ms: f64,
    pub wind_direction_deg: f64,
    pub rating: SurfRating,
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
    /// ISO 8601 starttidspunkt
    pub start: String,
    pub venue: String,
    pub url: String,
}

/// Gjøremål — fylles fra Linear når integrasjonen kobles på.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub title: String,
    /// f.eks. "Todo", "In Progress"
    pub status: String,
    pub project: Option<String>,
    /// ISO-dato
    pub due: Option<String>,
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
    #[serde(default)]
    pub todos: Vec<TodoItem>,
}
