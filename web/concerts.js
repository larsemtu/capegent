// Live Radar and watchlists — manually curated from LAR-24.
// The big concerts come automatically from whatsonincapetown; this is the
// supplement: real live music (choirs, orchestras, jazz) the aggregators
// miss. Updated via Linear + 'ask Claude'.
const LIVE_RADAR = [
  { start: '2026-08-18', title: 'Stabat Mater – Karl Jenkins', venue: 'Cape Town City Hall', pri: 'High', note: 'Choir + Windworx + symphonic winds' },
  { start: '2026-08-22', title: 'Mandisi Dyantyis – Symphonic Celebration', venue: 'Artscape Opera House', pri: 'MUST', note: 'Live band + Cape Town Philharmonic' },
  { start: '2026-08-29', title: 'Wayne Bosch & Friends – Homecoming', venue: 'Artscape', pri: 'High', note: 'CT guitarist, original jazz, live band' },
  { start: '2026-08-30', title: 'BOO! + Nomadic Orchestra', venue: 'Brass Bell, Kalk Bay', pri: 'MUST', note: 'Live brass, Balkan, jazz, rock, hip-hop' },
  { start: '2026-08-30', title: 'City Praise – Cape Town Gospel Choir', venue: 'Cape Town City Hall', pri: 'High', note: 'Choir + band + full symphony orchestra' },
  { start: '2026-09-05', title: 'Youth Classical Concert', venue: 'Artscape', pri: 'Medium', note: 'CT Philharmonic + young classical talent' },
  { start: '2026-09-08', title: 'IMMERSION', venue: 'Cape Town City Hall', pri: 'High', note: '7 sound artists, 150+ acoustic instruments' },
  { start: '2026-09-11', title: 'Cape Town Jazzathon – 30 years', venue: 'Artscape', pri: 'MUST', note: 'Multi-genre jazz festival, several dates' },
  { start: '2026-09-17', title: 'Reuben T. Caluza – The B-Side', venue: 'Artscape', pri: 'Medium', note: 'Cine-concert with ensemble (17–19th)' },
  { start: '2026-09-18', title: 'WOMAD – Korean Culture Concert', venue: 'Artscape', pri: 'High', note: 'Gugak + Samulnori + amapiano elements' },
  { start: '2026-09-19', title: 'WOMAD – New Generations + Peace Concert', venue: 'Artscape', pri: 'High', note: 'World music, two concerts same day' },
  { start: '2026-09-20', title: 'WOMAD – Buena Vista Social Club 30 Years', venue: 'Artscape', pri: 'MUST', note: 'Afro-Cuban live' },
  { start: '2026-10-03', title: 'Toyota Stellenbosch Woordfees', venue: 'Stellenbosch', pri: 'MUST', note: '444+ programme items (3–11 Oct): jazz, Vusi Mahlasela, Jeremy Loops, choirs' },
  { start: '2026-10-06', title: 'Stellenbosch University Choir – 90 years', venue: 'Endler, Stellenbosch', pri: 'MUST', note: '6/7/11 Oct, several concerts' },
  { start: '2026-10-09', title: 'Cape Town Chamber Choir – Met woord en lied', venue: 'Fismer, Stellenbosch', pri: 'High', note: 'Afrikaans a cappella' },
  { start: '2026-10-18', title: 'Cape Town Camerata – A Cappella', venue: 'Endler, Stellenbosch', pri: 'High', note: 'Folk, African, Renaissance, modern' },
  { start: '2026-11-13', title: 'Swing With The King', venue: 'Artscape', pri: 'High', note: 'Gospel jazz + big band + Cape Ghoema (13–14th)' },
  { start: '2026-11-24', title: 'From Hanover Street – 60 Years', venue: 'Artscape', pri: 'Medium', note: 'Heritage concert (24–29th)' },
];

const CONCERT_NOTES = `
<h4>🇿🇦 Joburg — weekend-trip candidates</h4>
<p>12 Dec: J. Cole "The Fall Off" @ FNB Stadium (only Africa stop) ·
13 Dec: Limp Bizkit @ FNB Stadium · 9 Jan 2027: Tyla @ Expo Centre Nasrec</p>
<h4>📅 Annual — likely, not yet announced</h4>
<p>Milk + Cookies (Jan, Kenilworth Racecourse) · ULTRA South Africa (April,
The Ostrich) · Calabash (Feb, DHL Stadium) · Cape Town Jazz Week (March) ·
Jerk x Jollof NYE takeover</p>
<h4>🎷 Cape Town Int. Jazz Festival — March 2027</h4>
<p><strong>MUST.</strong> Lineup announced Nov/Dec 2026 — month-long
celebration with two flagship weekends. 2026 had Jacob Collier, Yussef Dayes,
Abdullah Ibrahim and more. Buy tickets early if big names land.</p>
<h4>👀 Watchlist</h4>
<p>Lizwi Choir (Makhaza/Khayelitsha) · Major Voices (isicathamiya male choir) ·
Cape Malay Choir Board (ghoema) — published once dates are verified.
Afrobeats bookings (the Wizkid/Burna/Asake segment) usually land in the
Nov–Dec window, announced 6–10 weeks ahead via AFROTRAX/PLAYY, Jerk x Jollof,
In The City and Milk + Cookies.</p>`;
