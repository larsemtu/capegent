//! Google Calendar iCal-feeder (private URL-er i CALENDAR_ICS_URL,
//! kommaseparert for flere kalendere). Gjentakende avtaler (RRULE)
//! ekspanderes via rrule-craten. Returnerer de neste kommende hendelsene.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};
use ical::IcalParser;
use rrule::{RRuleSet, Tz as RTz};
use schema::CalendarEvent;
use std::io::BufReader;
use tracing::warn;

const HORIZON_DAYS: i64 = 30;
const MAX_EVENTS: usize = 6;

/// Cape Town har ikke sommertid, fast UTC+2 holder.
fn sast() -> FixedOffset {
    FixedOffset::east_opt(2 * 3600).unwrap()
}

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<CalendarEvent>> {
    let Some(urls) = crate::env_nonempty("CALENDAR_ICS_URL") else {
        return Ok(vec![]);
    };

    let mut events: Vec<(NaiveDateTime, CalendarEvent)> = vec![];
    for url in urls.split(',').map(str::trim).filter(|u| !u.is_empty()) {
        match fetch_one(client, url).await {
            Ok(mut evs) => events.append(&mut evs),
            Err(e) => warn!("kalenderfeed feilet: {e:#}"),
        }
    }

    events.sort_by_key(|(dt, _)| *dt);
    events.dedup_by(|a, b| a.1.start == b.1.start && a.1.title == b.1.title);
    Ok(events.into_iter().take(MAX_EVENTS).map(|(_, e)| e).collect())
}

async fn fetch_one(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<(NaiveDateTime, CalendarEvent)>> {
    let body = client
        .get(url)
        .send()
        .await
        .context("GET iCal-feed")?
        .error_for_status()?
        .text()
        .await?;
    Ok(parse_ics(&body))
}

/// Parser en ics-kropp til kommende hendelser. Skilt ut for testbarhet.
pub fn parse_ics(body: &str) -> Vec<(NaiveDateTime, CalendarEvent)> {
    let now_local = Utc::now().with_timezone(&sast());
    let today = now_local.date_naive();
    let horizon = today + Duration::days(HORIZON_DAYS);
    let mut events = vec![];

    for cal in IcalParser::new(BufReader::new(body.as_bytes())) {
        let Ok(cal) = cal else { continue };
        for event in cal.events {
            let mut title = String::new();
            let mut dtstart_raw: Option<String> = None;
            let mut rrule_raw: Option<String> = None;
            for prop in &event.properties {
                match prop.name.as_str() {
                    "SUMMARY" => title = prop.value.clone().unwrap_or_default(),
                    "DTSTART" => dtstart_raw = prop.value.clone(),
                    "RRULE" => rrule_raw = prop.value.clone(),
                    _ => {}
                }
            }
            let (Some(raw), false) = (dtstart_raw, title.is_empty()) else {
                continue;
            };
            let Some((first, all_day)) = parse_dtstart(&raw) else {
                continue;
            };

            let starts: Vec<NaiveDateTime> = match &rrule_raw {
                // Gjentakende: ekspander forekomster innen horisonten
                Some(rule) => expand_rrule(rule, first, horizon).unwrap_or_else(|e| {
                    warn!("rrule-ekspansjon feilet for «{title}»: {e:#}");
                    vec![first]
                }),
                None => vec![first],
            };

            for dt in starts {
                if dt.date() < today || dt.date() > horizon {
                    continue;
                }
                let iso = if all_day {
                    dt.date().to_string()
                } else {
                    dt.format("%Y-%m-%dT%H:%M").to_string()
                };
                events.push((
                    dt,
                    CalendarEvent { start: iso, title: title.clone(), all_day },
                ));
            }
        }
    }
    events
}

/// Ekspander en RRULE-streng til lokale starttidspunkter frem til horisonten.
fn expand_rrule(
    rule: &str,
    first: NaiveDateTime,
    horizon: NaiveDate,
) -> Result<Vec<NaiveDateTime>> {
    // rrule-craten vil ha DTSTART + RRULE som én blokk, med tz-annotert DTSTART
    let dtstart_utc = sast()
        .from_local_datetime(&first)
        .single()
        .context("tvetydig dtstart")?
        .with_timezone(&Utc);
    let set: RRuleSet = format!(
        "DTSTART:{}\nRRULE:{}",
        dtstart_utc.format("%Y%m%dT%H%M%SZ"),
        rule
    )
    .parse()
    .context("parse rrule")?;

    let until = sast()
        .from_local_datetime(&horizon.and_hms_opt(23, 59, 59).unwrap())
        .single()
        .context("tvetydig horisont")?
        .with_timezone(&RTz::UTC);

    Ok(set
        .before(until)
        .all(200)
        .dates
        .into_iter()
        .map(|d: DateTime<RTz>| d.with_timezone(&sast()).naive_local())
        .collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ukentlig_rrule_ekspanderes() {
        // En ukentlig avtale som startet for lenge siden skal gi forekomster
        // innenfor 30-dagershorisonten
        let start = Utc::now().with_timezone(&sast()) - Duration::days(60);
        let ics = format!(
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Ukesmøte\nDTSTART:{}\nRRULE:FREQ=WEEKLY\nEND:VEVENT\nEND:VCALENDAR\n",
            start.format("%Y%m%dT100000")
        );
        let events = parse_ics(&ics);
        assert!(
            events.len() >= 3,
            "forventet minst 3 ukentlige forekomster, fikk {}",
            events.len()
        );
        assert!(events.iter().all(|(_, e)| e.title == "Ukesmøte"));
    }

    #[test]
    fn engangsavtale_frem_i_tid() {
        let start = Utc::now().with_timezone(&sast()) + Duration::days(3);
        let ics = format!(
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Middag\nDTSTART:{}\nEND:VEVENT\nEND:VCALENDAR\n",
            start.format("%Y%m%dT190000")
        );
        let events = parse_ics(&ics);
        assert_eq!(events.len(), 1);
        assert!(!events[0].1.all_day);
    }
}
