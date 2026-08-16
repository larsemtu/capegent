// Live Radar og watchlists — manuelt kuratert innhold fra LAR-24.
// De store konsertene hentes automatisk fra whatsonincapetown; dette er
// tillegget: ekte live-musikk (kor, orkester, jazz) som aggregatorene ikke
// dekker. Oppdateres via Linear + «si til Claude».
const LIVE_RADAR = [
  { start: '2026-08-18', title: 'Stabat Mater – Karl Jenkins', venue: 'Cape Town City Hall', pri: 'Høy', note: 'Kor + Windworx + symfonisk blås' },
  { start: '2026-08-22', title: 'Mandisi Dyantyis – Symphonic Celebration', venue: 'Artscape Opera House', pri: 'MUST', note: 'Liveband + Cape Town Philharmonic' },
  { start: '2026-08-29', title: 'Wayne Bosch & Friends – Homecoming', venue: 'Artscape', pri: 'Høy', note: 'CT-gitarist, original jazz, liveband' },
  { start: '2026-08-30', title: 'BOO! + Nomadic Orchestra', venue: 'Brass Bell, Kalk Bay', pri: 'MUST', note: 'Live brass, Balkan, jazz, rock, hiphop' },
  { start: '2026-08-30', title: 'City Praise – Cape Town Gospel Choir', venue: 'Cape Town City Hall', pri: 'Høy', note: 'Kor + band + fullt symfoniorkester' },
  { start: '2026-09-05', title: 'Youth Classical Concert', venue: 'Artscape', pri: 'Medium', note: 'CT Philharmonic + unge klassiske' },
  { start: '2026-09-08', title: 'IMMERSION', venue: 'Cape Town City Hall', pri: 'Høy', note: '7 sound-artists, 150+ akustiske instrumenter' },
  { start: '2026-09-11', title: 'Cape Town Jazzathon – 30 år', venue: 'Artscape', pri: 'MUST', note: 'Multi-genre jazzfestival, flere datoer' },
  { start: '2026-09-17', title: 'Reuben T. Caluza – The B-Side', venue: 'Artscape', pri: 'Medium', note: 'Cine-concert med ensemble (17.–19.)' },
  { start: '2026-09-18', title: 'WOMAD – Korean Culture Concert', venue: 'Artscape', pri: 'Høy', note: 'Gugak + Samulnori + amapiano-elementer' },
  { start: '2026-09-19', title: 'WOMAD – New Generations + Peace Concert', venue: 'Artscape', pri: 'Høy', note: 'World music, to konserter samme dag' },
  { start: '2026-09-20', title: 'WOMAD – Buena Vista Social Club 30 Years', venue: 'Artscape', pri: 'MUST', note: 'Afro-cubansk live' },
  { start: '2026-10-03', title: 'Toyota Stellenbosch Woordfees', venue: 'Stellenbosch', pri: 'MUST', note: '444+ programpunkter (3.–11. okt): jazz, Vusi Mahlasela, Jeremy Loops, kor' },
  { start: '2026-10-06', title: 'Stellenbosch University Choir – 90 år', venue: 'Endler, Stellenbosch', pri: 'MUST', note: '6/7/11. okt, flere konserter' },
  { start: '2026-10-09', title: 'Cape Town Chamber Choir – Met woord en lied', venue: 'Fismer, Stellenbosch', pri: 'Høy', note: 'Afrikaans a cappella' },
  { start: '2026-10-18', title: 'Cape Town Camerata – A Cappella', venue: 'Endler, Stellenbosch', pri: 'Høy', note: 'Folk, afrikansk, renessanse, moderne' },
  { start: '2026-11-13', title: 'Swing With The King', venue: 'Artscape', pri: 'Høy', note: 'Gospel jazz + big band + Cape Ghoema (13.–14.)' },
  { start: '2026-11-24', title: 'From Hanover Street – 60 Years', venue: 'Artscape', pri: 'Medium', note: 'Heritage-konsert (24.–29.)' },
];

const CONCERT_NOTES = `
<h4>🇿🇦 Joburg — helgetur-kandidater</h4>
<p>12. des: J. Cole «The Fall Off» @ FNB Stadium (eneste Afrika-stopp) ·
13. des: Limp Bizkit @ FNB Stadium · 9. jan 2027: Tyla @ Expo Centre Nasrec</p>
<h4>📅 Årlige — sannsynlige, ikke annonsert</h4>
<p>Milk + Cookies (jan, Kenilworth Racecourse) · ULTRA South Africa (april,
The Ostrich) · Calabash (feb, DHL Stadium) · Cape Town Jazz Week (mars) ·
Jerk x Jollof NYE-takeover</p>
<h4>🎷 Cape Town Int. Jazz Festival — mars 2027</h4>
<p><strong>MUST.</strong> Lineup annonseres nov/des 2026 — månedslang feiring
med to flagship-helger. 2026 hadde Jacob Collier, Yussef Dayes, Abdullah
Ibrahim m.fl. Kjøp billetter tidlig ved store navn.</p>
<h4>👀 Watchlist</h4>
<p>Lizwi Choir (Makhaza/Khayelitsha) · Major Voices (isicathamiya-mannskor) ·
Cape Malay Choir Board (ghoema) — publiseres når datoer er verifisert.
Afrobeats-bookinger (Wizkid/Burna/Asake-segmentet) lander normalt i
nov–des-vinduet, annonseres 6–10 uker før via AFROTRAX/PLAYY, Jerk x Jollof,
In The City og Milk + Cookies.</p>`;
