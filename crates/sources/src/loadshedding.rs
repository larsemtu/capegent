use anyhow::{Context, Result};
use schema::LoadShedding;
use serde::Deserialize;

#[derive(Deserialize)]
struct StatusResp {
    status: Status,
}

#[derive(Deserialize)]
struct Status {
    capetown: AreaStatus,
}

#[derive(Deserialize)]
struct AreaStatus {
    stage: String,
    #[serde(default)]
    stage_updated: Option<String>,
}

/// EskomSePush-status for Cape Town. Returnerer Ok(None) uten token,
/// slik at lokal kjøring fungerer uten oppsett.
/// Gratis-kvoten er 50 kall/døgn. Load shedding har vært suspendert siden
/// april 2025, så i fredstid sjekkes det bare hver 6. time — men står
/// forrige status på stage > 0, sjekkes det hver time. `active` = forrige
/// kjente stage var > 0. Ok(None) betyr «gjenbruk forrige verdi».
pub async fn fetch(client: &reqwest::Client, active: bool) -> Result<Option<LoadShedding>> {
    let Some(token) = crate::env_nonempty("SEPUSH_TOKEN") else {
        return Ok(None);
    };
    use chrono::Timelike;
    let now = chrono::Utc::now();
    let due = if active {
        now.minute() < 15 // hver time
    } else {
        now.minute() < 15 && now.hour() % 6 == 0 // 4 ganger i døgnet
    };
    if !due && crate::env_nonempty("SEPUSH_FORCE").is_none() {
        return Ok(None);
    }
    let resp: StatusResp = client
        .get("https://developer.sepush.co.za/business/2.0/status")
        .header("token", token)
        .send()
        .await
        .context("GET sepush status")?
        .error_for_status()?
        .json()
        .await
        .context("parse sepush status")?;

    let stage: i32 = resp.status.capetown.stage.parse().unwrap_or(0);
    let note = match resp.status.capetown.stage_updated {
        Some(ts) => format!("Siste endring {}", &ts[..10.min(ts.len())]),
        None => String::new(),
    };
    Ok(Some(LoadShedding { stage, note }))
}
