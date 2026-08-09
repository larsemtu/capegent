use anyhow::{Context, Result};
use chrono::{Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};
use ical::IcalParser;
use schema::CalendarEvent;
use std::io::BufReader;

/// Cape Town har ikke sommertid, fast UTC+2 holder.
fn sast() -> FixedOffset {
    FixedOffset::east_opt(2 * 3600).unwrap()
}

/// Google Calendar iCal-feed (privat URL i CALENDAR_ICS_URL).
/// Returnerer de neste 5 kommende hendelsene. Ok(vec![]) uten URL.
/// Begrensning v1: RRULE (gjentakende hendelser) ekspanderes ikke.
pub async fn fetch(client: &reqwest::Client) -> Result<Vec<CalendarEvent>> {
    let Some(url) = crate::env_nonempty("CALENDAR_ICS_URL") else {
        return Ok(vec![]);
    };
    let body = client
        .get(&url)
        .send()
        .await
        .context("GET iCal-feed")?
        .error_for_status()?
        .text()
        .await?;

    let now_local = Utc::now().with_timezone(&sast());
    let today = now_local.date_naive();
    let mut events: Vec<(NaiveDateTime, CalendarEvent)> = vec![];

    for cal in IcalParser::new(BufReader::new(body.as_bytes())) {
        let cal = cal.context("parse iCal")?;
        for event in cal.events {
            let mut title = String::new();
            let mut start: Option<(NaiveDateTime, bool)> = None;
            for prop in &event.properties {
                match prop.name.as_str() {
                    "SUMMARY" => title = prop.value.clone().unwrap_or_default(),
                    "DTSTART" => start = prop.value.as_deref().and_then(parse_dtstart),
                    _ => {}
                }
            }
            let Some((dt, all_day)) = start else { continue };
            if title.is_empty() || dt.date() < today {
                continue;
            }
            let iso = if all_day {
                dt.date().to_string()
            } else {
                dt.format("%Y-%m-%dT%H:%M").to_string()
            };
            events.push((dt, CalendarEvent { start: iso, title, all_day }));
        }
    }

    // Kun neste 30 dager, ellers fyller fjerne heldagshendelser panelet
    let horizon = today + Duration::days(30);
    events.retain(|(dt, _)| dt.date() <= horizon);
    events.sort_by_key(|(dt, _)| *dt);
    Ok(events.into_iter().take(5).map(|(_, e)| e).collect())
}

/// Håndterer de tre vanlige DTSTART-formene fra Google Calendar:
/// "20260810" (heldag), "20260810T120000Z" (UTC), "20260810T120000" (lokal/TZID).
fn parse_dtstart(value: &str) -> Option<(NaiveDateTime, bool)> {
    if let Some(stripped) = value.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S").ok()?;
        let local = Utc.from_utc_datetime(&naive).with_timezone(&sast());
        return Some((local.naive_local(), false));
    }
    if value.contains('T') {
        return NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| (dt, false));
    }
    NaiveDate::parse_from_str(value, "%Y%m%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| (dt, true))
}
