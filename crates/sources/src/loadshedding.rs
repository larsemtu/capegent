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
pub async fn fetch(client: &reqwest::Client) -> Result<Option<LoadShedding>> {
    let Some(token) = crate::env_nonempty("SEPUSH_TOKEN") else {
        return Ok(None);
    };
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
        Some(ts) => format!("Oppdatert {ts}"),
        None => String::new(),
    };
    Ok(Some(LoadShedding { stage, note }))
}
